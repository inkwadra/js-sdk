//! File Service

use crate::client_response_error::ClientResponseError;
use crate::services::base_service::BaseService;
use crate::tools::dtos::RecordModel;
use crate::tools::options::{FileOptions, SendOptions};
use crate::Client;
use std::sync::Arc;

/// Service for file API endpoints.
pub struct FileService {
    client: Arc<Client>,
}

impl FileService {
    /// Creates a new FileService.
    pub fn new(client: Arc<Client>) -> Self {
        Self { client }
    }

    /// Builds and returns an absolute record file URL for the provided filename.
    pub fn get_url(&self, record: &RecordModel, filename: &str, options: Option<FileOptions>) -> String {
        if filename.is_empty() || record.id.is_empty() {
            return String::new();
        }

        let collection_ref = if !record.collection_id.is_empty() {
            &record.collection_id
        } else if !record.collection_name.is_empty() {
            &record.collection_name
        } else {
            return String::new();
        };

        let path = format!(
            "api/files/{}/{}/{}",
            urlencoding::encode(collection_ref),
            urlencoding::encode(&record.id),
            urlencoding::encode(filename)
        );

        let mut result = self.client.build_url(&path);

        if let Some(opts) = options {
            let mut query_parts = Vec::new();

            if let Some(thumb) = opts.thumb {
                query_parts.push(format!("thumb={}", urlencoding::encode(&thumb)));
            }

            if opts.download == Some(true) {
                query_parts.push("download=true".to_string());
            }

            if !query_parts.is_empty() {
                result.push(if result.contains('?') { '&' } else { '?' });
                result.push_str(&query_parts.join("&"));
            }
        }

        result
    }

    /// Requests a new private file access token for the current auth model.
    pub async fn get_token(&self) -> Result<String, ClientResponseError> {
        let options = SendOptions::post();
        let response: serde_json::Value = self.client.send("/api/files/token", options).await?;
        Ok(response
            .get("token")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string())
    }
}

impl BaseService for FileService {
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
