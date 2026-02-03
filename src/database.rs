use crate::add::grobid::EmbeddedPaper;
use rusqlite::{Connection, Transaction, params};
use std::path::Path;
use std::time::SystemTime;
use termion::color;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DbError {
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
}

pub type Result<T> = std::result::Result<T, DbError>;

#[derive(Debug, Clone)]
pub struct Paper {
    pub key: String,
    pub title: Option<String>,
    pub authors: Option<String>,
    pub year: Option<i32>,
    pub link: Option<String>,
    pub processed: bool,
    pub last_touched: i64,
}

impl Paper {
    pub fn display(&self, max_chars: usize) -> PaperDisplay<'_> {
        PaperDisplay {
            paper: self,
            max_chars,
            show_tags: false,
            show_year: true,
        }
    }
}

/// Builder for paper display formatting
pub struct PaperDisplay<'a> {
    paper: &'a Paper,
    max_chars: usize,
    show_tags: bool,
    show_year: bool,
}

impl<'a> PaperDisplay<'a> {
    pub fn with_tags(mut self) -> Self {
        self.show_tags = true;
        self
    }

    pub fn without_year(mut self) -> Self {
        self.show_year = false;
        self
    }
}

impl std::fmt::Display for PaperDisplay<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let year = if self.show_year {
            self.paper
                .year
                .map(|y| format!("{:4} ", y))
                .unwrap_or_else(|| "---- ".into())
        } else {
            String::new()
        };
        let authors = format_authors(self.paper.authors.as_deref());
        let title = self.paper.title.as_deref().unwrap_or("Untitled");

        if self.show_tags {
            // With tags: YEAR Authors Title [pdf/web]
            let (tag, colored_tag) = if self.paper.processed {
                (
                    "[pdf]",
                    format!("{}[pdf]{}", color::Fg(color::Red), color::Fg(color::Reset)),
                )
            } else if self.paper.link.is_some() {
                (
                    "[web]",
                    format!("{}[web]{}", color::Fg(color::Blue), color::Fg(color::Reset)),
                )
            } else {
                ("", String::new())
            };

            let prefix = format!("{}{} • ", year, authors);
            let prefix_len = prefix.chars().count();
            let tag_len = tag.len(); // ASCII so len() is fine

            // Reserve space for tag (plus one space before it if tag exists)
            let reserved = if tag.is_empty() { 0 } else { tag_len + 1 };
            let available = self.max_chars.saturating_sub(prefix_len + reserved);

            let truncated_title = truncate_str(title, available);

            let title_len = truncated_title.chars().count();
            let content_len = prefix_len + title_len + tag_len;
            let padding = self.max_chars.saturating_sub(content_len);

            write!(
                f,
                "{}{}{}{}",
                prefix,
                truncated_title,
                " ".repeat(padding),
                colored_tag,
            )
        } else {
            // Simple: YEAR Authors Title (truncated to max_chars)
            let base = format!("{}{} • ", year, authors);
            let available = self.max_chars.saturating_sub(base.chars().count());
            let title_truncated = truncate_str(title, available);
            write!(f, "{}{}", base, title_truncated)
        }
    }
}

fn truncate_str(s: &str, max_chars: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= max_chars {
        s.to_string()
    } else {
        format!(
            "{}...",
            chars[..max_chars.saturating_sub(3)].iter().collect::<String>()
        )
    }
}
pub fn format_authors(authors: Option<&str>) -> String {
    let Some(authors) = authors else {
        return "Unknown".into();
    };

    let names: Vec<&str> = authors.split(", ").collect();

    match names.len() {
        0 => "Unknown".into(),
        1 => names[0].into(),
        2 => format!("{} and {}", names[0], names[1]),
        _ => format!("{} et al.", names[0]),
    }
}

/// Lightweight struct for similarity computation (no text loaded)
pub struct ParagraphEmbedding {
    pub id: i64,
    pub embedding: Vec<f32>,
}

/// Full paragraph data for LLM context
pub struct ParagraphContext {
    pub id: i64,
    pub source_key: String,
    pub text: String,
    pub cited_keys: Vec<String>,
}

pub struct DbStats {
    pub paper_count: usize,
    pub paragraph_count: usize,
    pub citation_count: usize,
}

pub struct CitationDb {
    conn: Connection,
}

