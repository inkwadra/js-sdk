//! Async Auth Store
//!
//! AsyncAuthStore is a helper auth store implementation that can be used with
//! any external async persistent layer (key-value db, local file, etc.).
//!
//! # Example
//!
//! ```rust,ignore
//! use pocketbase_sdk::stores::{AsyncAuthStore, AsyncSaveFunc, AsyncClearFunc};
//!
//! // Create an async store with file-based persistence
//! let store = AsyncAuthStore::new(
//!     Box::new(|serialized| {
//!         Box::pin(async move {
//!             tokio::fs::write("pb_auth.json", serialized).await?;
//!             Ok(())
//!         })
//!     }),
//!     Some(Box::new(|| {
//!         Box::pin(async move {
//!             tokio::fs::remove_file("pb_auth.json").await.ok();
//!             Ok(())
//!         })
//!     })),
//!     None,
//! );
//! ```

use crate::stores::base_auth_store::{AuthRecord, OnStoreChangeFunc};
use crate::tools::dtos::RecordModel;
use crate::tools::jwt::{get_token_payload, is_token_expired};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, RwLock};
use tokio::sync::Mutex;

/// Type alias for the async save function.
pub type AsyncSaveFunc = Box<
    dyn Fn(String) -> Pin<Box<dyn Future<Output = Result<(), AsyncAuthStoreError>> + Send>>
        + Send
        + Sync,
>;

/// Type alias for the async clear function.
pub type AsyncClearFunc = Box<
    dyn Fn() -> Pin<Box<dyn Future<Output = Result<(), AsyncAuthStoreError>> + Send>> + Send + Sync,
>;

/// Type alias for async callback functions in the queue.
type QueueFunc = Box<dyn FnOnce() -> Pin<Box<dyn Future<Output = ()> + Send>> + Send>;

/// Error type for AsyncAuthStore operations.
#[derive(Debug, Clone, thiserror::Error)]
pub enum AsyncAuthStoreError {
    #[error("IO error: {0}")]
    Io(String),
    #[error("Serialization error: {0}")]
    Serialization(String),
    #[error("Other error: {0}")]
    Other(String),
}

impl From<std::io::Error> for AsyncAuthStoreError {
    fn from(err: std::io::Error) -> Self {
        AsyncAuthStoreError::Io(err.to_string())
    }
}

impl From<serde_json::Error> for AsyncAuthStoreError {
    fn from(err: serde_json::Error) -> Self {
        AsyncAuthStoreError::Serialization(err.to_string())
    }
}

/// Serializable auth state for persistence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializedAuthState {
    /// The authentication token.
    pub token: String,
    /// The authenticated record/user.
    pub record: Option<RecordModel>,
}

/// AsyncAuthStore is a helper auth store implementation that can be used with
/// any external async persistent layer (key-value db, local file, etc.).
pub struct AsyncAuthStore {
    /// The current token.
    token: Arc<RwLock<String>>,
    /// The current authenticated record.
    record: Arc<RwLock<AuthRecord>>,
    /// On-change callbacks.
    on_change_callbacks: Arc<RwLock<Vec<OnStoreChangeFunc>>>,
    /// The async save function.
    save_func: Arc<AsyncSaveFunc>,
    /// The optional async clear function.
    clear_func: Arc<Option<AsyncClearFunc>>,
    /// Queue for async operations.
    queue: Arc<Mutex<VecDeque<QueueFunc>>>,
    /// Whether the queue is currently being processed.
    processing: Arc<Mutex<bool>>,
}

