use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::Path;

pub type Result<T> = std::result::Result<T, String>;
pub const MAX_DOCUMENT_BYTES: usize = 2 * 1024 * 1024;
pub const MAX_MEDIA_SOURCE_BYTES: usize = 32 * 1024 * 1024;
const MAX_MEDIA_PREPARED_BYTES: usize = 64 * 1024 * 1024;
const MAX_MEDIA_LIBRARY_BYTES: usize = 512 * 1024 * 1024;
const MAX_MEDIA_PARTS: usize = 36;
const MAX_MEDIA_PART_BYTES: usize = 2 * 1024 * 1024;
const MAX_MEDIA_DURATION_MS: u64 = 120_000;
const EMBEDDING_DIMENSIONS: usize = 768;
const MAX_EMBEDDING_PROFILE_CHARS: usize = 128;
const MAX_PENDING_EMBEDDINGS: usize = 32;
const MAX_HIT_TEXT_BYTES: usize = 8192;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub id: String,
    pub title: String,
    pub kind: String,
    pub bytes: usize,
    pub chunks: usize,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hit {
    pub id: String,
    pub document_id: String,
    pub title: String,
    pub ordinal: usize,
    pub text: String,
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub start_ms: Option<u64>,
    #[serde(default)]
    pub end_ms: Option<u64>,
    #[serde(default)]
    pub page_number: Option<u32>,
}

pub struct Store {
    conn: Connection,
}

