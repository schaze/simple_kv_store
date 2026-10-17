use rusqlite::{params, Connection};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct SQLiteStore {
    conn: Arc<Mutex<Connection>>, // Clonable reference
}

impl SQLiteStore {
    /// Opens (or creates) the database at `db_path` and ensures the store table exists.
    pub async fn new(db_path: &str) -> Result<Self, rusqlite::Error> {
        let conn = Connection::open(db_path)?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS kv_store (key TEXT PRIMARY KEY, value TEXT)",
            [],
        )?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn unopenable_path_returns_error() {
        let path = std::env::temp_dir().join("simple_kv_store_missing_dir/sub/store.db");
        assert!(SQLiteStore::new(path.to_str().unwrap()).await.is_err());
    }

    #[tokio::test]
    async fn opened_store_reads_and_writes() {
        let store = SQLiteStore::new(":memory:").await.unwrap();
        store.set("k", "v").await.unwrap();
        assert_eq!(store.get("k").await.as_deref(), Some("v"));
    }
}