impl AsyncAuthStore {
    /// Creates a new AsyncAuthStore.
    ///
    /// # Arguments
    ///
    /// * `save_func` - The async function called every time the auth store state needs to be persisted.
    /// * `clear_func` - An optional async function called when the auth store needs to be cleared.
    ///                  If not provided, `save_func` with empty data will be used.
    /// * `initial` - Optional initial data to load into the store.
    pub fn new(
        save_func: AsyncSaveFunc,
        clear_func: Option<AsyncClearFunc>,
        initial: Option<String>,
    ) -> Arc<Self> {
        let store = Arc::new(Self {
            token: Arc::new(RwLock::new(String::new())),
            record: Arc::new(RwLock::new(None)),
            on_change_callbacks: Arc::new(RwLock::new(Vec::new())),
            save_func: Arc::new(save_func),
            clear_func: Arc::new(clear_func),
            queue: Arc::new(Mutex::new(VecDeque::new())),
            processing: Arc::new(Mutex::new(false)),
        });

        // Load initial data if provided
        if let Some(initial_data) = initial {
            let store_clone = Arc::clone(&store);
            tokio::spawn(async move {
                store_clone.load_initial(&initial_data).await;
            });
        }

        store
    }

    /// Creates a new AsyncAuthStore and waits for initial data to be loaded.
    ///
    /// This is an async version of `new` that blocks until initial data is loaded.
    pub async fn new_async(
        save_func: AsyncSaveFunc,
        clear_func: Option<AsyncClearFunc>,
        initial: Option<String>,
    ) -> Arc<Self> {
        let store = Arc::new(Self {
            token: Arc::new(RwLock::new(String::new())),
            record: Arc::new(RwLock::new(None)),
            on_change_callbacks: Arc::new(RwLock::new(Vec::new())),
            save_func: Arc::new(save_func),
            clear_func: Arc::new(clear_func),
            queue: Arc::new(Mutex::new(VecDeque::new())),
            processing: Arc::new(Mutex::new(false)),
        });

        // Load initial data if provided
        if let Some(initial_data) = initial {
            store.load_initial(&initial_data).await;
        }

        store
    }

    /// Retrieves the stored token.
    pub fn token(&self) -> String {
        self.token.read().unwrap().clone()
    }

    /// Retrieves the stored record.
    pub fn record(&self) -> AuthRecord {
        self.record.read().unwrap().clone()
    }

    /// Loosely checks if the store has a valid token.
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