impl Store {
    pub fn open(path: &Path) -> Result<Self> {
        let conn = Connection::open(path).map_err(|e| e.to_string())?;
        conn.busy_timeout(std::time::Duration::from_secs(5))
            .map_err(|e| e.to_string())?;
        conn.execute_batch("PRAGMA foreign_keys=ON;
            PRAGMA journal_mode=WAL;
            CREATE TABLE IF NOT EXISTS documents (
                id TEXT PRIMARY KEY, title TEXT NOT NULL, kind TEXT NOT NULL,
                content TEXT NOT NULL, bytes INTEGER NOT NULL,
                created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now'))
            );
            CREATE TABLE IF NOT EXISTS chunks (
                id TEXT PRIMARY KEY, document_id TEXT NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
                ordinal INTEGER NOT NULL, text TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS embeddings (
                chunk_id TEXT PRIMARY KEY REFERENCES chunks(id) ON DELETE CASCADE,
                profile TEXT NOT NULL,
                vector BLOB NOT NULL
            ) WITHOUT ROWID;
            CREATE VIRTUAL TABLE IF NOT EXISTS chunks_fts USING fts5(chunk_id UNINDEXED, document_id UNINDEXED, title, text);
            CREATE TABLE IF NOT EXISTS assets (
                document_id TEXT PRIMARY KEY REFERENCES documents(id) ON DELETE CASCADE,
                mime TEXT NOT NULL,
                payload BLOB NOT NULL
            );
            CREATE TABLE IF NOT EXISTS media_parts (
                chunk_id TEXT PRIMARY KEY REFERENCES chunks(id) ON DELETE CASCADE,
                mime TEXT NOT NULL,
                payload BLOB NOT NULL,
                start_ms INTEGER,
                end_ms INTEGER,
                label TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS pdf_pages (
                document_id TEXT NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
                page_number INTEGER NOT NULL,
                text TEXT NOT NULL,
                PRIMARY KEY(document_id, page_number)
            ) WITHOUT ROWID;
            CREATE TABLE IF NOT EXISTS pdf_chunk_pages (
                chunk_id TEXT PRIMARY KEY REFERENCES chunks(id) ON DELETE CASCADE,
                page_number INTEGER NOT NULL
            ) WITHOUT ROWID;
        ").map_err(|e| e.to_string())?;
        let mut conn = conn;
        migrate_legacy_embeddings_schema(&mut conn)?;
        Ok(Self { conn })
    }

    pub fn import_text(&mut self, title: &str, kind: &str, text: &str) -> Result<Document> {
        let title = title.trim();
        if title.is_empty() || title.chars().count() > 240 {
            return Err("Document title must contain 1 to 240 characters.".into());
        }
        if text.len() > MAX_DOCUMENT_BYTES || text.trim().is_empty() || text.contains('\0') {
            return Err("Import a nonempty UTF-8 text document of at most 2 MB.".into());
        }
        if !["txt", "md"].contains(&kind) {
            return Err("This build currently imports .txt and .md files.".into());
        }
        let id = format!("{:x}", Sha256::digest(text.as_bytes()));
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e| e.to_string())?;
        let existing = tx.query_row("SELECT d.id,d.title,d.kind,d.bytes,(SELECT count(*) FROM chunks WHERE document_id=d.id),d.created_at FROM documents d WHERE d.id=?1", [&id], |r|Ok(Document{id:r.get(0)?,title:r.get(1)?,kind:r.get(2)?,bytes:r.get(3)?,chunks:r.get(4)?,created_at:r.get(5)?})).optional().map_err(|e|e.to_string())?;
        if let Some(existing) = existing {
            return Ok(existing);
        }
        let count: usize = tx
            .query_row("SELECT count(*) FROM documents", [], |r| r.get(0))
            .map_err(|e| e.to_string())?;
        if count >= 500 {
            return Err("The current library limit is 500 documents.".into());
        }
        let parts = chunks(text, 240, 30);
        let total: usize = tx
            .query_row("SELECT count(*) FROM chunks", [], |r| r.get(0))
            .map_err(|e| e.to_string())?;
        if total + parts.len() > 10_000 {
            return Err("The current index limit is 10,000 chunks.".into());
        }
        tx.execute(
            "INSERT INTO documents(id,title,kind,content,bytes) VALUES(?1,?2,?3,?4,?5)",
            params![id, title, kind, text, text.len()],
        )
        .map_err(|e| e.to_string())?;
        for (ordinal, part) in parts.iter().enumerate() {
            let chunk_id = format!("{id}:{ordinal}");
            tx.execute(
                "INSERT INTO chunks(id,document_id,ordinal,text) VALUES(?1,?2,?3,?4)",
                params![chunk_id, id, ordinal, part],
            )
            .map_err(|e| e.to_string())?;
            tx.execute(
                "INSERT INTO chunks_fts(chunk_id,document_id,title,text) VALUES(?1,?2,?3,?4)",
                params![chunk_id, id, title, part],
            )
            .map_err(|e| e.to_string())?;
        }
        tx.commit().map_err(|e| e.to_string())?;
        self.documents()?
            .into_iter()
            .find(|d| d.id == id)
            .ok_or_else(|| "Import did not produce a document.".into())
    }

    pub fn import_media(
        &mut self,
        title: &str,
        media: &crate::media::PreparedMedia,
    ) -> Result<Document> {
        let title = validate_title(title)?;
        validate_media(media)?;
        let id = media.fingerprint.clone();
        let preview = media_preview(title, media);
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e| e.to_string())?;
        if let Some(existing) = document_by_id(&tx, &id)? {
            return Ok(existing);
        }
        let document_count: usize = tx
            .query_row("SELECT count(*) FROM documents", [], |r| r.get(0))
            .map_err(|e| e.to_string())?;
        if document_count >= 500 {
            return Err("The current library limit is 500 documents.".into());
        }
        let chunk_count: usize = tx
            .query_row("SELECT count(*) FROM chunks", [], |r| r.get(0))
            .map_err(|e| e.to_string())?;
        if chunk_count + media.parts.len() > 10_000 {
            return Err("The current index limit is 10,000 chunks.".into());
        }
        let existing_media_bytes: usize = tx
            .query_row(
                "SELECT COALESCE((SELECT sum(length(payload)) FROM assets), 0) + COALESCE((SELECT sum(length(payload)) FROM media_parts), 0)",
                [],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        let incoming_bytes = media
            .source
            .len()
            .checked_add(
                media
                    .parts
                    .iter()
                    .map(|part| part.data.len())
                    .sum::<usize>(),
            )
            .ok_or_else(|| "Prepared media is too large.".to_string())?;
        if existing_media_bytes
            .checked_add(incoming_bytes)
            .ok_or_else(|| "The media library is too large.".to_string())?
            > MAX_MEDIA_LIBRARY_BYTES
        {
            return Err("The current media library limit is 512 MB.".into());
        }
        tx.execute(
            "INSERT INTO documents(id,title,kind,content,bytes) VALUES(?1,?2,?3,?4,?5)",
            params![id, title, media.kind, preview, media.source.len()],
        )
        .map_err(|e| e.to_string())?;
        tx.execute(
            "INSERT INTO assets(document_id,mime,payload) VALUES(?1,?2,?3)",
            params![media.fingerprint, media.mime, media.source],
        )
        .map_err(|e| e.to_string())?;
        for (ordinal, part) in media.parts.iter().enumerate() {
            let chunk_id = format!("{}:{ordinal}", media.fingerprint);
            let text = media_part_text(title, part);
            tx.execute(
                "INSERT INTO chunks(id,document_id,ordinal,text) VALUES(?1,?2,?3,?4)",
                params![chunk_id, media.fingerprint, ordinal, text],
            )
            .map_err(|e| e.to_string())?;
            tx.execute(
                "INSERT INTO media_parts(chunk_id,mime,payload,start_ms,end_ms,label) VALUES(?1,?2,?3,?4,?5,?6)",
                params![chunk_id, part.mime, part.data, part.start_ms, part.end_ms, part.label],
            )
            .map_err(|e| e.to_string())?;
            tx.execute(
                "INSERT INTO chunks_fts(chunk_id,document_id,title,text) VALUES(?1,?2,?3,?4)",
                params![chunk_id, media.fingerprint, title, text],
            )
            .map_err(|e| e.to_string())?;
        }
        tx.commit().map_err(|e| e.to_string())?;
        self.documents()?
            .into_iter()
            .find(|document| document.id == media.fingerprint)
            .ok_or_else(|| "Import did not produce a document.".into())
    }

    pub fn import_pdf(&mut self, title: &str, pdf: &crate::pdf::PreparedPdf) -> Result<Document> {
        let title = validate_title(title)?;
        validate_pdf(pdf)?;
        let id = pdf_source_id(&pdf.source);
        let content = pdf
            .pages
            .iter()
            .map(|page| page.text.as_str())
            .collect::<Vec<_>>()
            .join("\n\n");
        let page_chunks = pdf
            .pages
            .iter()
            .map(|page| chunks(&page.text, 240, 30))
            .collect::<Vec<_>>();
        let chunk_total = page_chunks.iter().map(Vec::len).sum::<usize>();
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e| e.to_string())?;
        if let Some(existing) = document_by_id(&tx, &id)? {
            return Ok(existing);
        }
        let document_count: usize = tx
            .query_row("SELECT count(*) FROM documents", [], |r| r.get(0))
            .map_err(|e| e.to_string())?;
        if document_count >= 500 {
            return Err("The current library limit is 500 documents.".into());
        }
        let existing_chunks: usize = tx
            .query_row("SELECT count(*) FROM chunks", [], |r| r.get(0))
            .map_err(|e| e.to_string())?;
        if existing_chunks + chunk_total > 10_000 {
            return Err("The current index limit is 10,000 chunks.".into());
        }
        let existing_asset_bytes: usize = tx
            .query_row(
                "SELECT COALESCE((SELECT sum(length(payload)) FROM assets), 0) + COALESCE((SELECT sum(length(payload)) FROM media_parts), 0)",
                [],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        if existing_asset_bytes
            .checked_add(pdf.source.len())
            .ok_or_else(|| "The media library is too large.".to_string())?
            > MAX_MEDIA_LIBRARY_BYTES
        {
            return Err("The current media library limit is 512 MB.".into());
        }
        tx.execute(
            "INSERT INTO documents(id,title,kind,content,bytes) VALUES(?1,?2,'pdf',?3,?4)",
            params![id, title, content, pdf.source.len()],
        )
        .map_err(|e| e.to_string())?;
        tx.execute(
            "INSERT INTO assets(document_id,mime,payload) VALUES(?1,'application/pdf',?2)",
            params![id, pdf.source],
        )
        .map_err(|e| e.to_string())?;
        let mut ordinal = 0usize;
        for (page, parts) in pdf.pages.iter().zip(page_chunks) {
            tx.execute(
                "INSERT INTO pdf_pages(document_id,page_number,text) VALUES(?1,?2,?3)",
                params![id, page.number, page.text],
            )
            .map_err(|e| e.to_string())?;
            for part in parts {
                let chunk_id = format!("{id}:{ordinal}");
                tx.execute(
                    "INSERT INTO chunks(id,document_id,ordinal,text) VALUES(?1,?2,?3,?4)",
                    params![chunk_id, id, ordinal, part],
                )
                .map_err(|e| e.to_string())?;
                tx.execute(
                    "INSERT INTO pdf_chunk_pages(chunk_id,page_number) VALUES(?1,?2)",
                    params![chunk_id, page.number],
                )
                .map_err(|e| e.to_string())?;
                tx.execute(
                    "INSERT INTO chunks_fts(chunk_id,document_id,title,text) VALUES(?1,?2,?3,?4)",
                    params![chunk_id, id, title, part],
                )
                .map_err(|e| e.to_string())?;
                ordinal += 1;
            }
        }
        tx.commit().map_err(|e| e.to_string())?;
        self.documents()?
            .into_iter()
            .find(|document| document.id == id)
            .ok_or_else(|| "Import did not produce a document.".into())
    }

    pub fn pdf_pages(&self, id: &str) -> Result<Vec<crate::pdf::PdfPage>> {
        let kind: Option<String> = self
            .conn
            .query_row("SELECT kind FROM documents WHERE id=?1", [id], |row| {
                row.get(0)
            })
            .optional()
            .map_err(|e| e.to_string())?;
        match kind.as_deref() {
            Some("pdf") => {}
            Some(_) => return Err("Document is not a PDF.".into()),
            None => return Err("PDF document is no longer in the library.".into()),
        }
        let mut statement = self
            .conn
            .prepare(
                "SELECT page_number,text FROM pdf_pages WHERE document_id=?1 ORDER BY page_number",
            )
            .map_err(|e| e.to_string())?;
        let rows = statement
            .query_map([id], |row| {
                Ok(crate::pdf::PdfPage {
                    number: row.get(0)?,
                    text: row.get(1)?,
                })
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())
    }

    pub fn documents(&self) -> Result<Vec<Document>> {
        let mut stmt = self.conn.prepare("SELECT d.id,d.title,d.kind,d.bytes,count(c.id),d.created_at FROM documents d LEFT JOIN chunks c ON c.document_id=d.id GROUP BY d.id ORDER BY d.created_at DESC,d.title").map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| {
                Ok(Document {
                    id: r.get(0)?,
                    title: r.get(1)?,
                    kind: r.get(2)?,
                    bytes: r.get(3)?,
                    chunks: r.get(4)?,
                    created_at: r.get(5)?,
                })
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())
    }

    pub fn search(&self, query: &str) -> Result<Vec<Hit>> {
        if query.chars().count() > 2000 {
            return Err("Search queries are limited to 2,000 characters.".into());
        }
        let words: Vec<_> = query
            .split(|c: char| !c.is_alphanumeric())
            .filter(|s| !s.is_empty())
            .take(32)
            .map(|s| format!("\"{s}\""))
            .collect();
        if words.is_empty() {
            return Ok(Vec::new());
        }
        let expression = words.join(" OR ");
        let mut stmt = self.conn.prepare("SELECT c.id,c.document_id,d.title,c.ordinal,c.text,d.kind,p.start_ms,p.end_ms,pp.page_number FROM chunks_fts f JOIN chunks c ON c.id=f.chunk_id JOIN documents d ON d.id=c.document_id LEFT JOIN media_parts p ON p.chunk_id=c.id LEFT JOIN pdf_chunk_pages pp ON pp.chunk_id=c.id WHERE chunks_fts MATCH ?1 ORDER BY bm25(chunks_fts,0,0,3,1) LIMIT 8").map_err(|e|e.to_string())?;
        let rows = stmt
            .query_map([expression], hit_from_row)
            .map_err(|e| e.to_string())?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())
    }

    pub fn pending_embeddings(&self, profile: &str, limit: usize) -> Result<Vec<Hit>> {
        validate_profile(profile)?;
        let limit = limit.min(MAX_PENDING_EMBEDDINGS);
        if limit == 0 {
            return Ok(Vec::new());
        }
        let mut stmt = self
            .conn
            .prepare(
                "SELECT c.id,c.document_id,d.title,c.ordinal,c.text,d.kind,p.start_ms,p.end_ms,pp.page_number
             FROM chunks c JOIN documents d ON d.id=c.document_id
             LEFT JOIN media_parts p ON p.chunk_id=c.id
             LEFT JOIN pdf_chunk_pages pp ON pp.chunk_id=c.id
             LEFT JOIN embeddings e ON e.chunk_id=c.id AND e.profile=?1
             WHERE e.chunk_id IS NULL
             ORDER BY c.id ASC LIMIT ?2",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![profile, limit], hit_from_row)
            .map_err(|e| e.to_string())?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())
    }

    pub fn save_embedding(&self, chunk_id: &str, profile: &str, vector: &[f32]) -> Result<()> {
        validate_profile(profile)?;
        validate_embedding(vector)?;
        let encoded = encode_embedding(vector);
        let changed = self
            .conn
            .execute(
                "INSERT INTO embeddings(chunk_id,profile,vector) VALUES(?1,?2,?3)
             ON CONFLICT(chunk_id) DO UPDATE SET profile=excluded.profile,vector=excluded.vector",
                params![chunk_id, profile, encoded],
            )
            .map_err(|e| e.to_string())?;
        if changed != 1 {
            return Err("The chunk is no longer in the library.".into());
        }
        Ok(())
    }

    pub fn embedding_counts(&self, profile: &str) -> Result<(usize, usize)> {
        validate_profile(profile)?;
        self.conn
            .query_row(
                "SELECT
                 (SELECT count(*) FROM embeddings WHERE profile=?1),
                 (SELECT count(*) FROM chunks)",
                [profile],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map_err(|e| e.to_string())
    }

    pub fn semantic_search(&self, profile: &str, vector: &[f32]) -> Result<Vec<Hit>> {
        validate_profile(profile)?;
        validate_embedding(vector)?;
        let mut stmt = self
            .conn
            .prepare("SELECT e.chunk_id,e.vector FROM embeddings e WHERE e.profile=?1")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([profile], |row| {
                let id: String = row.get(0)?;
                let bytes: Vec<u8> = row.get(1)?;
                Ok((id, decode_embedding(&bytes)))
            })
            .map_err(|e| e.to_string())?;
        let mut winners: Vec<(f32, String)> = Vec::with_capacity(8);
        for row in rows {
            let (id, stored) = row.map_err(|e| e.to_string())?;
            let stored = stored?;
            let score = vector
                .iter()
                .zip(stored.iter())
                .map(|(a, b)| a * b)
                .sum::<f32>();
            winners.push((score, id));
            winners.sort_by(score_then_id);
            winners.truncate(8);
        }
        winners
            .into_iter()
            .map(|(_, id)| self.hit_by_id(&id))
            .collect()
    }

    fn hit_by_id(&self, id: &str) -> Result<Hit> {
        self.conn
            .query_row(
                "SELECT c.id,c.document_id,d.title,c.ordinal,c.text,d.kind,p.start_ms,p.end_ms,pp.page_number
                 FROM chunks c JOIN documents d ON d.id=c.document_id
                 LEFT JOIN media_parts p ON p.chunk_id=c.id
                 LEFT JOIN pdf_chunk_pages pp ON pp.chunk_id=c.id WHERE c.id=?1",
                [id],
                hit_from_row,
            )
            .map_err(|_| "Embedding references a missing chunk.".into())
    }

    pub fn document_text(&self, id: &str) -> Result<String> {
        self.conn
            .query_row("SELECT content FROM documents WHERE id=?1", [id], |r| {
                r.get(0)
            })
            .map_err(|_| "Document is no longer in the library.".into())
    }

    pub fn media_part(&self, chunk_id: &str) -> Result<Option<crate::media::MediaPart>> {
        self.media_part_bounded(chunk_id, crate::media::MAX_MEDIA_BYTES)
    }

    pub fn media_part_bounded(
        &self,
        chunk_id: &str,
        max_bytes: usize,
    ) -> Result<Option<crate::media::MediaPart>> {
        self.conn
            .query_row(
                "SELECT mime,CASE WHEN length(payload)<=?2 THEN payload ELSE NULL END,start_ms,end_ms,label FROM media_parts WHERE chunk_id=?1",
                params![chunk_id, max_bytes.min(crate::media::MAX_MEDIA_BYTES) as i64],
                |row| {
                    Ok(crate::media::MediaPart {
                        mime: row.get(0)?,
                        data: row.get(1)?,
                        start_ms: row.get(2)?,
                        end_ms: row.get(3)?,
                        label: row.get(4)?,
                    })
                },
            )
            .optional()
            .map_err(|e| e.to_string())
    }

    pub fn asset_info(&self, document_id: &str) -> Result<(String, usize)> {
        self.conn
            .query_row(
                "SELECT mime,length(payload) FROM assets WHERE document_id=?1",
                [document_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "Media asset is no longer in the library.".into())
    }

    pub fn asset_range(&self, document_id: &str, start: usize, length: usize) -> Result<Vec<u8>> {
        if length > MAX_MEDIA_SOURCE_BYTES {
            return Err("Media byte ranges are limited to 32 MB.".into());
        }
        let (_, bytes) = self.asset_info(document_id)?;
        let end = start
            .checked_add(length)
            .ok_or_else(|| "Media byte range overflows.".to_string())?;
        if start > bytes || end > bytes {
            return Err("Media byte range is outside the asset.".into());
        }
        self.conn
            .query_row(
                "SELECT substr(payload,?2,?3) FROM assets WHERE document_id=?1",
                params![document_id, start + 1, length],
                |row| row.get(0),
            )
            .map_err(|e| e.to_string())
    }

    pub fn delete(&mut self, id: &str) -> Result<()> {
        let tx = self.conn.transaction().map_err(|e| e.to_string())?;
        tx.execute("DELETE FROM chunks_fts WHERE document_id=?1", [id])
            .map_err(|e| e.to_string())?;
        tx.execute("DELETE FROM documents WHERE id=?1", [id])
            .map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())
    }
}

fn migrate_legacy_embeddings_schema(conn: &mut Connection) -> Result<()> {
    let legacy_composite_key = {
        let mut statement = conn
            .prepare("PRAGMA table_info(embeddings)")
            .map_err(|e| e.to_string())?;
        let primary_key_columns = statement
            .query_map([], |row| row.get::<_, i64>(5))
            .map_err(|e| e.to_string())?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        primary_key_columns
            .iter()
            .filter(|position| **position > 0)
            .count()
            > 1
    };
    if !legacy_composite_key {
        return Ok(());
    }

    let transaction = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|e| e.to_string())?;
    transaction
        .execute_batch(
            "CREATE TABLE embeddings_replacement (
                chunk_id TEXT PRIMARY KEY REFERENCES chunks(id) ON DELETE CASCADE,
                profile TEXT NOT NULL,
                vector BLOB NOT NULL
             ) WITHOUT ROWID;
             INSERT INTO embeddings_replacement(chunk_id,profile,vector)
             SELECT source.chunk_id,source.profile,source.vector
             FROM embeddings source
             WHERE NOT EXISTS (
                 SELECT 1 FROM embeddings candidate
                 WHERE candidate.chunk_id=source.chunk_id
                   AND candidate.profile < source.profile
             );
             DROP TABLE embeddings;
             ALTER TABLE embeddings_replacement RENAME TO embeddings;",
        )
        .map_err(|e| e.to_string())?;
    transaction.commit().map_err(|e| e.to_string())
}

fn validate_title(title: &str) -> Result<&str> {
    let title = title.trim();
    if title.is_empty() || title.chars().count() > 240 {
        return Err("Document title must contain 1 to 240 characters.".into());
    }
    Ok(title)
}

fn validate_media(media: &crate::media::PreparedMedia) -> Result<()> {
    let valid_kind_mime = matches!(
        (media.kind.as_str(), media.mime.as_str()),
        ("image", "image/png") | ("audio", "audio/wav") | ("video", "video/mp4")
    );
    if !valid_kind_mime {
        return Err("Media must be a supported image, audio, or video type.".into());
    }
    if media.source.is_empty() || media.source.len() > MAX_MEDIA_SOURCE_BYTES {
        return Err("Media source files must contain 1 byte to 32 MB.".into());
    }
    if media.parts.is_empty() || media.parts.len() > MAX_MEDIA_PARTS {
        return Err("Prepared media must contain 1 to 36 parts.".into());
    }
    // `media::prepare` hashes the original selected file before it produces a
    // bounded normalized preview in `source`. Keep that stable identity while
    // validating its canonical SHA-256 representation at this boundary.
    if media.fingerprint.len() != 64
        || !media
            .fingerprint
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
    {
        return Err("Media fingerprint must be a SHA-256 digest.".into());
    }
    let mut prepared_bytes = media.source.len();
    for part in &media.parts {
        if !is_allowed_part_mime(&media.kind, &part.mime)
            || part.data.is_empty()
            || part.data.len() > MAX_MEDIA_PART_BYTES
        {
            return Err("Media parts must contain supported nonempty payloads.".into());
        }
        if part.label.trim().is_empty() || part.label.chars().count() > 1_000 {
            return Err("Media part labels must contain 1 to 1,000 characters.".into());
        }
        match (&media.kind[..], part.start_ms, part.end_ms) {
            ("image", None, None) => {}
            ("audio" | "video", Some(start), Some(end))
                if start < end && end <= MAX_MEDIA_DURATION_MS => {}
            ("audio" | "video", Some(_), Some(_)) => {
                return Err(
                    "Timed media parts must have increasing times within 120 seconds.".into(),
                )
            }
            _ => {
                return Err("Image parts are untimed; audio and video parts require times.".into())
            }
        }
        prepared_bytes = prepared_bytes
            .checked_add(part.data.len())
            .ok_or_else(|| "Prepared media is too large.".to_string())?;
    }
    if prepared_bytes > MAX_MEDIA_PREPARED_BYTES {
        return Err("Prepared media must use at most 64 MB.".into());
    }
    Ok(())
}

fn validate_pdf(pdf: &crate::pdf::PreparedPdf) -> Result<()> {
    if pdf.source.is_empty()
        || pdf.source.len() > crate::pdf::MAX_PDF_BYTES
        || !pdf.source.starts_with(b"%PDF-")
    {
        return Err("PDF source must be a PDF file of at most 16 MB.".into());
    }
    if pdf.pages.is_empty() || pdf.pages.len() > crate::pdf::MAX_PDF_PAGES {
        return Err("PDFs must contain 1 to 200 pages.".into());
    }
    let mut total_text_bytes = 0usize;
    for (index, page) in pdf.pages.iter().enumerate() {
        if page.number != (index + 1) as u32 || page.text.contains('\0') {
            return Err("PDF pages must have sequential page numbers and valid text.".into());
        }
        total_text_bytes = total_text_bytes
            .checked_add(page.text.len())
            .ok_or_else(|| "PDF text is too large.".to_string())?;
    }
    if total_text_bytes > crate::pdf::MAX_PDF_TEXT_BYTES
        || pdf.pages.iter().all(|page| page.text.trim().is_empty())
    {
        return Err("PDFs must contain nonempty extracted text of at most 2 MB.".into());
    }
    Ok(())
}

fn pdf_source_id(source: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"cachella:pdf:source:v1\0");
    hasher.update(source);
    format!("{:x}", hasher.finalize())
}

