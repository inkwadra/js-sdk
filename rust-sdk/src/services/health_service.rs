//! Health Service

use crate::client_response_error::ClientResponseError;
use crate::services::base_service::BaseService;
use crate::tools::dtos::HealthCheckResponse;
use crate::tools::options::SendOptions;
use crate::Client;
use std::sync::Arc;

/// Service for health check API endpoints.
pub struct HealthService {
    client: Arc<Client>,
}

impl HealthService {
    /// Creates a new HealthService.
    pub fn new(client: Arc<Client>) -> Self {
        Self { client }
    }

    /// Checks the health status of the API.
    pub async fn check(&self) -> Result<HealthCheckResponse, ClientResponseError> {
        let options = SendOptions::get();
        self.client.send("/api/health", options).await
    }
}

impl BaseService for HealthService {
    fn client(&self) -> &Arc<Client> {
        &self.client
    }
}
