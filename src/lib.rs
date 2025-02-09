mod inmemory;
mod kubernetes;
mod sqlite;
pub use inmemory::*;
pub use kubernetes::*;
use serde::{de::DeserializeOwned, Serialize};
pub use sqlite::*;

/// Provides a multi backend simple Key Value store
#[derive(Clone)]
pub enum KeyValueStore {
    InMemory(InMemoryStore),
    Kubernetes(KubernetesStore),
    SQLite(SQLiteStore),
}

impl KeyValueStore {
    pub async fn get<T: DeserializeOwned>(&self, key: &str) -> Option<T> {
        let value_str = match self {
            KeyValueStore::InMemory(store) => store.get(key).await,
            KeyValueStore::Kubernetes(store) => store.get(key).await,
            KeyValueStore::SQLite(store) => store.get(key).await,
        }?;
        serde_json::from_str(&value_str).ok()
    }

    pub async fn set<T: Serialize>(
        &self,
        key: &str,
        value: &T,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let value_str = serde_json::to_string(value)?;
        match self {
            KeyValueStore::InMemory(store) => store.set(key, &value_str).await,
            KeyValueStore::Kubernetes(store) => store.set(key, &value_str).await,
            KeyValueStore::SQLite(store) => store.set(key, &value_str).await,
        }
    }

    pub async fn delete(&self, key: &str) -> Result<(), Box<dyn std::error::Error>> {
        match self {
            KeyValueStore::InMemory(store) => store.delete(key).await,
            KeyValueStore::Kubernetes(store) => store.delete(key).await,
            KeyValueStore::SQLite(store) => store.delete(key).await,
        }
    }
}

/// Normalizes a key to be compatible with Kubernetes ConfigMap and Secret keys.
///
/// Kubernetes requires keys to consist of **alphanumeric characters (`A-Z`, `a-z`, `0-9`), dashes (`-`), underscores (`_`), and dots (`.`)**.
/// Any other characters (such as `/`, `:`) will be replaced with underscores (`_`).
///
/// # Examples
///
/// ```
/// let key = "device/switch/state";
/// assert_eq!(normalize_key(key), "device_switch_state");
///
/// let key = "config:mode/type";
/// assert_eq!(normalize_key(key), "config_mode_type");
///
/// let key = "user@domain.com";
/// assert_eq!(normalize_key(key), "user_domain.com");
/// ```
pub fn normalize_key(key: &str) -> String {
    key.chars()
        .map(|c| match c {
            '/' | ':' | '@' => '_', // Replace invalid separators with underscore
            _ if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' => c, // Keep valid characters
            _ => '_', // Replace any other invalid characters with '_'
        })
        .collect()
}
