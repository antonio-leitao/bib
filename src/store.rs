use crate::base::Paper;
use rusqlite::{params, Connection, Row};
use std::path::Path;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum StoreError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("Paper not found with id: {0}")]
    NotFound(u128),
    #[error("Paper with key already exists: {0}")]
    KeyExists(String),
}

pub struct PaperStore {
    conn: Connection,
}

impl PaperStore {
    /// Create a new store with a file-based database
    pub fn new<P: AsRef<Path>>(db_path: P) -> Result<Self, StoreError> {
        let conn = Connection::open(db_path)?;
        let store = PaperStore { conn };
        store.init_schema()?;
        Ok(store)
    }

    /// Initialize the database schema
    fn init_schema(&self) -> Result<(), StoreError> {
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

        Ok(())
    }

    /// Convert a database row to a Paper struct
    fn row_to_paper(row: &Row) -> Result<Paper, StoreError> {
        let id_str: String = row.get("id")?;
        let id = id_str.parse::<u128>().map_err(|e| {
            StoreError::Database(rusqlite::Error::FromSqlConversionFailure(
                0,
                rusqlite::types::Type::Text,
                Box::new(e),
            ))
        })?;

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

    /// Insert a new paper into the database
    pub fn create(&mut self, paper: &Paper) -> Result<(), StoreError> {
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
                Err(StoreError::KeyExists(paper.key.clone()))
            }
            Err(e) => Err(StoreError::Database(e)),
        }
    }

    /// Get a paper by its ID
    pub fn get_by_id(&self, id: u128) -> Result<Option<Paper>, StoreError> {
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

    /// Update an existing paper
    pub fn update(&mut self, paper: &Paper) -> Result<(), StoreError> {
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
            return Err(StoreError::NotFound(paper.id));
        }

        Ok(())
    }

    /// Delete a paper by ID
    pub fn delete(&mut self, id: u128) -> Result<(), StoreError> {
        let affected = self
            .conn
            .execute("DELETE FROM papers WHERE id = ?1", [id.to_string()])?;

        if affected == 0 {
            return Err(StoreError::NotFound(id));
        }

        Ok(())
    }

    /// List all papers (with optional limit)
    /// TODO: order by last updated
    pub fn list_all(&self, limit: Option<usize>) -> Result<Vec<Paper>, StoreError> {
        let sql = if let Some(limit) = limit {
            format!(
                "SELECT id, key, author, year, title, notes, content, bibtex 
                     FROM papers 
                     ORDER BY year DESC, title ASC 
                     LIMIT {}",
                limit
            )
        } else {
            "SELECT id, key, author, year, title, notes, content, bibtex 
             FROM papers 
             ORDER BY year DESC, title ASC"
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

    /// Check if a paper with the given key exists
    pub fn exists_by_key(&self, key: &str) -> Result<bool, StoreError> {
        let count: i64 =
            self.conn
                .query_row("SELECT COUNT(*) FROM papers WHERE key = ?1", [key], |row| {
                    row.get(0)
                })?;

        Ok(count > 0)
    }

    /// Get the total count of papers
    pub fn count(&self) -> Result<usize, StoreError> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM papers", [], |row| row.get(0))?;
        Ok(count as usize)
    }

    /// Clear all papers from the database
    pub fn clear(&mut self) -> Result<(), StoreError> {
        self.conn.execute("DELETE FROM papers", [])?;
        Ok(())
    }
}