fn is_allowed_part_mime(kind: &str, mime: &str) -> bool {
    match kind {
        "image" => mime == "image/png",
        "audio" => mime == "audio/wav",
        "video" => matches!(mime, "image/png" | "audio/wav"),
        _ => false,
    }
}

fn media_preview(title: &str, media: &crate::media::PreparedMedia) -> String {
    let labels = media
        .parts
        .iter()
        .map(|part| part.label.trim())
        .collect::<Vec<_>>()
        .join("; ");
    format!("{title}\n{} media\n{labels}", media.kind)
}

fn media_part_text(title: &str, part: &crate::media::MediaPart) -> String {
    let timing = match (part.start_ms, part.end_ms) {
        (Some(start), Some(end)) => format!(" [{start}-{end} ms]"),
        _ => String::new(),
    };
    format!("{title}: {}{timing}", part.label.trim())
}

fn document_by_id(transaction: &rusqlite::Transaction<'_>, id: &str) -> Result<Option<Document>> {
    transaction
        .query_row("SELECT d.id,d.title,d.kind,d.bytes,(SELECT count(*) FROM chunks WHERE document_id=d.id),d.created_at FROM documents d WHERE d.id=?1", [id], |r|Ok(Document{id:r.get(0)?,title:r.get(1)?,kind:r.get(2)?,bytes:r.get(3)?,chunks:r.get(4)?,created_at:r.get(5)?}))
        .optional()
        .map_err(|e| e.to_string())
}

