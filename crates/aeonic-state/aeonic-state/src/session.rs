use aeonic_core::{error::Result, types::Message};
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

/// A conversation session — groups messages under a session ID.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub messages: Vec<Message>,
    pub metadata: serde_json::Value,
}

impl Session {
    pub fn new() -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            messages: Vec::new(),
            metadata: serde_json::Value::Null,
        }
    }

    pub fn push(&mut self, message: Message) {
        self.updated_at = Utc::now();
        self.messages.push(message);
    }

    pub fn message_count(&self) -> usize {
        self.messages.len()
    }
}

impl Default for Session {
    fn default() -> Self {
        Self::new()
    }
}

/// In-memory session store.
/// Stores full conversation history per session ID.
#[derive(Clone, Default)]
pub struct SessionStore {
    sessions: Arc<DashMap<Uuid, Session>>,
}

impl SessionStore {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(DashMap::new()),
        }
    }

    /// Create a new session and return its ID.
    pub fn create(&self) -> Uuid {
        let session = Session::new();
        let id = session.id;
        self.sessions.insert(id, session);
        id
    }

    /// Get a session by ID.
    pub fn get(&self, id: &Uuid) -> Option<Session> {
        self.sessions.get(id).map(|s| s.clone())
    }

    /// Append a message to a session.
    pub fn push_message(&self, session_id: &Uuid, message: Message) -> Result<()> {
        if let Some(mut session) = self.sessions.get_mut(session_id) {
            session.push(message);
        }
        Ok(())
    }

    /// Delete a session.
    pub fn delete(&self, id: &Uuid) {
        self.sessions.remove(id);
    }

    /// Number of active sessions.
    pub fn len(&self) -> usize {
        self.sessions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.sessions.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aeonic_core::types::Message;

    #[test]
    fn create_and_push_messages() {
        let store = SessionStore::new();
        let id = store.create();

        store.push_message(&id, Message::user("Hello")).unwrap();
        store.push_message(&id, Message::assistant("Hi!")).unwrap();

        let session = store.get(&id).unwrap();
        assert_eq!(session.message_count(), 2);
    }
}
