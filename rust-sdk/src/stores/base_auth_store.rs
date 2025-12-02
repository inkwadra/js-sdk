//! Base Auth Store

use crate::tools::dtos::RecordModel;
use crate::tools::jwt::{get_token_payload, is_token_expired};
use std::sync::{Arc, RwLock};

/// Type alias for the auth record.
pub type AuthRecord = Option<RecordModel>;

/// Callback function type for auth store changes.
pub type OnStoreChangeFunc = Box<dyn Fn(&str, &AuthRecord) + Send + Sync>;

/// Base AuthStore that stores the auth state in memory.
///
/// This is the base implementation that other auth stores can extend.
#[derive(Default)]
pub struct BaseAuthStore {
    token: Arc<RwLock<String>>,
    record: Arc<RwLock<AuthRecord>>,
    on_change_callbacks: Arc<RwLock<Vec<OnStoreChangeFunc>>>,
}

impl BaseAuthStore {
    /// Creates a new BaseAuthStore.
    pub fn new() -> Self {
        Self {
            token: Arc::new(RwLock::new(String::new())),
            record: Arc::new(RwLock::new(None)),
            on_change_callbacks: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Retrieves the stored token (if any).
    pub fn token(&self) -> String {
        self.token.read().unwrap().clone()
    }

    /// Retrieves the stored record (if any).
    pub fn record(&self) -> AuthRecord {
        self.record.read().unwrap().clone()
    }

    /// Loosely checks if the store has a valid token (existing and unexpired exp claim).
    pub fn is_valid(&self) -> bool {
        !is_token_expired(&self.token(), 0)
    }

    /// Loosely checks whether the currently loaded store state is for superuser.
    pub fn is_superuser(&self) -> bool {
        let token = self.token();
        let payload = get_token_payload(&token);

        if payload.token_type.as_deref() != Some("auth") {
            return false;
        }

        if let Some(ref record) = *self.record.read().unwrap() {
            if record.collection_name == "_superusers" {
                return true;
            }
        }

        // Fallback check using collection ID
        payload.collection_id.as_deref() == Some("pbc_3142635823")
    }

    /// Saves the provided new token and record data in the auth store.
    pub fn save(&self, token: &str, record: AuthRecord) {
        *self.token.write().unwrap() = token.to_string();
        *self.record.write().unwrap() = record;
        self.trigger_change();
    }

    /// Removes the stored token and record data from the auth store.
    pub fn clear(&self) {
        *self.token.write().unwrap() = String::new();
        *self.record.write().unwrap() = None;
        self.trigger_change();
    }

    /// Register a callback function that will be called on store change.
    /// Returns a removal function that can be called to unsubscribe.
    pub fn on_change(&self, callback: OnStoreChangeFunc) -> impl FnOnce() {
        let callbacks = Arc::clone(&self.on_change_callbacks);
        let index = {
            let mut cbs = callbacks.write().unwrap();
            cbs.push(callback);
            cbs.len() - 1
        };

        move || {
            let mut cbs = callbacks.write().unwrap();
            if index < cbs.len() {
                let _ = cbs.remove(index);
            }
        }
    }

    /// Triggers the change callbacks.
    fn trigger_change(&self) {
        let token = self.token();
        let record = self.record();
        let callbacks = self.on_change_callbacks.read().unwrap();
        for callback in callbacks.iter() {
            callback(&token, &record);
        }
    }
}

impl Clone for BaseAuthStore {
    fn clone(&self) -> Self {
        Self {
            token: Arc::new(RwLock::new(self.token())),
            record: Arc::new(RwLock::new(self.record())),
            on_change_callbacks: Arc::new(RwLock::new(Vec::new())),
        }
    }
}

impl std::fmt::Debug for BaseAuthStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BaseAuthStore")
            .field("token", &self.token())
            .field("record", &self.record())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_save_and_clear() {
        let store = BaseAuthStore::new();
        
        assert!(store.token().is_empty());
        assert!(store.record().is_none());

        let record = RecordModel {
            id: "test123".to_string(),
            collection_id: "users".to_string(),
            collection_name: "users".to_string(),
            ..Default::default()
        };

        store.save("token123", Some(record.clone()));
        
        assert_eq!(store.token(), "token123");
        assert!(store.record().is_some());
        assert_eq!(store.record().unwrap().id, "test123");

        store.clear();
        
        assert!(store.token().is_empty());
        assert!(store.record().is_none());
    }
}
