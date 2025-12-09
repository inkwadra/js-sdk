//! Local Auth Store
//!
//! A file-based persistent auth store implementation for Rust applications.
//! Unlike the JavaScript SDK which uses browser localStorage, this implementation
//! uses the filesystem for persistence, making it suitable for desktop and server applications.

use crate::stores::base_auth_store::{AuthRecord, BaseAuthStore};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

/// Serializable auth data for persistence.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct AuthData {
    token: String,
    record: Option<crate::tools::dtos::RecordModel>,
}

/// The default token store for Rust applications with auto fallback
/// to runtime/memory if file storage is unavailable.
///
/// This is the Rust equivalent of the JavaScript SDK's `LocalAuthStore`,
/// but uses file-based storage instead of browser localStorage.
///
/// # Example
///
/// ```rust,ignore
/// use pocketbase_sdk::stores::LocalAuthStore;
///
/// // Create a store with default path (~/.pocketbase_auth)
/// let store = LocalAuthStore::new(None);
///
/// // Create a store with custom path
/// let store = LocalAuthStore::new(Some("./my_auth.json"));
/// ```
pub struct LocalAuthStore {
    /// Inner base auth store for in-memory state and callbacks.
    inner: BaseAuthStore,

    /// The storage key/path for persistence.
    storage_path: PathBuf,

    /// Fallback in-memory storage when file operations fail.
    storage_fallback: Arc<RwLock<Option<AuthData>>>,
}

impl LocalAuthStore {
    /// Creates a new LocalAuthStore.
    ///
    /// # Arguments
    ///
    /// * `storage_path` - Optional path for the auth file. If not provided,
    ///   defaults to `.pocketbase_auth` in the current directory.
    pub fn new(storage_path: Option<&str>) -> Self {
        let path = storage_path
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(".pocketbase_auth"));

        let store = Self {
            inner: BaseAuthStore::new(),
            storage_path: path,
            storage_fallback: Arc::new(RwLock::new(None)),
        };

        // Load initial state from storage
        store.load_from_storage();

        store
    }

    /// Creates a new LocalAuthStore with a path relative to the user's home directory.
    ///
    /// # Arguments
    ///
    /// * `filename` - The filename to use in the home directory.
    pub fn in_home_dir(filename: &str) -> Self {
        let path = dirs::home_dir()
            .map(|p| p.join(filename))
            .unwrap_or_else(|| PathBuf::from(filename));

        Self::new(Some(path.to_str().unwrap_or(filename)))
    }

    /// Retrieves the stored token (if any).
    pub fn token(&self) -> String {
        let data = self.storage_get();
        data.token
    }

    /// Retrieves the stored record (if any).
    pub fn record(&self) -> AuthRecord {
        let data = self.storage_get();
        data.record
    }

    /// Loosely checks if the store has a valid token.
    pub fn is_valid(&self) -> bool {
        self.inner.is_valid()
    }

    /// Loosely checks whether the currently loaded store state is for superuser.
    pub fn is_superuser(&self) -> bool {
        self.inner.is_superuser()
    }

    /// Saves the provided new token and record data in the auth store.
    pub fn save(&self, token: &str, record: AuthRecord) {
        let data = AuthData {
            token: token.to_string(),
            record: record.clone(),
        };

        self.storage_set(data);
        self.inner.save(token, record);
    }

    /// Removes the stored token and record data from the auth store.
    pub fn clear(&self) {
        self.storage_remove();
        self.inner.clear();
    }

    /// Register a callback function that will be called on store change.
    pub fn on_change(
        &self,
        callback: crate::stores::base_auth_store::OnStoreChangeFunc,
    ) -> impl FnOnce() {
        self.inner.on_change(callback)
    }

    /// Returns the storage path.
    pub fn storage_path(&self) -> &Path {
        &self.storage_path
    }

    // ---------------------------------------------------------------
    // Internal helpers:
    // ---------------------------------------------------------------

    /// Loads initial state from storage into the inner BaseAuthStore.
    fn load_from_storage(&self) {
        let data = self.storage_get();
        if !data.token.is_empty() || data.record.is_some() {
            self.inner.save(&data.token, data.record);
        }
    }

    /// Retrieves auth data from storage.
    fn storage_get(&self) -> AuthData {
        // Try to read from file
        if let Ok(contents) = fs::read_to_string(&self.storage_path) {
            if let Ok(data) = serde_json::from_str::<AuthData>(&contents) {
                return data;
            }
        }

        // Fallback to memory
        self.storage_fallback
            .read()
            .unwrap()
            .clone()
            .unwrap_or_default()
    }

    /// Stores auth data to storage.
    fn storage_set(&self, data: AuthData) {
        // Try to write to file
        if let Ok(json) = serde_json::to_string_pretty(&data) {
            if fs::write(&self.storage_path, json).is_ok() {
                return;
            }
        }

        // Fallback to memory
        *self.storage_fallback.write().unwrap() = Some(data);
    }

    /// Removes auth data from storage.
    fn storage_remove(&self) {
        // Remove file
        let _ = fs::remove_file(&self.storage_path);

        // Clear fallback
        *self.storage_fallback.write().unwrap() = None;
    }
}

