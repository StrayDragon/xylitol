use std::collections::HashMap;
use std::sync::Mutex;

use async_trait::async_trait;

use super::error::XyError;
use super::types::XyContent;

#[async_trait]
pub(crate) trait XySession: Send + Sync {
    async fn load(&self, session_id: &str) -> Result<Vec<XyContent>, XyError>;
    async fn save(&self, session_id: &str, messages: &[XyContent]) -> Result<(), XyError>;
    async fn exists(&self, session_id: &str) -> bool;
    async fn create(&self, session_id: &str) -> Result<(), XyError>;
    async fn list_session_ids(&self) -> Result<Vec<String>, XyError>;
}

pub(crate) struct InMemorySession {
    store: Mutex<HashMap<String, Vec<XyContent>>>,
}

impl InMemorySession {
    pub(crate) fn new() -> Self {
        Self {
            store: Mutex::new(HashMap::new()),
        }
    }
}

#[async_trait]
impl XySession for InMemorySession {
    async fn load(&self, session_id: &str) -> Result<Vec<XyContent>, XyError> {
        let store = self.store.lock().unwrap();
        Ok(store.get(session_id).cloned().unwrap_or_default())
    }

    async fn save(&self, session_id: &str, messages: &[XyContent]) -> Result<(), XyError> {
        let mut store = self.store.lock().unwrap();
        store.insert(session_id.to_string(), messages.to_vec());
        Ok(())
    }

    async fn exists(&self, session_id: &str) -> bool {
        let store = self.store.lock().unwrap();
        store.contains_key(session_id)
    }

    async fn create(&self, session_id: &str) -> Result<(), XyError> {
        let mut store = self.store.lock().unwrap();
        store.entry(session_id.to_string()).or_default();
        Ok(())
    }

    async fn list_session_ids(&self) -> Result<Vec<String>, XyError> {
        let store = self.store.lock().unwrap();
        Ok(store.keys().cloned().collect())
    }
}