fn hit_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Hit> {
    let value = row.get_ref(4)?;
    let text = value.as_str().map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(4, value.data_type(), Box::new(error))
    })?;
    Ok(Hit {
        id: row.get(0)?,
        document_id: row.get(1)?,
        title: row.get(2)?,
        ordinal: row.get(3)?,
        text: hit_text_preview(text),
        kind: row.get(5)?,
        start_ms: row.get(6)?,
        end_ms: row.get(7)?,
        page_number: row.get(8)?,
    })
}

// Bound every retrieval/indexing projection, including previously stored chunks.
// Keep the first 1,800 Unicode characters intact for generation (at most 7,200
// bytes), and the embedding prefix intact. Full source inspection is unchanged.
fn hit_text_preview(text: &str) -> String {
    const SUFFIX: &str = "\n[Preview truncated; open source for full text.]";
    if text.len() <= MAX_HIT_TEXT_BYTES {
        return text.to_owned();
    }
    let mut end = MAX_HIT_TEXT_BYTES - SUFFIX.len();
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    let mut preview = String::with_capacity(MAX_HIT_TEXT_BYTES);
    preview.push_str(&text[..end]);
    preview.push_str(SUFFIX);
    preview
}

fn score_then_id(
    (score_a, id_a): &(f32, String),
    (score_b, id_b): &(f32, String),
) -> std::cmp::Ordering {
    score_b.total_cmp(score_a).then_with(|| id_a.cmp(id_b))
}

fn validate_profile(profile: &str) -> Result<()> {
    if profile.trim().is_empty() || profile.chars().count() > MAX_EMBEDDING_PROFILE_CHARS {
        return Err("Embedding profiles must contain 1 to 128 characters.".into());
    }
    Ok(())
}

fn validate_embedding(vector: &[f32]) -> Result<()> {
    if vector.len() != EMBEDDING_DIMENSIONS {
        return Err("Embeddings must contain exactly 768 dimensions.".into());
    }
    if vector.iter().any(|value| !value.is_finite()) {
        return Err("Embeddings must contain only finite values.".into());
    }
    let norm = vector.iter().map(|value| value * value).sum::<f32>().sqrt();
    if !(0.999..=1.001).contains(&norm) {
        return Err("Embeddings must be normalized to unit length.".into());
    }
    Ok(())
}

fn encode_embedding(vector: &[f32]) -> Vec<u8> {
    vector
        .iter()
        .flat_map(|value| value.to_le_bytes())
        .collect()
}

