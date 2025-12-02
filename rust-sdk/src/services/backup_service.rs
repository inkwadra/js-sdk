//! Backup Service

use crate::client_response_error::ClientResponseError;
use crate::services::base_service::BaseService;
use crate::tools::dtos::BackupFileInfo;
use crate::tools::options::SendOptions;
use crate::Client;
use serde_json::json;
use std::sync::Arc;

/// Service for backup API endpoints.
pub struct BackupService {
    client: Arc<Client>,
}

impl BackupService {
    /// Creates a new BackupService.
    pub fn new(client: Arc<Client>) -> Self {
        Self { client }
    }

    /// Returns list with all available backup files.
    pub async fn get_full_list(&self) -> Result<Vec<BackupFileInfo>, ClientResponseError> {
        let options = SendOptions::get();
        self.client.send("/api/backups", options).await
    }

    /// Initializes a new backup.
    pub async fn create(&self, basename: &str) -> Result<bool, ClientResponseError> {
        let options = SendOptions::post().with_body(json!({ "name": basename }));
        self.client
            .send::<serde_json::Value>("/api/backups", options)
            .await?;
        Ok(true)
    }

    /// Uploads an existing backup file.
    pub async fn upload(
        &self,
        body_params: serde_json::Value,
    ) -> Result<bool, ClientResponseError> {
        let options = SendOptions::post().with_body(body_params);
        self.client
            .send::<serde_json::Value>("/api/backups/upload", options)
            .await?;
        Ok(true)
    }

    /// Deletes a single backup file.
    pub async fn delete(&self, key: &str) -> Result<bool, ClientResponseError> {
        let options = SendOptions::delete();
        let path = format!("/api/backups/{}", urlencoding::encode(key));
        self.client
            .send::<serde_json::Value>(&path, options)
            .await?;
        Ok(true)
    }

    /// Initializes an app data restore from an existing backup.
    pub async fn restore(&self, key: &str) -> Result<bool, ClientResponseError> {
        let options = SendOptions::post();
        let path = format!("/api/backups/{}/restore", urlencoding::encode(key));
        self.client
            .send::<serde_json::Value>(&path, options)
            .await?;
        Ok(true)
    }

    /// Builds a download URL for a single existing backup using a
    /// superuser file token and the backup file key.
    pub fn get_download_url(&self, token: &str, key: &str) -> String {
        self.client.build_url(&format!(
            "/api/backups/{}?token={}",
            urlencoding::encode(key),
            urlencoding::encode(token)
        ))
    }
}

impl BaseService for BackupService {
    fn client(&self) -> &Arc<Client> {
        &self.client
    }
}

mod urlencoding {
    pub fn encode(input: &str) -> String {
        let mut result = String::new();
        for c in input.chars() {
            match c {
                'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' | '.' | '~' => {
                    result.push(c);
                }
                _ => {
                    for byte in c.to_string().as_bytes() {
                        result.push_str(&format!("%{:02X}", byte));
                    }
                }
            }
        }
        result
    }
}
