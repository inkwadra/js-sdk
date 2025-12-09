//! Cron Service

use crate::client_response_error::ClientResponseError;
use crate::services::base_service::BaseService;
use crate::tools::dtos::CronJob;
use crate::tools::options::SendOptions;
use crate::Client;
use std::sync::Arc;

/// Service for cron job API endpoints.
pub struct CronService {
    client: Arc<Client>,
}

impl CronService {
    /// Creates a new CronService.
    pub fn new(client: Arc<Client>) -> Self {
        Self { client }
    }

    /// Returns list with all registered cron jobs.
    pub async fn get_full_list(&self) -> Result<Vec<CronJob>, ClientResponseError> {
        let options = SendOptions::get();
        self.client.send("/api/crons", options).await
    }

    /// Runs the specified cron job.
    pub async fn run(&self, job_id: &str) -> Result<bool, ClientResponseError> {
        let options = SendOptions::post();
        let path = format!("/api/crons/{}", urlencoding::encode(job_id));
        self.client
            .send::<serde_json::Value>(&path, options)
            .await?;
        Ok(true)
    }
}

impl BaseService for CronService {
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