fn decode_embedding(bytes: &[u8]) -> Result<Vec<f32>> {
    if bytes.len() != EMBEDDING_DIMENSIONS * std::mem::size_of::<f32>() {
        return Err("Stored embedding has an invalid length.".into());
    }
    let vector = bytes
        .as_chunks::<4>()
        .0
        .iter()
        .map(|chunk| f32::from_le_bytes(*chunk))
        .collect::<Vec<_>>();
    validate_embedding(&vector)?;
    Ok(vector)
}

pub fn read_text_bounded(reader: impl std::io::Read) -> Result<String> {
    use std::io::Read;
    let mut bytes = Vec::new();
    reader
        .take(MAX_DOCUMENT_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "Cannot read this file.".to_string())?;
    if bytes.len() > MAX_DOCUMENT_BYTES {
        return Err("Choose a text file of at most 2 MB.".into());
    }
    String::from_utf8(bytes).map_err(|_| "File must contain valid UTF-8 text.".into())
}

pub fn read_text_file_bounded(path: &Path) -> Result<String> {
    let file = crate::regular_file::open_regular_file(path).map_err(|error| {
        if error.kind() == std::io::ErrorKind::InvalidInput {
            "Choose a text file of at most 2 MB.".to_string()
        } else {
            "Cannot open this file.".to_string()
        }
    })?;
    read_text_bounded(file)
}

