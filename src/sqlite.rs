use rusqlite::{params, Connection};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct SQLiteStore {
    conn: Arc<Mutex<Connection>>, // Clonable reference
}

impl SQLiteStore {
    pub async fn new(db_path: &str) -> Self {
        let conn = Connection::open(db_path).expect("Failed to open database");
        conn.execute(
            "CREATE TABLE IF NOT EXISTS kv_store (key TEXT PRIMARY KEY, value TEXT)",
            [],
        )
        .expect("Failed to create table");

        Self {
            conn: Arc::new(Mutex::new(conn)),
        }
    }

    pub async fn get(&self, key: &str) -> Option<String> {
        let conn = self.conn.lock().await;
        let mut stmt = conn
            .prepare("SELECT value FROM kv_store WHERE key = ?")
            .ok()?;

        stmt.query_row(params![key], |row| row.get(0)).ok()
    }

    pub async fn set(&self, key: &str, value: &str) -> Result<(), Box<dyn std::error::Error>> {
        let conn = self.conn.lock().await;
        conn.execute(
            "INSERT INTO kv_store (key, value) VALUES (?, ?) ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )?;
        Ok(())
    }

    pub async fn delete(&self, key: &str) -> Result<(), Box<dyn std::error::Error>> {
        let conn = self.conn.lock().await;
        conn.execute("DELETE FROM kv_store WHERE key = ?", params![key])?;
        Ok(())
    }
}