impl CitationDb {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let conn = Connection::open(path)?;
        let db = Self { conn };
        db.init_schema()?;
        db.set_pragmas()?;
        Ok(db)
    }

    fn set_pragmas(&self) -> Result<()> {
        self.conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             PRAGMA cache_size = -64000;
             PRAGMA temp_store = MEMORY;",
        )?;
        Ok(())
    }

    fn init_schema(&self) -> Result<()> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS papers (
                key TEXT PRIMARY KEY,
                title TEXT,
                authors TEXT,
                year INTEGER,
                link TEXT,
                last_touched INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS processed (
                key TEXT PRIMARY KEY
            );

            CREATE TABLE IF NOT EXISTS paragraphs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                source_key TEXT NOT NULL,
                text TEXT NOT NULL,
                embedding BLOB NOT NULL
            );

            CREATE TABLE IF NOT EXISTS paragraph_citations (
                paragraph_id INTEGER NOT NULL,
                cited_key TEXT NOT NULL,
                FOREIGN KEY (paragraph_id) REFERENCES paragraphs(id)
            );

            CREATE INDEX IF NOT EXISTS idx_paragraphs_source ON paragraphs(source_key);
            CREATE INDEX IF NOT EXISTS idx_paragraph_citations_paragraph ON paragraph_citations(paragraph_id);
            CREATE INDEX IF NOT EXISTS idx_paragraph_citations_cited ON paragraph_citations(cited_key);",
        )?;
        Ok(())
    }

    pub fn is_processed(&self, key: &str) -> Result<bool> {
        let exists: bool = self.conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM processed WHERE key = ?1)",
            [key],
            |row| row.get(0),
        )?;
        Ok(exists)
    }

    pub fn ingest(&mut self, paper: &EmbeddedPaper) -> Result<()> {
        let tx = self.conn.transaction()?;

        // Upsert source paper (no link - it has a local PDF)
        let source_year = paper.year.as_ref().and_then(|y| y.parse().ok());
        upsert_source_paper(&tx, &paper.key, &paper.title, &paper.authors, source_year)?;

        // Upsert all references (with link if available)
        for reference in &paper.references {
            let year = reference.year.as_ref().and_then(|y| y.parse().ok());
            upsert_reference(
                &tx,
                &reference.key,
                &reference.title,
                &reference.authors,
                year,
                reference.link.as_deref(),
            )?;
        }

        // Insert paragraphs and their citations
        for para in &paper.paragraphs {
            let blob = embedding_to_blob(&para.embedding);

            tx.execute(
                "INSERT INTO paragraphs (source_key, text, embedding) VALUES (?1, ?2, ?3)",
                params![&paper.key, &para.text, &blob],
            )?;

            let paragraph_id = tx.last_insert_rowid();

            // Insert citation links
            for cited_key in &para.cited_keys {
                tx.execute(
                    "INSERT INTO paragraph_citations (paragraph_id, cited_key) VALUES (?1, ?2)",
                    params![paragraph_id, cited_key],
                )?;
            }
        }

        // Mark as processed
        tx.execute(
            "INSERT OR IGNORE INTO processed (key) VALUES (?1)",
            [&paper.key],
        )?;
        tx.commit()?;
        Ok(())
    }

    /// Get all embeddings for similarity computation (lightweight, no text)
    pub fn get_all_embeddings(&self) -> Result<Vec<ParagraphEmbedding>> {
        let mut stmt = self.conn.prepare("SELECT id, embedding FROM paragraphs")?;

        let records = stmt
            .query_map([], |row| {
                Ok(ParagraphEmbedding {
                    id: row.get(0)?,
                    embedding: blob_to_embedding(&row.get::<_, Vec<u8>>(1)?),
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(records)
    }

    /// Get full paragraph context for a set of paragraph IDs (for LLM)
    pub fn get_paragraph_contexts(&self, ids: &[i64]) -> Result<Vec<ParagraphContext>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }

        let placeholders = ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");

        // Get paragraph text and source
        let sql = format!(
            "SELECT id, source_key, text FROM paragraphs WHERE id IN ({})",
            placeholders
        );

        let mut stmt = self.conn.prepare(&sql)?;
        let params: Vec<&dyn rusqlite::ToSql> =
            ids.iter().map(|id| id as &dyn rusqlite::ToSql).collect();

        let mut paragraphs: Vec<ParagraphContext> = stmt
            .query_map(params.as_slice(), |row| {
                Ok(ParagraphContext {
                    id: row.get(0)?,
                    source_key: row.get(1)?,
                    text: row.get(2)?,
                    cited_keys: Vec::new(), // fill in next
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        // Get cited keys for each paragraph
        for para in &mut paragraphs {
            let mut cite_stmt = self
                .conn
                .prepare("SELECT cited_key FROM paragraph_citations WHERE paragraph_id = ?1")?;

            para.cited_keys = cite_stmt
                .query_map([para.id], |row| row.get(0))?
                .collect::<std::result::Result<Vec<_>, _>>()?;
        }

        Ok(paragraphs)
    }

    /// Get papers by keys. If keys is empty, returns all papers.
    /// If processed_only is true, only returns papers that have been processed (have PDFs).
    pub fn get_papers(&self, keys: &[&str], processed_only: bool) -> Result<Vec<Paper>> {
        let base_query = "SELECT p.key, p.title, p.authors, p.year, p.link,
                          (pr.key IS NOT NULL) as processed, p.last_touched
                          FROM papers p
                          LEFT JOIN processed pr ON p.key = pr.key";

        let papers = if keys.is_empty() {
            // Get all papers (optionally filtered by processed status)
            let sql = if processed_only {
                format!("{} WHERE pr.key IS NOT NULL ORDER BY p.last_touched DESC", base_query)
            } else {
                format!("{} ORDER BY p.last_touched DESC", base_query)
            };
            let mut stmt = self.conn.prepare(&sql)?;
            stmt.query_map([], |row| {
                Ok(Paper {
                    key: row.get(0)?,
                    title: row.get(1)?,
                    authors: row.get(2)?,
                    year: row.get(3)?,
                    link: row.get(4)?,
                    processed: row.get(5)?,
                    last_touched: row.get(6)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?
        } else {
            // Get specific papers
            let placeholders = keys.iter().map(|_| "?").collect::<Vec<_>>().join(",");
            let sql = format!("{} WHERE p.key IN ({})", base_query, placeholders);
            let mut stmt = self.conn.prepare(&sql)?;
            let params: Vec<&dyn rusqlite::ToSql> =
                keys.iter().map(|k| k as &dyn rusqlite::ToSql).collect();
            stmt.query_map(params.as_slice(), |row| {
                Ok(Paper {
                    key: row.get(0)?,
                    title: row.get(1)?,
                    authors: row.get(2)?,
                    year: row.get(3)?,
                    link: row.get(4)?,
                    processed: row.get(5)?,
                    last_touched: row.get(6)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?
        };

        Ok(papers)
    }

    /// Update the last_touched timestamp for a paper.
    /// Returns true if the paper was found and updated, false otherwise.
    pub fn touch_paper(&self, key: &str) -> Result<bool> {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let rows = self.conn.execute(
            "UPDATE papers SET last_touched = ?1 WHERE key = ?2",
            params![now, key],
        )?;
        Ok(rows > 0)
    }

    pub fn stats(&self) -> Result<DbStats> {
        let paper_count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM papers", [], |row| row.get(0))?;
        let paragraph_count: i64 =
            self.conn
                .query_row("SELECT COUNT(*) FROM paragraphs", [], |row| row.get(0))?;
        let citation_count: i64 =
            self.conn
                .query_row("SELECT COUNT(*) FROM paragraph_citations", [], |row| {
                    row.get(0)
                })?;

        Ok(DbStats {
            paper_count: paper_count as usize,
            paragraph_count: paragraph_count as usize,
            citation_count: citation_count as usize,
        })
    }
}

/// Upsert a source paper (one we processed). Clears link since we have the PDF.
fn upsert_source_paper(
    tx: &Transaction,
    key: &str,
    title: &str,
    authors: &str,
    year: Option<i32>,
) -> Result<()> {
    let title = if title.is_empty() { None } else { Some(title) };
    let authors = if authors.is_empty() {
        None
    } else {
        Some(authors)
    };

    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    tx.execute(
        "INSERT INTO papers (key, title, authors, year, link, last_touched) VALUES (?1, ?2, ?3, ?4, NULL, ?5)
         ON CONFLICT(key) DO UPDATE SET
             title = COALESCE(excluded.title, papers.title),
             authors = COALESCE(excluded.authors, papers.authors),
             year = COALESCE(excluded.year, papers.year),
             link = NULL,
             last_touched = excluded.last_touched",
        params![key, title, authors, year, now],
    )?;
    Ok(())
}

/// Upsert a reference paper. Only sets link if paper isn't already processed.
fn upsert_reference(
    tx: &Transaction,
    key: &str,
    title: &str,
    authors: &str,
    year: Option<i32>,
    link: Option<&str>,
) -> Result<()> {
    let title = if title.is_empty() { None } else { Some(title) };
    let authors = if authors.is_empty() {
        None
    } else {
        Some(authors)
    };

    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    tx.execute(
        "INSERT INTO papers (key, title, authors, year, link, last_touched) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
         ON CONFLICT(key) DO UPDATE SET
             title = COALESCE(excluded.title, papers.title),
             authors = COALESCE(excluded.authors, papers.authors),
             year = COALESCE(excluded.year, papers.year),
             link = CASE
                 WHEN (SELECT 1 FROM processed WHERE key = excluded.key) IS NULL
                 THEN COALESCE(excluded.link, papers.link)
                 ELSE papers.link
             END,
             last_touched = excluded.last_touched",
        params![key, title, authors, year, link, now],
    )?;
    Ok(())
}

fn embedding_to_blob(embedding: &[f32]) -> Vec<u8> {
    embedding.iter().flat_map(|f| f.to_le_bytes()).collect()
}

fn blob_to_embedding(blob: &[u8]) -> Vec<f32> {
    blob.chunks_exact(4)
        .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect()
}
