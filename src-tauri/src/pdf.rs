//! Local PDF extraction with stable physical page numbers.
use crate::store::Result;
use serde::{Deserialize, Serialize};
use std::{
    ffi::OsStr,
    io::Read,
    path::{Path, PathBuf},
    time::Duration,
};

pub const MAX_PDF_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_PDF_PAGES: usize = 200;
pub const MAX_PDF_TEXT_BYTES: usize = 2 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PdfPage {
    pub number: u32,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct PreparedPdf {
    pub source: Vec<u8>,
    pub pages: Vec<PdfPage>,
}

const WORKER_FLAG: &str = "--cachella-extract-pdf";
const STREAM_LIMIT: usize = 4 * 1024 * 1024;
const RESULT_LIMIT: usize = MAX_PDF_TEXT_BYTES * 6 + 64 * 1024;

/// Parse in a separate guarded process. The input snapshot also supplies the
/// bytes persisted by the store, so later changes to the selected file cannot
/// make the extracted pages disagree with the stored original.
pub async fn prepare(path: &Path) -> Result<PreparedPdf> {
    let source = read_source(path)?;
    let temporary = crate::scratch::Workspace::new()?;
    let input = temporary.path().join("input.pdf");
    let result_path = temporary.path().join("pages.json");
    std::fs::write(&input, &source).map_err(|_| "Could not snapshot PDF input.")?;
    let output = crate::media::run_command(
        &worker_executable()?,
        vec![
            WORKER_FLAG.into(),
            input.into_os_string(),
            result_path.as_os_str().to_owned(),
        ],
        Duration::from_secs(30),
    )
    .await
    .map_err(|_| "PDF extraction stopped or exceeded its resource limits.".to_string())?;
    if !output.success {
        return Err("PDF extraction failed. Try an unencrypted PDF with selectable text.".into());
    }
    let bytes = read_bounded(&result_path, RESULT_LIMIT)?;
    let pages: Result<Vec<PdfPage>> = serde_json::from_slice(&bytes)
        .map_err(|_| "PDF extraction returned an invalid result.".to_string())?;
    Ok(PreparedPdf {
        source,
        pages: pages?,
    })
}

/// Called before creating a GUI or async runtime in the helper executable.
pub fn entry() -> Option<i32> {
    let mut args = std::env::args_os().skip(1);
    if args.next().as_deref() != Some(OsStr::new(WORKER_FLAG)) {
        return None;
    }
    Some(
        (|| -> Result<()> {
            let input = args.next().ok_or("Missing PDF input.")?;
            let output = args.next().ok_or("Missing PDF output.")?;
            if args.next().is_some() {
                return Err("Unexpected PDF arguments.".into());
            }
            let result = read_source(Path::new(&input)).and_then(|bytes| extract(&bytes));
            let encoded = serde_json::to_vec(&result).map_err(|_| "Could not encode PDF pages.")?;
            if encoded.len() > RESULT_LIMIT {
                return Err("PDF result exceeds its limit.".into());
            }
            std::fs::write(Path::new(&output), encoded)
                .map_err(|_| "Could not save extracted PDF pages.".into())
        })()
        .map(|_| 0)
        .unwrap_or(1),
    )
}

fn worker_executable() -> Result<PathBuf> {
    let current = std::env::current_exe().map_err(|_| "Cannot locate PDF helper.")?;
    #[cfg(test)]
    {
        let profile = current
            .parent()
            .and_then(Path::parent)
            .ok_or("Missing test profile.")?;
        let fixture = profile.join("examples").join(format!(
            "process_guard_fixture{}",
            std::env::consts::EXE_SUFFIX
        ));
        if !fixture.is_file() {
            return Err("Build process_guard_fixture before running PDF tests.".into());
        }
        Ok(fixture)
    }
    #[cfg(not(test))]
    {
        Ok(current)
    }
}

fn read_bounded(path: &Path, limit: usize) -> Result<Vec<u8>> {
    let file = crate::regular_file::open_regular_file(path).map_err(|_| "Cannot open PDF data.")?;
    let metadata = file.metadata().map_err(|_| "Cannot inspect PDF data.")?;
    if metadata.len() > limit as u64 {
        return Err("PDF data exceeds the supported file size.".into());
    }
    let mut bytes = Vec::new();
    file.take(limit as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "Cannot read PDF data.")?;
    if bytes.len() > limit {
        return Err("PDF data grew beyond its size limit.".into());
    }
    Ok(bytes)
}

fn read_source(path: &Path) -> Result<Vec<u8>> {
    let bytes = read_bounded(path, MAX_PDF_BYTES)?;
    validate_source(&bytes)?;
    Ok(bytes)
}

fn validate_source(bytes: &[u8]) -> Result<()> {
    if bytes.len() > MAX_PDF_BYTES || !bytes.starts_with(b"%PDF-") {
        return Err("Choose a PDF file of at most 16 MB.".into());
    }
    Ok(())
}

fn extract(bytes: &[u8]) -> Result<Vec<PdfPage>> {
    validate_source(bytes)?;
    let document = lopdf::Document::load_mem_with_options(
        bytes,
        lopdf::LoadOptions {
            strict: true,
            max_decompressed_size: Some(STREAM_LIMIT),
            ..Default::default()
        },
    )
    .map_err(|_| {
        "Cannot read this PDF. It may be damaged, encrypted or exceed the supported limits."
    })?;
    // Password entry and permission-protected PDFs are outside this import path.
    if document.is_encrypted()
        || document.encryption_state.is_some()
        || document.trailer.has(b"Encrypt")
    {
        return Err(
            "Import an unencrypted PDF; password-protected files are not supported.".into(),
        );
    }
    let physical_pages = document.get_pages();
    if physical_pages.is_empty() || physical_pages.len() > MAX_PDF_PAGES {
        return Err("Import a PDF with 1 to 200 pages.".into());
    }
    let mut pages = Vec::with_capacity(physical_pages.len());
    let mut text_bytes = 0;
    for number in physical_pages.keys() {
        let text = document
            .extract_text_with_limit(&[*number], STREAM_LIMIT)
            .map_err(|_| "A PDF page could not be extracted within the supported limits.")?;
        let text = text.trim().to_owned();
        text_bytes += text.len();
        if text_bytes > MAX_PDF_TEXT_BYTES || text.contains('\0') {
            return Err("PDF text is invalid or exceeds the 2 MB extracted-text limit.".into());
        }
        pages.push(PdfPage {
            number: *number,
            text,
        });
    }
    if pages.iter().all(|page| page.text.is_empty()) {
        return Err("This PDF has no extractable text. Scanned pages require OCR, which is not available yet.".into());
    }
    Ok(pages)
}

#[cfg(test)]
mod tests {
    use super::*;
    const FIXTURE: &[u8] = include_bytes!("../../evaluation/pdf/fixtures/project-brief.pdf");

    #[test]
    fn extracts_physical_pages_without_renumbering_blank_pages() {
        let pages = extract(FIXTURE).unwrap();
        assert_eq!(
            pages.iter().map(|p| p.number).collect::<Vec<_>>(),
            vec![1, 2, 3]
        );
        assert!(pages[0].text.contains("Friday"));
        assert!(pages[1].text.trim().is_empty());
        assert!(pages[2].text.contains("nine dollars"));
    }

    #[test]
    fn rejects_invalid_and_oversized_sources() {
        assert!(extract(b"not a PDF").is_err());
        assert!(extract(b"%PDF-1.7\nmalformed").is_err());
        let mut huge = vec![b' '; MAX_PDF_BYTES + 1];
        huge[..5].copy_from_slice(b"%PDF-");
        assert!(extract(&huge).is_err());
    }

    fn save(mut document: lopdf::Document) -> Vec<u8> {
        let mut bytes = Vec::new();
        document.save_to(&mut bytes).unwrap();
        bytes
    }

    #[test]
    fn rejects_documents_without_selectable_text() {
        let mut document = lopdf::Document::load_mem(FIXTURE).unwrap();
        for id in document.get_pages().values() {
            document
                .get_object_mut(*id)
                .unwrap()
                .as_dict_mut()
                .unwrap()
                .remove(b"Contents");
        }
        assert!(extract(&save(document)).unwrap_err().contains("OCR"));
    }

    #[test]
    fn rejects_page_stream_expansion_beyond_the_limit() {
        let mut document = lopdf::Document::load_mem(FIXTURE).unwrap();
        let mut stream = lopdf::Stream::new(lopdf::Dictionary::new(), vec![b' '; STREAM_LIMIT + 1]);
        stream.compress().unwrap();
        let stream_id = document.add_object(stream);
        let page_id = document.get_pages()[&3];
        document
            .get_object_mut(page_id)
            .unwrap()
            .as_dict_mut()
            .unwrap()
            .set("Contents", stream_id);
        let bytes = save(document);
        assert!(bytes.len() < MAX_PDF_BYTES);
        assert!(extract(&bytes).is_err());
    }

    #[test]
    fn rejects_permission_protected_and_password_protected_pdfs() {
        for password in ["", "fixture-password"] {
            let mut document = lopdf::Document::load_mem(FIXTURE).unwrap();
            let state = lopdf::EncryptionState::try_from(lopdf::EncryptionVersion::V1 {
                document: &document,
                owner_password: "fixture-owner",
                user_password: password,
                permissions: lopdf::Permissions::empty(),
            })
            .unwrap();
            document.encrypt(&state).unwrap();
            assert!(extract(&save(document)).is_err());
        }
    }

    fn decode_fixture_cmap(last: &str, target: &str) -> lopdf::Result<String> {
        let mut document = lopdf::Document::new();
        let cmap = format!("/CIDInit /ProcSet findresource begin\n12 dict begin\nbegincmap\n/CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def\n/CMapName /Test def\n/CMapType 2 def\n1 begincodespacerange\n<00000000> <{last}>\nendcodespacerange\n1 beginbfrange\n<00000000> <{last}> <{target}>\nendbfrange\nendcmap\nCMapName currentdict /CMap defineresource pop\nend\nend");
        let id = document.add_object(lopdf::Stream::new(
            lopdf::Dictionary::new(),
            cmap.into_bytes(),
        ));
        let mut font = lopdf::Dictionary::new();
        font.set("Type", "Font");
        font.set("Subtype", "Type0");
        font.set("Encoding", "Identity-H");
        font.set("ToUnicode", id);
        let encoding = font.get_font_encoding_with_limit(&document, STREAM_LIMIT)?;
        lopdf::Document::decode_text(&encoding, &[0, 0, 0, 0])
    }

    #[test]
    fn rejects_excessive_font_mapping_cardinality_before_expansion() {
        assert!(decode_fixture_cmap("00010000", "0041").is_err());
    }

    #[test]
    fn bounds_font_mapping_payload_and_keeps_normal_unicode_mapping() {
        assert_eq!(decode_fixture_cmap("00000001", "0041").unwrap(), "A");
        assert!(decode_fixture_cmap("00001000", &"0041".repeat(256)).is_err());
    }

    #[tokio::test]
    async fn guarded_helper_prepares_the_exact_input_snapshot() {
        let prepared = prepare(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../evaluation/pdf/fixtures/project-brief.pdf")
                .as_path(),
        )
        .await
        .unwrap();
        assert_eq!(prepared.source, FIXTURE);
        assert_eq!(prepared.pages.len(), 3);
        assert!(prepared.pages[1].text.is_empty());
        assert!(prepared.pages[2].text.contains("nine dollars"));
    }
}
