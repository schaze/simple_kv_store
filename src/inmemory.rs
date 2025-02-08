use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct InMemoryStore {
    store: Arc<RwLock<HashMap<String, String>>>,
}

impl InMemoryStore {
    pub fn new() -> Self {
        Self {
            store: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn get(&self, key: &str) -> Option<String> {
        let store = self.store.read().await;
        store.get(key).cloned()
    }

    pub async fn set(&self, key: &str, value: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut store = self.store.write().await;
        store.insert(key.to_string(), value.to_string());
        Ok(())
    }

    pub async fn delete(&self, key: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut store = self.store.write().await;
        store.remove(key);
        Ok(())
    }
}

impl Default for InMemoryStore {
    fn default() -> Self {
        Self::new()
    }
}