impl Clone for LocalAuthStore {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            storage_path: self.storage_path.clone(),
            storage_fallback: Arc::new(RwLock::new(self.storage_fallback.read().unwrap().clone())),
        }
    }
}

impl std::fmt::Debug for LocalAuthStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LocalAuthStore")
            .field("storage_path", &self.storage_path)
            .field("token", &self.token())
            .field("record", &self.record())
            .finish()
    }
}

/// Optional feature: Watch for external changes to the auth file.
/// This is similar to the JavaScript SDK's storage event listener.
#[cfg(feature = "file-watcher")]
impl LocalAuthStore {
    /// Starts watching the storage file for external changes.
    ///
    /// This is useful when multiple processes might modify the auth file.
    pub fn watch_storage_changes(&self) -> notify::Result<notify::RecommendedWatcher> {
        use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};

        let storage_path = self.storage_path.clone();
        let inner = self.inner.clone();

        let mut watcher = RecommendedWatcher::new(
            move |res: Result<Event, notify::Error>| {
                if let Ok(event) = res {
                    if event.kind.is_modify() {
                        // Reload from storage
                        if let Ok(contents) = fs::read_to_string(&storage_path) {
                            if let Ok(data) = serde_json::from_str::<AuthData>(&contents) {
                                inner.save(&data.token, data.record);
                            }
                        }
                    }
                }
            },
            notify::Config::default(),
        )?;

        watcher.watch(&self.storage_path, RecursiveMode::NonRecursive)?;

        Ok(watcher)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::dtos::RecordModel;
    use std::env;
    use std::fs;

    fn temp_path() -> PathBuf {
        let id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        env::temp_dir().join(format!("pocketbase_test_{}.json", id))
    }

    #[test]
    fn test_save_and_load() {
        let path = temp_path();
        let path_str = path.to_str().unwrap();

        // Create store and save
        {
            let store = LocalAuthStore::new(Some(path_str));

            let record = RecordModel {
                id: "test123".to_string(),
                collection_id: "users".to_string(),
                collection_name: "users".to_string(),
                ..Default::default()
            };

            store.save("token123", Some(record));
        }

        // Create new store and verify data persisted
        {
            let store = LocalAuthStore::new(Some(path_str));

            assert_eq!(store.token(), "token123");
            assert!(store.record().is_some());
            assert_eq!(store.record().unwrap().id, "test123");
        }

        // Cleanup
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn test_clear() {
        let path = temp_path();
        let path_str = path.to_str().unwrap();

        let store = LocalAuthStore::new(Some(path_str));

        let record = RecordModel {
            id: "test123".to_string(),
            ..Default::default()
        };

        store.save("token123", Some(record));
        assert!(!store.token().is_empty());

        store.clear();
        assert!(store.token().is_empty());
        assert!(store.record().is_none());

        // Verify file is removed
        assert!(!path.exists());
    }

    #[test]
    fn test_fallback_to_memory() {
        // Use an invalid path that can't be written
        let store = LocalAuthStore::new(Some("/nonexistent/path/auth.json"));

        let record = RecordModel {
            id: "test123".to_string(),
            ..Default::default()
        };

        store.save("token123", Some(record));

        // Should still work via memory fallback
        assert_eq!(store.token(), "token123");
    }
}