pub fn chunks(text: &str, size: usize, overlap: usize) -> Vec<String> {
    if size == 0 {
        return Vec::new();
    }
    let words: Vec<_> = text.split_whitespace().collect();
    let step = size.saturating_sub(overlap).max(1);
    let mut result = Vec::new();
    let mut start = 0;
    while start < words.len() {
        let end = (start + size).min(words.len());
        result.push(words[start..end].join(" "));
        if end == words.len() {
            break;
        }
        start += step;
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn oversized_passages_have_bounded_projections_and_preserve_sources() {
        let tmp = tempfile::tempdir().unwrap();
        let mut db = Store::open(&tmp.path().join("workspace.sqlite")).unwrap();
        // No whitespace: the existing word-based chunker produces one passage.
        let text = "界".repeat(MAX_DOCUMENT_BYTES / 3);
        let doc = db.import_text("projectionmatch", "txt", &text).unwrap();
        assert_eq!(doc.chunks, 1);
        let hits = db.search("projectionmatch").unwrap();
        assert_eq!(hits.len(), 1);
        assert!(hits[0].text.len() <= 8192);
        assert!(hits[0]
            .text
            .ends_with("[Preview truncated; open source for full text.]"));
        assert_eq!(
            hits[0].text.chars().take(1800).collect::<String>(),
            text.chars().take(1800).collect::<String>()
        );
        assert_eq!(db.document_text(&doc.id).unwrap(), text);
        let pending = db.pending_embeddings("projection", 32).unwrap();
        assert_eq!(pending[0].text, hits[0].text);
        db.save_embedding(&pending[0].id, "projection", &normalized(0))
            .unwrap();
        let semantic = db.semantic_search("projection", &normalized(0)).unwrap();
        assert_eq!(semantic[0].text, hits[0].text);
    }

    #[test]
    fn passage_projection_preserves_exact_boundary_and_model_prefixes() {
        let exact = "x".repeat(MAX_HIT_TEXT_BYTES);
        assert_eq!(hit_text_preview(&exact), exact);
        let oversized = "🦀".repeat(MAX_HIT_TEXT_BYTES / 4 + 1);
        let preview = hit_text_preview(&oversized);
        assert!(preview.len() <= MAX_HIT_TEXT_BYTES);
        assert_eq!(
            preview.chars().take(1800).collect::<String>(),
            oversized.chars().take(1800).collect::<String>()
        );
        assert!(preview.ends_with("[Preview truncated; open source for full text.]"));
    }

    #[test]
    fn chunks_preserve_unicode_and_overlap() {
        let actual = chunks("Héllo 世界 launch moved to Friday", 4, 1);
        assert_eq!(actual, vec!["Héllo 世界 launch moved", "moved to Friday"]);
    }

    #[test]
    fn empty_documents_have_no_chunks() {
        assert!(chunks(" \n\t", 400, 40).is_empty());
    }

    #[test]
    fn invalid_chunk_settings_do_not_loop() {
        assert!(chunks("one two", 0, 0).is_empty());
        assert_eq!(chunks("one two three", 2, 3), vec!["one two", "two three"]);
    }

    #[test]
    fn imported_document_survives_restart_and_is_searchable() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("workspace.sqlite");
        let doc = Store::open(&path)
            .unwrap()
            .import_text(
                "Launch decision",
                "md",
                "The launch moved to Friday because security review is incomplete.",
            )
            .unwrap();
        let db = Store::open(&path).unwrap();
        assert_eq!(db.documents().unwrap().len(), 1);
        assert_eq!(
            db.search("Why launch Friday?").unwrap()[0].document_id,
            doc.id
        );
        assert!(db.document_text(&doc.id).unwrap().contains("security"));
    }

    #[test]
    fn deduplicates_content_and_removes_all_search_evidence_on_delete() {
        let mut db = Store::open(Path::new(":memory:")).unwrap();
        let first = db
            .import_text("One", "txt", "Unique zeppelin evidence")
            .unwrap();
        let second = db
            .import_text("Renamed", "txt", "Unique zeppelin evidence")
            .unwrap();
        assert_eq!(first.id, second.id);
        assert_eq!(db.documents().unwrap().len(), 1);
        db.delete(&first.id).unwrap();
        assert!(db.search("zeppelin").unwrap().is_empty());
        assert!(db.documents().unwrap().is_empty());
        assert!(db.document_text(&first.id).is_err());
    }

    #[test]
    fn search_treats_query_syntax_as_plain_text() {
        let mut db = Store::open(Path::new(":memory:")).unwrap();
        db.import_text("Budget", "txt", "The project budget is ten dollars.")
            .unwrap();
        assert!(!db.search("\"budget\" OR title:* --").unwrap().is_empty());
        assert!(db.search("\"*() :").unwrap().is_empty());
        assert_eq!(db.documents().unwrap().len(), 1);
    }

    #[test]
    fn rejects_empty_oversized_and_binary_documents() {
        let mut db = Store::open(Path::new(":memory:")).unwrap();
        assert!(db.import_text("Empty", "txt", "  ").is_err());
        assert!(db.import_text("Binary", "txt", "hello\0world").is_err());
        assert!(db
            .import_text("Large", "txt", &"x".repeat(MAX_DOCUMENT_BYTES + 1))
            .is_err());
        assert!(db.documents().unwrap().is_empty());
    }

    #[test]
    fn file_reader_enforces_limit_during_read() {
        let data = vec![b'a'; MAX_DOCUMENT_BYTES + 100];
        let mut reader = std::io::Cursor::new(data);
        assert!(read_text_bounded(&mut reader).is_err());
        assert_eq!(reader.position(), MAX_DOCUMENT_BYTES as u64 + 1);
        assert!(read_text_bounded(&[0xff][..]).is_err());
        assert_eq!(
            read_text_bounded("Hello 世界".as_bytes()).unwrap(),
            "Hello 世界"
        );
    }

    #[test]
    fn concurrent_imports_respect_capacity() {
        use std::sync::{Arc, Barrier};
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("capacity.sqlite");
        let db = Store::open(&path).unwrap();
        db.conn.execute_batch("WITH RECURSIVE n(x) AS (SELECT 1 UNION ALL SELECT x+1 FROM n WHERE x<499) INSERT INTO documents(id,title,kind,content,bytes) SELECT cast(x AS TEXT),'Fixture','txt','fixture',7 FROM n;").unwrap();
        let gate = Arc::new(Barrier::new(2));
        let threads: Vec<_> = (0..2)
            .map(|i| {
                let path = path.clone();
                let gate = gate.clone();
                std::thread::spawn(move || {
                    let mut db = Store::open(&path).unwrap();
                    gate.wait();
                    db.import_text("Concurrent", "txt", &format!("Unique import {i}"))
                        .is_ok()
                })
            })
            .collect();
        let results: Vec<_> = threads.into_iter().map(|t| t.join().unwrap()).collect();
        assert_eq!(results.iter().filter(|success| **success).count(), 1);
        assert_eq!(db.documents().unwrap().len(), 500);
    }

    fn normalized(value_at: usize) -> Vec<f32> {
        let mut vector = vec![0.0; 768];
        vector[value_at] = 1.0;
        vector
    }

    #[test]
    fn embeddings_persist_and_resume_pending_chunks_after_restart() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("workspace.sqlite");
        let mut db = Store::open(&path).unwrap();
        let doc = db.import_text("Launch", "txt", "alpha beta gamma").unwrap();
        let pending = db.pending_embeddings("embeddinggemma-2-q8", 32).unwrap();
        assert_eq!(pending.len(), 1);
        db.save_embedding(&pending[0].id, "embeddinggemma-2-q8", &normalized(7))
            .unwrap();
        drop(db);

        let db = Store::open(&path).unwrap();
        assert_eq!(db.embedding_counts("embeddinggemma-2-q8").unwrap(), (1, 1));
        assert!(db
            .pending_embeddings("embeddinggemma-2-q8", 32)
            .unwrap()
            .is_empty());
        assert_eq!(
            db.semantic_search("embeddinggemma-2-q8", &normalized(7))
                .unwrap()[0]
                .document_id,
            doc.id
        );
    }

    #[test]
    fn embeddings_are_profile_specific_and_pending_is_capped() {
        let mut db = Store::open(Path::new(":memory:")).unwrap();
        let first = db.import_text("First", "txt", "alpha evidence").unwrap();
        db.import_text("Second", "txt", "beta evidence").unwrap();
        let profile_a = "embeddinggemma-2-q8";
        let profile_b = "embeddinggemma-2-q4";
        let pending = db.pending_embeddings(profile_a, 99).unwrap();
        assert_eq!(pending.len(), 2);
        assert_eq!(db.pending_embeddings(profile_a, 0).unwrap().len(), 0);
        let first_chunk = pending
            .iter()
            .find(|hit| hit.document_id == first.id)
            .unwrap();
        let second_chunk = pending
            .iter()
            .find(|hit| hit.document_id != first.id)
            .unwrap();
        db.save_embedding(&first_chunk.id, profile_a, &normalized(1))
            .unwrap();
        db.save_embedding(&second_chunk.id, profile_a, &normalized(2))
            .unwrap();
        assert_eq!(db.embedding_counts(profile_a).unwrap(), (2, 2));
        assert_eq!(db.embedding_counts(profile_b).unwrap(), (0, 2));
        assert_eq!(db.pending_embeddings(profile_b, 32).unwrap().len(), 2);
        assert_eq!(
            db.semantic_search(profile_a, &normalized(1)).unwrap()[0].document_id,
            first.id
        );
        assert!(db
            .semantic_search(profile_b, &normalized(1))
            .unwrap()
            .is_empty());
        db.save_embedding(&first_chunk.id, profile_b, &normalized(3))
            .unwrap();
        assert_eq!(db.embedding_counts(profile_a).unwrap(), (1, 2));
        assert_eq!(db.embedding_counts(profile_b).unwrap(), (1, 2));
    }

    #[test]
    fn embedding_validation_rejects_malformed_vectors_and_delete_cascades() {
        let mut db = Store::open(Path::new(":memory:")).unwrap();
        let doc = db.import_text("One", "txt", "alpha evidence").unwrap();
        let chunk = db.pending_embeddings("profile", 1).unwrap().remove(0);
        assert!(db.save_embedding(&chunk.id, "", &normalized(1)).is_err());
        assert!(db
            .save_embedding(&chunk.id, &"x".repeat(129), &normalized(1))
            .is_err());
        assert!(db
            .save_embedding(&chunk.id, "profile", &[0.0; 767])
            .is_err());
        let mut non_finite = normalized(1);
        non_finite[3] = f32::NAN;
        assert!(db
            .save_embedding(&chunk.id, "profile", &non_finite)
            .is_err());
        assert!(db
            .save_embedding(&chunk.id, "profile", &vec![0.0; 768])
            .is_err());
        db.save_embedding(&chunk.id, "profile", &normalized(1))
            .unwrap();
        db.delete(&doc.id).unwrap();
        assert_eq!(db.embedding_counts("profile").unwrap(), (0, 0));
        assert!(db
            .semantic_search("profile", &normalized(1))
            .unwrap()
            .is_empty());
    }

    #[test]
    fn semantic_search_returns_only_top_eight_and_breaks_ties_by_chunk_id() {
        let mut db = Store::open(Path::new(":memory:")).unwrap();
        for index in 0..9 {
            db.import_text(
                &format!("Document {index}"),
                "txt",
                &format!("Evidence {index}"),
            )
            .unwrap();
        }
        let pending = db.pending_embeddings("profile", 32).unwrap();
        for hit in &pending {
            db.save_embedding(&hit.id, "profile", &normalized(0))
                .unwrap();
        }
        let expected = pending
            .iter()
            .map(|hit| hit.id.clone())
            .take(8)
            .collect::<Vec<_>>();
        let actual = db
            .semantic_search("profile", &normalized(0))
            .unwrap()
            .into_iter()
            .map(|hit| hit.id)
            .collect::<Vec<_>>();
        assert_eq!(actual, expected);
        assert_eq!(actual.len(), 8);
    }

    #[test]
    fn migrates_legacy_multi_profile_embeddings_without_losing_documents() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("legacy.sqlite");
        let mut db = Store::open(&path).unwrap();
        let doc = db
            .import_text("Legacy document", "txt", "existing document content")
            .unwrap();
        let chunk_id = db.pending_embeddings("alpha", 1).unwrap().remove(0).id;
        db.conn
            .execute_batch(
                "DROP TABLE embeddings;
                 CREATE TABLE embeddings (
                   chunk_id TEXT NOT NULL REFERENCES chunks(id) ON DELETE CASCADE,
                   profile TEXT NOT NULL,
                   vector BLOB NOT NULL,
                   PRIMARY KEY(chunk_id, profile)
                 ) WITHOUT ROWID;",
            )
            .unwrap();
        db.conn
            .execute(
                "INSERT INTO embeddings(chunk_id,profile,vector) VALUES(?1,?2,?3)",
                params![chunk_id, "zeta", encode_embedding(&normalized(2))],
            )
            .unwrap();
        db.conn
            .execute(
                "INSERT INTO embeddings(chunk_id,profile,vector) VALUES(?1,?2,?3)",
                params![chunk_id, "alpha", encode_embedding(&normalized(1))],
            )
            .unwrap();
        drop(db);

        let migrated = Store::open(&path).unwrap();
        assert_eq!(migrated.documents().unwrap()[0].id, doc.id);
        assert_eq!(migrated.embedding_counts("alpha").unwrap(), (1, 1));
        assert_eq!(migrated.embedding_counts("zeta").unwrap(), (0, 1));
        assert_eq!(
            migrated.semantic_search("alpha", &normalized(1)).unwrap()[0].id,
            chunk_id
        );
        drop(migrated);
        let mut migrated = Store::open(&path).unwrap();
        assert_eq!(migrated.embedding_counts("alpha").unwrap(), (1, 1));
        migrated
            .save_embedding(&chunk_id, "new-profile", &normalized(3))
            .unwrap();
        assert_eq!(migrated.embedding_counts("new-profile").unwrap(), (1, 1));
        migrated.delete(&doc.id).unwrap();
        assert_eq!(migrated.embedding_counts("new-profile").unwrap(), (0, 0));
    }

    #[test]
    fn legacy_embedding_migration_rolls_back_when_a_row_is_orphaned() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("corrupt-legacy.sqlite");
        let mut db = Store::open(&path).unwrap();
        db.import_text("Source document", "txt", "source text survives failure")
            .unwrap();
        db.conn
            .execute_batch(
                "DROP TABLE embeddings;
                 CREATE TABLE embeddings (
                   chunk_id TEXT NOT NULL REFERENCES chunks(id) ON DELETE CASCADE,
                   profile TEXT NOT NULL,
                   vector BLOB NOT NULL,
                   PRIMARY KEY(chunk_id, profile)
                 ) WITHOUT ROWID;
                 PRAGMA foreign_keys=OFF;",
            )
            .unwrap();
        db.conn
            .execute(
                "INSERT INTO embeddings(chunk_id,profile,vector) VALUES(?1,?2,?3)",
                params!["missing-chunk", "alpha", encode_embedding(&normalized(1))],
            )
            .unwrap();
        drop(db);

        assert!(Store::open(&path).is_err());
        let conn = Connection::open(&path).unwrap();
        let primary_key_columns: usize = conn
            .query_row(
                "SELECT count(*) FROM pragma_table_info('embeddings') WHERE pk > 0",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let embedding_rows: usize = conn
            .query_row("SELECT count(*) FROM embeddings", [], |row| row.get(0))
            .unwrap();
        let source_documents: usize = conn
            .query_row("SELECT count(*) FROM documents", [], |row| row.get(0))
            .unwrap();
        let replacement_tables: usize = conn
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='embeddings_replacement'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(primary_key_columns, 2);
        assert_eq!(embedding_rows, 1);
        assert_eq!(source_documents, 1);
        assert_eq!(replacement_tables, 0);
    }

    fn prepared_image(label: &str) -> crate::media::PreparedMedia {
        crate::media::PreparedMedia {
            kind: "image".into(),
            mime: "image/png".into(),
            // A normalized preview is intentionally allowed to differ from
            // the original bytes represented by the deduplication digest.
            source: b"\x89PNG\r\n\x1a\nnormalized-preview".to_vec(),
            fingerprint: format!("{:x}", Sha256::digest(b"original-selected-image")),
            parts: vec![crate::media::MediaPart {
                mime: "image/png".into(),
                data: b"\x89PNG\r\n\x1a\nembedding-part".to_vec(),
                start_ms: None,
                end_ms: None,
                label: label.into(),
            }],
        }
    }

    fn prepared_audio(label: &str) -> crate::media::PreparedMedia {
        crate::media::PreparedMedia {
            kind: "audio".into(),
            mime: "audio/wav".into(),
            source: b"RIFFnormalized-preview".to_vec(),
            fingerprint: format!("{:x}", Sha256::digest(b"original-selected-audio")),
            parts: vec![crate::media::MediaPart {
                mime: "audio/wav".into(),
                data: b"RIFFembedding-part".to_vec(),
                start_ms: Some(0),
                end_ms: Some(10_000),
                label: label.into(),
            }],
        }
    }

    fn prepared_pdf(pages: Vec<(u32, &str)>) -> crate::pdf::PreparedPdf {
        crate::pdf::PreparedPdf {
            source: b"%PDF-1.7\npdf fixture".to_vec(),
            pages: pages
                .into_iter()
                .map(|(number, text)| crate::pdf::PdfPage {
                    number,
                    text: text.into(),
                })
                .collect(),
        }
    }

    #[test]
    fn pdf_schema_migrates_populated_old_library_without_losing_documents() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("old.sqlite");
        let mut db = Store::open(&path).unwrap();
        let text = db
            .import_text("Existing", "txt", "existing source survives pdf migration")
            .unwrap();
        db.conn
            .execute_batch("DROP TABLE pdf_chunk_pages; DROP TABLE pdf_pages;")
            .unwrap();
        drop(db);

        let db = Store::open(&path).unwrap();
        assert_eq!(
            db.document_text(&text.id).unwrap(),
            "existing source survives pdf migration"
        );
        assert_eq!(db.search("migration").unwrap()[0].document_id, text.id);
        assert!(db.pdf_pages(&text.id).is_err());
    }

    #[test]
    fn pdf_persists_pages_and_page_citations_across_search_paths() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("pdf.sqlite");
        let mut db = Store::open(&path).unwrap();
        let pdf = prepared_pdf(vec![
            (1, "Page one background"),
            (2, ""),
            (3, "Page three conclusion with satellite evidence"),
        ]);
        let document = db.import_pdf("Report", &pdf).unwrap();
        assert_eq!(document.kind, "pdf");
        assert_eq!(document.bytes, pdf.source.len());
        assert_eq!(
            page_pairs(&db.pdf_pages(&document.id).unwrap()),
            page_pairs(&pdf.pages)
        );
        assert_eq!(db.pdf_pages(&document.id).unwrap()[1].text, "");
        assert_eq!(
            db.asset_info(&document.id).unwrap(),
            ("application/pdf".into(), pdf.source.len())
        );
        assert_eq!(
            db.document_text(&document.id).unwrap(),
            "Page one background\n\n\n\nPage three conclusion with satellite evidence"
        );
        let hit = db.search("satellite").unwrap().remove(0);
        assert_eq!(hit.page_number, Some(3));
        db.save_embedding(&hit.id, "pdf-profile", &normalized(9))
            .unwrap();
        drop(db);

        let db = Store::open(&path).unwrap();
        assert_eq!(
            page_pairs(&db.pdf_pages(&document.id).unwrap()),
            page_pairs(&pdf.pages)
        );
        assert_eq!(
            db.semantic_search("pdf-profile", &normalized(9)).unwrap()[0].page_number,
            Some(3)
        );
        assert_eq!(
            db.pending_embeddings("pdf-profile", 32).unwrap()[0].page_number,
            Some(1)
        );
    }

    #[test]
    fn pdf_deduplicates_rejects_invalid_input_without_writing_and_cascades_delete() {
        let mut db = Store::open(Path::new(":memory:")).unwrap();
        let pdf = prepared_pdf(vec![(1, "A retained page")]);
        let first = db.import_pdf("First", &pdf).unwrap();
        let second = db.import_pdf("Renamed", &pdf).unwrap();
        assert_eq!(first.id, second.id);
        assert_eq!(db.documents().unwrap().len(), 1);

        let mut invalid = prepared_pdf(vec![(1, "Valid"), (3, "Skipped")]);
        assert!(db.import_pdf("Invalid pages", &invalid).is_err());
        invalid = prepared_pdf(vec![(1, "   ")]);
        assert!(db.import_pdf("Empty text", &invalid).is_err());
        invalid = prepared_pdf(vec![(1, "Text")]);
        invalid.source = b"not a pdf".to_vec();
        assert!(db.import_pdf("Invalid header", &invalid).is_err());
        assert_eq!(db.documents().unwrap().len(), 1);

        let hit = db.search("retained").unwrap().remove(0);
        db.save_embedding(&hit.id, "pdf-profile", &normalized(10))
            .unwrap();
        db.delete(&first.id).unwrap();
        assert!(db.pdf_pages(&first.id).is_err());
        assert!(db.pdf_pages("missing-document").is_err());
        assert!(db.asset_info(&first.id).is_err());
        assert!(db.search("retained").unwrap().is_empty());
        assert_eq!(db.embedding_counts("pdf-profile").unwrap(), (0, 0));
        let page_rows: usize = db
            .conn
            .query_row("SELECT count(*) FROM pdf_pages", [], |row| row.get(0))
            .unwrap();
        let metadata_rows: usize = db
            .conn
            .query_row("SELECT count(*) FROM pdf_chunk_pages", [], |row| row.get(0))
            .unwrap();
        assert_eq!((page_rows, metadata_rows), (0, 0));
    }

    #[test]
    fn pdf_respects_document_capacity_without_writing_partial_imports() {
        let mut db = Store::open(Path::new(":memory:")).unwrap();
        db.conn
            .execute_batch(
                "WITH RECURSIVE n(x) AS (
                    SELECT 1 UNION ALL SELECT x + 1 FROM n WHERE x < 500
                 )
                 INSERT INTO documents(id,title,kind,content,bytes)
                 SELECT cast(x AS TEXT), 'Fixture', 'txt', 'fixture', 7 FROM n;",
            )
            .unwrap();
        let pdf = prepared_pdf(vec![(1, "new PDF evidence")]);
        assert!(db.import_pdf("Over capacity", &pdf).is_err());
        let counts: (usize, usize, usize) = db
            .conn
            .query_row(
                "SELECT
                   (SELECT count(*) FROM documents),
                   (SELECT count(*) FROM assets),
                   (SELECT count(*) FROM pdf_pages)",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        assert_eq!(counts, (500, 0, 0));
    }

    #[test]
    fn pdf_respects_chunk_capacity_without_writing_partial_imports() {
        let mut db = Store::open(Path::new(":memory:")).unwrap();
        db.conn
            .execute_batch(
                "INSERT INTO documents(id,title,kind,content,bytes)
                 VALUES('fixture', 'Fixture', 'txt', 'fixture', 7);
                 WITH RECURSIVE n(x) AS (
                    SELECT 0 UNION ALL SELECT x + 1 FROM n WHERE x < 9999
                 )
                 INSERT INTO chunks(id,document_id,ordinal,text)
                 SELECT 'fixture:' || x, 'fixture', x, 'fixture' FROM n;",
            )
            .unwrap();
        let pdf = prepared_pdf(vec![(1, "new PDF evidence")]);
        assert!(db.import_pdf("Over chunk capacity", &pdf).is_err());
        let counts: (usize, usize, usize, usize) = db
            .conn
            .query_row(
                "SELECT
                   (SELECT count(*) FROM documents),
                   (SELECT count(*) FROM chunks),
                   (SELECT count(*) FROM assets),
                   (SELECT count(*) FROM pdf_pages)",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .unwrap();
        assert_eq!(counts, (1, 10_000, 0, 0));
    }

    fn page_pairs(pages: &[crate::pdf::PdfPage]) -> Vec<(u32, String)> {
        pages
            .iter()
            .map(|page| (page.number, page.text.clone()))
            .collect()
    }

    #[test]
    fn media_schema_migrates_current_text_library_without_losing_sources() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("current.sqlite");
        let mut db = Store::open(&path).unwrap();
        let text = db
            .import_text("Existing source", "txt", "existing source survives upgrade")
            .unwrap();
        db.conn
            .execute_batch("DROP TABLE assets; DROP TABLE media_parts;")
            .unwrap();
        drop(db);

        let db = Store::open(&path).unwrap();
        assert_eq!(
            db.document_text(&text.id).unwrap(),
            "existing source survives upgrade"
        );
        assert_eq!(db.search("survives").unwrap()[0].document_id, text.id);
        assert!(db.asset_info(&text.id).is_err());
    }

    #[test]
    fn media_persists_deduplicates_indexes_and_cascades_delete() {
        let mut db = Store::open(Path::new(":memory:")).unwrap();
        let media = prepared_image("Red circle quarterly budget slide");
        let first = db.import_media("Budget image", &media).unwrap();
        let second = db.import_media("Renamed", &media).unwrap();
        assert_eq!(first.id, second.id);
        assert_eq!(first.kind, "image");
        assert_eq!(first.bytes, media.source.len());
        assert_eq!(first.chunks, 1);
        assert_eq!(db.documents().unwrap().len(), 1);

        let hit = db.search("quarterly budget").unwrap().remove(0);
        assert_eq!(hit.document_id, first.id);
        assert_eq!(hit.kind, "image");
        assert_eq!(hit.start_ms, None);
        assert_eq!(
            db.media_part(&hit.id).unwrap().unwrap().data,
            media.parts[0].data
        );
        assert!(db
            .media_part_bounded(&hit.id, media.parts[0].data.len() - 1)
            .is_err());
        assert_eq!(
            db.media_part_bounded(&hit.id, media.parts[0].data.len())
                .unwrap()
                .unwrap()
                .data,
            media.parts[0].data
        );
        assert!(db.media_part_bounded("missing", 0).unwrap().is_none());
        assert_eq!(
            db.asset_info(&first.id).unwrap(),
            ("image/png".into(), media.source.len())
        );
        assert_eq!(db.asset_range(&first.id, 0, 8).unwrap(), media.source[..8]);
        assert_eq!(
            db.asset_range(&first.id, media.source.len(), 0).unwrap(),
            Vec::<u8>::new()
        );
        assert!(db.asset_range(&first.id, media.source.len(), 1).is_err());
        assert!(db.asset_range(&first.id, usize::MAX, 1).is_err());
        assert!(db
            .asset_range(&first.id, 0, MAX_MEDIA_SOURCE_BYTES + 1)
            .is_err());

        db.save_embedding(&hit.id, "media-profile", &normalized(3))
            .unwrap();
        let semantic = db.semantic_search("media-profile", &normalized(3)).unwrap();
        assert_eq!(semantic[0].kind, "image");
        assert_eq!(semantic[0].id, hit.id);
        db.delete(&first.id).unwrap();
        assert!(db.media_part(&hit.id).unwrap().is_none());
        assert!(db.asset_info(&first.id).is_err());
        assert_eq!(db.embedding_counts("media-profile").unwrap(), (0, 0));
    }

    #[test]
    fn media_rejects_invalid_boundary_data_without_writing() {
        let mut db = Store::open(Path::new(":memory:")).unwrap();
        let mut invalid = prepared_image("Image");
        invalid.kind = "pdf".into();
        assert!(db.import_media("Invalid", &invalid).is_err());
        invalid = prepared_image("Image");
        invalid.fingerprint = "xyz".into();
        assert!(db.import_media("Invalid", &invalid).is_err());
        invalid = prepared_image("Image");
        invalid.parts[0].start_ms = Some(10);
        invalid.parts[0].end_ms = Some(9);
        assert!(db.import_media("Invalid", &invalid).is_err());
        invalid = prepared_image("Image");
        invalid.parts[0].mime = "audio/wav".into();
        assert!(db.import_media("Invalid", &invalid).is_err());
        assert!(db.documents().unwrap().is_empty());
    }

    #[test]
    fn media_rejects_payloads_outside_normalized_downstream_contract() {
        let mut db = Store::open(Path::new(":memory:")).unwrap();
        let mut invalid = prepared_image("Image");
        invalid.mime = "image/jpeg".into();
        assert!(db.import_media("Invalid source MIME", &invalid).is_err());

        invalid = prepared_image("Image");
        invalid.parts[0].mime = "image/jpeg".into();
        assert!(db.import_media("Invalid part MIME", &invalid).is_err());

        invalid = prepared_image("Image");
        invalid.parts[0].data = vec![0; 2 * 1024 * 1024 + 1];
        assert!(db
            .import_media("Oversized embedding input", &invalid)
            .is_err());

        invalid = prepared_audio("Audio");
        invalid.parts[0].end_ms = Some(120_001);
        assert!(db.import_media("Out of duration", &invalid).is_err());

        invalid = prepared_audio("Audio");
        invalid.parts[0].start_ms = None;
        invalid.parts[0].end_ms = None;
        assert!(db.import_media("Untimed audio", &invalid).is_err());
        assert!(db.documents().unwrap().is_empty());
    }

    #[test]
    fn media_accepts_video_parts_grouped_by_modality_with_valid_times() {
        let mut db = Store::open(Path::new(":memory:")).unwrap();
        let mut video = prepared_image("Frame at 10 seconds");
        video.kind = "video".into();
        video.mime = "video/mp4".into();
        video.source = b"normalized-mp4-preview".to_vec();
        video.parts[0].start_ms = Some(10_000);
        video.parts[0].end_ms = Some(15_000);
        // The decoder groups visual frames before audio windows. Their timeline
        // remains valid even though the next stored part starts earlier.
        video.parts.push(crate::media::MediaPart {
            mime: "audio/wav".into(),
            data: b"RIFFaudio".to_vec(),
            start_ms: Some(0),
            end_ms: Some(10_000),
            label: "Video audio 0:00–0:10".into(),
        });
        let document = db.import_media("Video", &video).unwrap();
        assert_eq!(document.chunks, 2);
        assert_eq!(db.search("audio").unwrap()[0].start_ms, Some(0));
    }
}
