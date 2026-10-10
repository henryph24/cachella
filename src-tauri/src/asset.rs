//! Range parsing for local media previews. Never resolves filesystem paths.
use crate::store::{Result, Store};

#[derive(Debug, PartialEq)]
pub struct ByteRange {
    pub start: usize,
    pub end: usize,
}

pub fn parse_range(value: &str, size: usize) -> Result<ByteRange> {
    if size == 0 {
        return Err("Empty media has no ranges.".into());
    }
    let value = value
        .strip_prefix("bytes=")
        .ok_or("Unsupported range unit.")?;
    let (first, last) = value.split_once('-').ok_or("Invalid byte range.")?;
    let number = |value: &str| -> Result<usize> {
        if value.is_empty() || !value.bytes().all(|b| b.is_ascii_digit()) {
            return Err("Invalid byte offset.".into());
        }
        value
            .parse()
            .map_err(|_| "Byte offset exceeds supported size.".into())
    };
    let (start, end) = if first.is_empty() {
        let suffix = number(last)?;
        if suffix == 0 {
            return Err("Empty suffix range.".into());
        }
        (size.saturating_sub(suffix), size - 1)
    } else {
        let start = number(first)?;
        let end = if last.is_empty() {
            size - 1
        } else {
            number(last)?.min(size - 1)
        };
        if start >= size || end < start {
            return Err("Unsatisfiable byte range.".into());
        }
        (start, end)
    };
    Ok(ByteRange {
        start,
        end: end.min(start.saturating_add(2 * 1024 * 1024 - 1)),
    })
}

pub struct AssetResponse {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

pub fn serve(store: &Store, path: &str, range: Option<&str>, method: &str) -> AssetResponse {
    let mut response = AssetResponse {
        status: 404,
        headers: vec![
            ("Cache-Control".into(), "no-store".into()),
            ("X-Content-Type-Options".into(), "nosniff".into()),
        ],
        body: vec![],
    };
    if !matches!(method, "GET" | "HEAD") {
        response.status = 405;
        return response;
    }
    let id = path.strip_prefix('/').unwrap_or(path);
    if id.len() != 64 || !id.bytes().all(|b| b.is_ascii_hexdigit()) {
        return response;
    }
    let Ok((mime, size)) = store.asset_info(id) else {
        return response;
    };
    if size == 0
        || size > 32 * 1024 * 1024
        || !matches!(mime.as_str(), "image/png" | "audio/wav" | "video/mp4")
    {
        return response;
    }
    let selected = if let Some(range) = range {
        match parse_range(range, size) {
            Ok(range) => {
                response.status = 206;
                response.headers.push((
                    "Content-Range".into(),
                    format!("bytes {}-{}/{size}", range.start, range.end),
                ));
                range
            }
            Err(_) => {
                response.status = 416;
                response
                    .headers
                    .push(("Content-Range".into(), format!("bytes */{size}")));
                return response;
            }
        }
    } else {
        response.status = 200;
        ByteRange {
            start: 0,
            end: size - 1,
        }
    };
    let length = selected.end - selected.start + 1;
    response.headers.extend([
        ("Content-Type".into(), mime),
        ("Content-Length".into(), length.to_string()),
        ("Accept-Ranges".into(), "bytes".into()),
    ]);
    if method == "GET" {
        match store.asset_range(id, selected.start, length) {
            Ok(bytes) if bytes.len() == length => response.body = bytes,
            _ => {
                response.status = 404;
                response
                    .headers
                    .retain(|(key, _)| key != "Content-Length" && key != "Content-Range");
            }
        }
    }
    response
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn serves_only_stored_media_and_honors_get_head_and_range_failures() {
        let mut db = Store::open(std::path::Path::new(":memory:")).unwrap();
        let data = b"\x89PNG\r\n\x1a\nfixture-media".to_vec();
        let media = crate::media::PreparedMedia {
            kind: "image".into(),
            mime: "image/png".into(),
            source: data.clone(),
            fingerprint: "ab".repeat(32),
            parts: vec![crate::media::MediaPart {
                mime: "image/png".into(),
                data: data.clone(),
                start_ms: None,
                end_ms: None,
                label: "Image".into(),
            }],
        };
        let doc = db.import_media("Image.png", &media).unwrap();
        let path = format!("/{}", doc.id);
        let full = serve(&db, &path, None, "GET");
        assert_eq!(full.status, 200);
        assert_eq!(full.body, data);
        assert!(full
            .headers
            .contains(&("X-Content-Type-Options".into(), "nosniff".into())));
        let partial = serve(&db, &path, Some("bytes=0-7"), "GET");
        assert_eq!(partial.status, 206);
        assert_eq!(partial.body, data[..8]);
        assert!(partial
            .headers
            .contains(&("Content-Length".into(), "8".into())));
        assert!(serve(&db, &path, None, "HEAD").body.is_empty());
        assert_eq!(serve(&db, &path, Some("bytes=9999-"), "GET").status, 416);
        assert_eq!(serve(&db, &path, None, "POST").status, 405);
        for unsafe_path in ["/../../secret", "/%2e%2e/secret", "/abc", "/ab/ab"] {
            assert_eq!(serve(&db, unsafe_path, None, "GET").status, 404);
        }
        db.delete(&doc.id).unwrap();
        assert_eq!(serve(&db, &path, None, "GET").status, 404);
    }
    #[test]
    fn ranges_support_bounded_open_and_suffix_requests() {
        assert_eq!(
            parse_range("bytes=0-9", 100).unwrap(),
            ByteRange { start: 0, end: 9 }
        );
        assert_eq!(
            parse_range("bytes=90-", 100).unwrap(),
            ByteRange { start: 90, end: 99 }
        );
        assert_eq!(
            parse_range("bytes=-10", 100).unwrap(),
            ByteRange { start: 90, end: 99 }
        );
        assert_eq!(
            parse_range("bytes=95-999", 100).unwrap(),
            ByteRange { start: 95, end: 99 }
        );
        assert_eq!(
            parse_range("bytes=-999", 100).unwrap(),
            ByteRange { start: 0, end: 99 }
        );
        assert!(parse_range("bytes=0-", 32 * 1024 * 1024).unwrap().end < 2 * 1024 * 1024);
    }
    #[test]
    fn rejects_empty_out_of_bounds_multiple_and_malformed_ranges() {
        for range in [
            "bytes=100-",
            "bytes=10-9",
            "bytes=-0",
            "bytes=0-1,4-5",
            "bytes=-",
            "items=0-1",
            "bytes=+1-2",
            "bytes=184467440737095516160-",
            "bytes=0-NaN",
        ] {
            assert!(parse_range(range, 100).is_err(), "{range}");
        }
        assert!(parse_range("bytes=0-", 0).is_err());
    }
}
