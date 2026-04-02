use aeonic_core::{error::Result, traits::StateStore};
use async_trait::async_trait;
use dashmap::DashMap;
use std::sync::Arc;

/// A simple in-memory key-value store.
/// Implements the core `StateStore` trait.
/// For production, swap with a Redis or database-backed implementation.
#[derive(Clone, Default)]
pub struct InMemoryStore {
    data: Arc<DashMap<String, serde_json::Value>>,
}

impl InMemoryStore {
    pub fn new() -> Self {
        Self {
            data: Arc::new(DashMap::new()),
        }
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
}

#[async_trait]
impl StateStore for InMemoryStore {
    async fn set(&self, key: &str, value: serde_json::Value) -> Result<()> {
        self.data.insert(key.to_string(), value);
        Ok(())
    }

    async fn get(&self, key: &str) -> Result<Option<serde_json::Value>> {
        Ok(self.data.get(key).map(|v| v.clone()))
    }

    async fn delete(&self, key: &str) -> Result<()> {
        self.data.remove(key);
        Ok(())
    }

    async fn list(&self, prefix: Option<&str>) -> Result<Vec<String>> {
        let keys: Vec<String> = self
            .data
            .iter()
            .map(|e| e.key().clone())
            .filter(|k| match prefix {
                Some(p) => k.starts_with(p),
                None => true,
            })
            .collect();
        Ok(keys)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn set_get_delete() {
        let store = InMemoryStore::new();

        store.set("key1", serde_json::json!("hello")).await.unwrap();
        let val = store.get("key1").await.unwrap();
        assert_eq!(val, Some(serde_json::json!("hello")));

        store.delete("key1").await.unwrap();
        let val = store.get("key1").await.unwrap();
        assert_eq!(val, None);
    }

    #[tokio::test]
    async fn list_with_prefix() {
        let store = InMemoryStore::new();
        store.set("session:abc:msg1", serde_json::json!(1)).await.unwrap();
        store.set("session:abc:msg2", serde_json::json!(2)).await.unwrap();
        store.set("other:xyz",        serde_json::json!(3)).await.unwrap();

        let keys = store.list(Some("session:abc")).await.unwrap();
        assert_eq!(keys.len(), 2);
        assert!(keys.iter().all(|k| k.starts_with("session:abc")));
    }
}
