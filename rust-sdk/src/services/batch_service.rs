//! Batch Service

use crate::client_response_error::ClientResponseError;
use crate::services::base_service::BaseService;
use crate::services::crud_service::encode_uri_component;
use crate::tools::dtos::{BatchRequest, BatchRequestResult};
use crate::tools::options::SendOptions;
use crate::Client;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;

/// Service for batch API operations.
pub struct BatchService {
    client: Arc<Client>,
    #[allow(dead_code)]
    requests: Vec<BatchRequest>,
    subs: HashMap<String, SubBatchService>,
}

impl BatchService {
    /// Creates a new BatchService.
    pub fn new(client: Arc<Client>) -> Self {
        Self {
            client,
            requests: Vec::new(),
            subs: HashMap::new(),
        }
    }

    /// Starts constructing a batch request entry for the specified collection.
    pub fn collection(&mut self, collection_id_or_name: &str) -> &mut SubBatchService {
        if !self.subs.contains_key(collection_id_or_name) {
            self.subs.insert(
                collection_id_or_name.to_string(),
                SubBatchService::new(collection_id_or_name),
            );
        }
        self.subs.get_mut(collection_id_or_name).unwrap()
    }

    /// Sends the batch requests.
    pub async fn send(&mut self) -> Result<Vec<BatchRequestResult>, ClientResponseError> {
        // Collect all requests from sub-services
        let mut all_requests: Vec<BatchRequest> = Vec::new();
        for sub in self.subs.values() {
            all_requests.extend(sub.requests.clone());
        }

        let json_data: Vec<serde_json::Value> = all_requests
            .iter()
            .map(|req| {
                json!({
                    "method": req.method,
                    "url": req.url,
                    "headers": req.headers,
                    "body": req.json
                })
            })
            .collect();

        let options = SendOptions::post().with_body(json!({
            "requests": json_data
        }));

        self.client.send("/api/batch", options).await
    }
}

impl BaseService for BatchService {
    fn client(&self) -> &Arc<Client> {
        &self.client
    }
}

/// Sub-batch service for a specific collection.
pub struct SubBatchService {
    collection_id_or_name: String,
    requests: Vec<BatchRequest>,
}

impl SubBatchService {
    /// Creates a new SubBatchService.
    pub fn new(collection_id_or_name: &str) -> Self {
        Self {
            collection_id_or_name: collection_id_or_name.to_string(),
            requests: Vec::new(),
        }
    }

    /// Registers a record upsert request into the current batch queue.
    pub fn upsert(&mut self, body_params: serde_json::Value) {
        let url = format!(
            "/api/collections/{}/records",
            encode_uri_component(&self.collection_id_or_name)
        );

        self.requests.push(BatchRequest {
            method: "PUT".to_string(),
            url,
            json: Some(
                body_params
                    .as_object()
                    .map(|o| o.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
                    .unwrap_or_default(),
            ),
            headers: None,
        });
    }

    /// Registers a record create request into the current batch queue.
    pub fn create(&mut self, body_params: serde_json::Value) {
        let url = format!(
            "/api/collections/{}/records",
            encode_uri_component(&self.collection_id_or_name)
        );

        self.requests.push(BatchRequest {
            method: "POST".to_string(),
            url,
            json: Some(
                body_params
                    .as_object()
                    .map(|o| o.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
                    .unwrap_or_default(),
            ),
            headers: None,
        });
    }

    /// Registers a record update request into the current batch queue.
    pub fn update(&mut self, id: &str, body_params: serde_json::Value) {
        let url = format!(
            "/api/collections/{}/records/{}",
            encode_uri_component(&self.collection_id_or_name),
            encode_uri_component(id)
        );

        self.requests.push(BatchRequest {
            method: "PATCH".to_string(),
            url,
            json: Some(
                body_params
                    .as_object()
                    .map(|o| o.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
                    .unwrap_or_default(),
            ),
            headers: None,
        });
    }

    /// Registers a record delete request into the current batch queue.
    pub fn delete(&mut self, id: &str) {
        let url = format!(
            "/api/collections/{}/records/{}",
            encode_uri_component(&self.collection_id_or_name),
            encode_uri_component(id)
        );

        self.requests.push(BatchRequest {
            method: "DELETE".to_string(),
            url,
            json: None,
            headers: None,
        });
    }
}
