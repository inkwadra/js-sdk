//! Log Service

use crate::client_response_error::ClientResponseError;
use crate::services::base_service::BaseService;
use crate::tools::dtos::{HourlyStats, ListResult, LogModel};
use crate::tools::options::{ListOptions, LogStatsOptions, SendOptions};
use crate::Client;
use std::sync::Arc;

/// Service for log API endpoints.
pub struct LogService {
    client: Arc<Client>,
}

impl LogService {
    /// Creates a new LogService.
    pub fn new(client: Arc<Client>) -> Self {
        Self { client }
    }

    /// Returns paginated logs list.
    pub async fn get_list(
        &self,
        page: u32,
        per_page: u32,
        options: Option<ListOptions>,
    ) -> Result<ListResult<LogModel>, ClientResponseError> {
        let mut opts: SendOptions = options.unwrap_or_default().into();
        opts.method = "GET".to_string();
        opts.query
            .insert("page".to_string(), serde_json::json!(page));
        opts.query
            .insert("perPage".to_string(), serde_json::json!(per_page));

        self.client.send("/api/logs", opts).await
    }

    /// Returns a single log by its ID.
    /// If `id` is empty it will return a 404 error.
    pub async fn get_one(&self, id: &str) -> Result<LogModel, ClientResponseError> {
        if id.is_empty() {
            return Err(ClientResponseError::not_found(
                &self.client.build_url("/api/logs/"),
                "Missing required log id.",
            ));
        }

        let options = SendOptions::get();
        let path = format!("/api/logs/{}", urlencoding::encode(id));
        self.client.send(&path, options).await
    }

    /// Returns logs statistics.
    pub async fn get_stats(
        &self,
        options: Option<LogStatsOptions>,
    ) -> Result<Vec<HourlyStats>, ClientResponseError> {
        let opts: SendOptions = options.unwrap_or_default().into();
        self.client.send("/api/logs/stats", opts).await
    }
}

impl BaseService for LogService {
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
