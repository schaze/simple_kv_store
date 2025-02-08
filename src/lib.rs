mod inmemory;
mod kubernetes;
mod sqlite;
pub use inmemory::*;
pub use kubernetes::*;
pub use sqlite::*;

/// Provides a multi backend simple Key Value store
#[derive(Clone)]
pub enum KeyValueStore {
    InMemory(InMemoryStore),
    Kubernetes(KubernetesStore),
    SQLite(SQLiteStore),
}

impl KeyValueStore {
    pub async fn get(&self, key: &str) -> Option<String> {
        match self {
            KeyValueStore::InMemory(store) => store.get(key).await,
            KeyValueStore::Kubernetes(store) => store.get(key).await,
            KeyValueStore::SQLite(store) => store.get(key).await,
        }
    }

    pub async fn set(&self, key: &str, value: &str) -> Result<(), Box<dyn std::error::Error>> {
        match self {
            KeyValueStore::InMemory(store) => store.set(key, value).await,
            KeyValueStore::Kubernetes(store) => store.set(key, value).await,
            KeyValueStore::SQLite(store) => store.set(key, value).await,
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
