//! Settings Service

use crate::client_response_error::ClientResponseError;
use crate::services::base_service::BaseService;
use crate::tools::dtos::AppleClientSecret;
use crate::tools::options::SendOptions;
use crate::Client;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;

/// Service for settings API endpoints.
pub struct SettingsService {
    client: Arc<Client>,
}

impl SettingsService {
    /// Creates a new SettingsService.
    pub fn new(client: Arc<Client>) -> Self {
        Self { client }
    }

    /// Fetches all available app settings.
    pub async fn get_all(&self) -> Result<HashMap<String, serde_json::Value>, ClientResponseError> {
        let options = SendOptions::get();
        self.client.send("/api/settings", options).await
    }

    /// Bulk updates app settings.
    pub async fn update(
        &self,
        body_params: HashMap<String, serde_json::Value>,
    ) -> Result<HashMap<String, serde_json::Value>, ClientResponseError> {
        let options = SendOptions::patch().with_body(json!(body_params));
        self.client.send("/api/settings", options).await
    }

    /// Performs a S3 filesystem connection test.
    /// The currently supported `filesystem` are "storage" and "backups".
    pub async fn test_s3(&self, filesystem: &str) -> Result<bool, ClientResponseError> {
        let options = SendOptions::post().with_body(json!({ "filesystem": filesystem }));
        self.client
            .send::<serde_json::Value>("/api/settings/test/s3", options)
            .await?;
        Ok(true)
    }

    /// Sends a test email.
    /// The possible `email_template` values are: verification, password-reset, email-change.
    pub async fn test_email(
        &self,
        collection_id_or_name: &str,
        to_email: &str,
        email_template: &str,
    ) -> Result<bool, ClientResponseError> {
        let options = SendOptions::post().with_body(json!({
            "email": to_email,
            "template": email_template,
            "collection": collection_id_or_name
        }));
        self.client
            .send::<serde_json::Value>("/api/settings/test/email", options)
            .await?;
        Ok(true)
    }

    /// Generates a new Apple OAuth2 client secret.
    pub async fn generate_apple_client_secret(
        &self,
        client_id: &str,
        team_id: &str,
        key_id: &str,
        private_key: &str,
        duration: u64,
    ) -> Result<AppleClientSecret, ClientResponseError> {
        let options = SendOptions::post().with_body(json!({
            "clientId": client_id,
            "teamId": team_id,
            "keyId": key_id,
            "privateKey": private_key,
            "duration": duration
        }));
        self.client
            .send("/api/settings/apple/generate-client-secret", options)
            .await
    }
}

impl BaseService for SettingsService {
    fn client(&self) -> &Arc<Client> {
        &self.client
    }
}