    /// Saves the provided token and record data in the auth store.
    pub fn save(self: &Arc<Self>, token: &str, record: AuthRecord) {
        // Update in-memory state
        *self.token.write().unwrap() = token.to_string();
        *self.record.write().unwrap() = record.clone();
        self.trigger_change();

        // Serialize and queue async save
        let serialized = match serde_json::to_string(&SerializedAuthState {
            token: token.to_string(),
            record,
        }) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("AsyncAuthStore: failed to stringify the new state: {}", e);
                return;
            }
        };

        let save_func = Arc::clone(&self.save_func);
        let this = Arc::clone(self);

        this.enqueue(Box::new(move || {
            Box::pin(async move {
                if let Err(e) = save_func(serialized).await {
                    eprintln!("AsyncAuthStore: save error: {}", e);
                }
            })
        }));
    }

    /// Removes the stored token and record data from the auth store.
    pub fn clear(self: &Arc<Self>) {
        // Update in-memory state
        *self.token.write().unwrap() = String::new();
        *self.record.write().unwrap() = None;
        self.trigger_change();

        let clear_func = Arc::clone(&self.clear_func);
        let save_func = Arc::clone(&self.save_func);
        let this = Arc::clone(self);

        this.enqueue(Box::new(move || {
            Box::pin(async move {
                let result = if let Some(ref clear_fn) = *clear_func {
                    clear_fn().await
                } else {
                    save_func(String::new()).await
                };

                if let Err(e) = result {
                    eprintln!("AsyncAuthStore: clear error: {}", e);
                }
            })
        }));
    }

    /// Register a callback function that will be called on store change.
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

    /// Loads initial data into the store.
    async fn load_initial(&self, payload: &str) {
        if payload.is_empty() {
            return;
        }

        match serde_json::from_str::<SerializedAuthState>(payload) {
            Ok(state) => {
                *self.token.write().unwrap() = state.token;
                *self.record.write().unwrap() = state.record;
                self.trigger_change();
            }
            Err(_) => {
                // Try legacy format with "model" instead of "record"
                #[derive(Deserialize)]
                struct LegacyState {
                    token: String,
                    model: Option<RecordModel>,
                }

                if let Ok(legacy) = serde_json::from_str::<LegacyState>(payload) {
                    *self.token.write().unwrap() = legacy.token;
                    *self.record.write().unwrap() = legacy.model;
                    self.trigger_change();
                }
            }
        }
    }

    /// Enqueues an async callback for sequential execution.
    fn enqueue(self: &Arc<Self>, callback: QueueFunc) {
        let this = Arc::clone(self);

        tokio::spawn(async move {
            {
                let mut queue = this.queue.lock().await;
                queue.push_back(callback);
            }

            this.dequeue().await;
        });
    }

    /// Processes the queue sequentially.
    async fn dequeue(self: &Arc<Self>) {
        // Check if already processing
        {
            let mut processing = self.processing.lock().await;
            if *processing {
                return;
            }
            *processing = true;
        }

        loop {
            let callback = {
                let mut queue = self.queue.lock().await;
                queue.pop_front()
            };

            match callback {
                Some(cb) => {
                    cb().await;
                }
                None => {
                    break;
                }
            }
        }

        *self.processing.lock().await = false;
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

impl std::fmt::Debug for AsyncAuthStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AsyncAuthStore")
            .field("token", &self.token())
            .field("record", &self.record())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[tokio::test]
    async fn test_async_auth_store_new() {
        let save_count = Arc::new(AtomicUsize::new(0));
        let save_count_clone = Arc::clone(&save_count);

        let store = AsyncAuthStore::new_async(
            Box::new(move |_| {
                let count = Arc::clone(&save_count_clone);
                Box::pin(async move {
                    count.fetch_add(1, Ordering::SeqCst);
                    Ok(())
                })
            }),
            None,
            None,
        )
        .await;

        assert!(store.token().is_empty());
        assert!(store.record().is_none());
    }

    #[tokio::test]
    async fn test_async_auth_store_with_initial() {
        let initial = serde_json::to_string(&SerializedAuthState {
            token: "test_token".to_string(),
            record: Some(RecordModel {
                id: "test123".to_string(),
                collection_id: "users".to_string(),
                collection_name: "users".to_string(),
                ..Default::default()
            }),
        })
        .unwrap();

        let store = AsyncAuthStore::new_async(
            Box::new(|_| Box::pin(async { Ok(()) })),
            None,
            Some(initial),
        )
        .await;

        assert_eq!(store.token(), "test_token");
        assert!(store.record().is_some());
        assert_eq!(store.record().unwrap().id, "test123");
    }

    #[tokio::test]
    async fn test_async_auth_store_save_and_clear() {
        let saved_data = Arc::new(RwLock::new(String::new()));
        let saved_data_clone = Arc::clone(&saved_data);

        let store = AsyncAuthStore::new_async(
            Box::new(move |data| {
                let saved = Arc::clone(&saved_data_clone);
                Box::pin(async move {
                    *saved.write().unwrap() = data;
                    Ok(())
                })
            }),
            None,
            None,
        )
        .await;

        let record = RecordModel {
            id: "test123".to_string(),
            collection_id: "users".to_string(),
            collection_name: "users".to_string(),
            ..Default::default()
        };

        store.save("my_token", Some(record));

        // Wait for async operation to complete
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        assert_eq!(store.token(), "my_token");
        assert!(store.record().is_some());

        // Verify saved data
        let saved = saved_data.read().unwrap().clone();
        assert!(saved.contains("my_token"));
        assert!(saved.contains("test123"));

        // Clear
        store.clear();

        // Wait for async operation to complete
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        assert!(store.token().is_empty());
        assert!(store.record().is_none());
    }
}
