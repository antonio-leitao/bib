mod error;
pub use error::StorageError;

use crate::core::{Embedding, Paper};
use rusqlite::{params, Connection, Row};
use std::path::Path;

pub struct PaperStore {
    conn: Connection,
}

impl PaperStore {
    pub fn new<P: AsRef<Path>>(db_path: P) -> Result<Self, StorageError> {
        let conn = Connection::open(db_path)?;
        let store = PaperStore { conn };
        store.init_schema()?;
        Ok(store)
    }

    fn init_schema(&self) -> Result<(), StorageError> {
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS papers (
                id TEXT PRIMARY KEY,
                key TEXT UNIQUE NOT NULL,
                author TEXT NOT NULL,
                year INTEGER NOT NULL,
                title TEXT NOT NULL,
                notes TEXT,
                content TEXT NOT NULL,
                bibtex TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        )?;

        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS embeddings (
                paper_id TEXT PRIMARY KEY,
                embedding BLOB NOT NULL,
                dimensions INTEGER NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (paper_id) REFERENCES papers (id) ON DELETE CASCADE
            )",
            [],
        )?;

        Ok(())
    }

    fn row_to_paper(row: &Row) -> Result<Paper, StorageError> {
        let id_str: String = row.get("id")?;
        let id = id_str
            .parse::<u128>()
            .map_err(|e| StorageError::Deserialization(e.to_string()))?;

        Ok(Paper {
            id,
            key: row.get("key")?,
            author: row.get("author")?,
            year: row.get("year")?,
            title: row.get("title")?,
            notes: row.get("notes")?,
            content: row.get("content")?,
            bibtex: row.get("bibtex")?,
        })
    }

    fn row_to_embedding(row: &Row) -> Result<Embedding, StorageError> {
        let id_str: String = row.get("paper_id")?;
        let id = id_str
            .parse::<u128>()
            .map_err(|e| StorageError::Deserialization(e.to_string()))?;

        let embedding_bytes: Vec<u8> = row.get("embedding")?;
        let coords: Vec<f32> = bincode::deserialize(&embedding_bytes)
            .map_err(|e| StorageError::Deserialization(e.to_string()))?;

        Ok(Embedding { id, coords })
    }

    pub fn create(&mut self, paper: &Paper) -> Result<(), StorageError> {
        match self.conn.execute(
            "INSERT INTO papers 
             (id, key, author, year, title, notes, content, bibtex)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                paper.id.to_string(),
                paper.key,
                paper.author,
                paper.year,
                paper.title,
                paper.notes,
                paper.content,
                paper.bibtex,
            ],
        ) {
            Ok(_) => Ok(()),
            Err(rusqlite::Error::SqliteFailure(err, _))
                if err.code == rusqlite::ErrorCode::ConstraintViolation =>
            {
                Err(StorageError::DuplicateKey(paper.key.clone()))
            }
            Err(e) => Err(StorageError::Database(e)),
        }
    }

    pub fn save_embedding(
        &mut self,
        paper_id: u128,
        embedding: &[f32],
    ) -> Result<(), StorageError> {
        let embedding_bytes = bincode::serialize(embedding)
            .map_err(|e| StorageError::Serialization(e.to_string()))?;

        let dimensions = embedding.len() as i64;
        let paper_id_str = paper_id.to_string();

        self.conn.execute(
            "INSERT OR REPLACE INTO embeddings 
             (paper_id, embedding, dimensions, updated_at)
             VALUES (?1, ?2, ?3, CURRENT_TIMESTAMP)",
            params![paper_id_str, embedding_bytes, dimensions],
        )?;

        Ok(())
    }

    pub fn load_all_embeddings(&self) -> Result<Vec<Embedding>, StorageError> {
        let mut stmt = self
            .conn
            .prepare("SELECT paper_id, embedding FROM embeddings")?;

        let embedding_iter = stmt.query_map([], |row| {
            Self::row_to_embedding(row)
                .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))
        })?;

        embedding_iter
            .collect::<Result<Vec<_>, _>>()
            .map_err(StorageError::from)
    }

    pub fn get_by_id(&self, id: u128) -> Result<Option<Paper>, StorageError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, key, author, year, title, notes, content, bibtex
             FROM papers WHERE id = ?1",
        )?;

        let mut rows = stmt.query_map([id.to_string()], |row| {
            Self::row_to_paper(row)
                .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))
        })?;

        match rows.next() {
            Some(row) => Ok(Some(row?)),
            None => Ok(None),
        }
    }

    pub fn get_by_ids(&self, ids: &[u128]) -> Result<Vec<Paper>, StorageError> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }

        let id_strings: Vec<String> = ids.iter().map(|&id| id.to_string()).collect();
        let placeholders = (1..=id_strings.len())
            .map(|i| format!("?{}", i))
            .collect::<Vec<_>>()
            .join(", ");

        let sql = format!(
            "SELECT id, key, author, year, title, notes, content, bibtex
             FROM papers WHERE id IN ({})",
            placeholders
        );

        let mut stmt = self.conn.prepare(&sql)?;
        let params: Vec<&dyn rusqlite::ToSql> = id_strings
            .iter()
            .map(|s| s as &dyn rusqlite::ToSql)
            .collect();

        let rows = stmt.query_map(&params[..], |row| {
            Self::row_to_paper(row)
                .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))
        })?;

        let mut papers = Vec::new();
        for row in rows {
            papers.push(row?);
        }

        Ok(papers)
    }

    pub fn update(&mut self, paper: &Paper) -> Result<(), StorageError> {
        let affected = self.conn.execute(
            "UPDATE papers SET 
             key = ?2, author = ?3, year = ?4, title = ?5, notes = ?6,
             content = ?7, bibtex = ?8, updated_at = CURRENT_TIMESTAMP
             WHERE id = ?1",
            params![
                paper.id.to_string(),
                paper.key,
                paper.author,
                paper.year,
                paper.title,
                paper.notes,
                paper.content,
                paper.bibtex,
            ],
        )?;

        if affected == 0 {
            return Err(StorageError::PaperNotFound(paper.id));
        }

        Ok(())
    }

    pub fn touch(&mut self, id: u128) -> Result<(), StorageError> {
        let affected = self.conn.execute(
            "UPDATE papers SET updated_at = CURRENT_TIMESTAMP WHERE id = ?1",
            [id.to_string()],
        )?;

        if affected == 0 {
            return Err(StorageError::PaperNotFound(id));
        }

        Ok(())
    }

    pub fn delete(&mut self, id: u128) -> Result<(), StorageError> {
        let affected = self
            .conn
            .execute("DELETE FROM papers WHERE id = ?1", [id.to_string()])?;

        self.conn.execute(
            "DELETE FROM embeddings WHERE paper_id = ?1",
            [id.to_string()],
        )?;

        if affected == 0 {
            return Err(StorageError::PaperNotFound(id));
        }

        Ok(())
    }

    pub fn list_all(&self, limit: Option<usize>) -> Result<Vec<Paper>, StorageError> {
        let sql = if let Some(limit) = limit {
            format!(
                "SELECT id, key, author, year, title, notes, content, bibtex 
                 FROM papers 
                 ORDER BY updated_at DESC, title ASC 
                 LIMIT {}",
                limit
            )
        } else {
            "SELECT id, key, author, year, title, notes, content, bibtex 
             FROM papers 
             ORDER BY updated_at DESC, title ASC"
                .to_string()
        };

        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map([], |row| {
            Self::row_to_paper(row)
                .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))
        })?;

        let mut papers = Vec::new();
        for row in rows {
            papers.push(row?);
        }

        Ok(papers)
    }

    pub fn exists_by_key(&self, key: &str) -> Result<bool, StorageError> {
        let count: i64 =
            self.conn
                .query_row("SELECT COUNT(*) FROM papers WHERE key = ?1", [key], |row| {
                    row.get(0)
                })?;

        Ok(count > 0)
    }

    pub fn count(&self) -> Result<usize, StorageError> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM papers", [], |row| row.get(0))?;

        Ok(count as usize)
    }

    pub fn clear(&mut self) -> Result<(), StorageError> {
        self.conn.execute("DELETE FROM papers", [])?;
        Ok(())
    }
}
