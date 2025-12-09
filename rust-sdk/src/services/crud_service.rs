//! CRUD Service trait

use crate::client_response_error::ClientResponseError;
use crate::tools::dtos::ListResult;
use crate::tools::options::{FullListOptions, ListOptions};
use crate::Client;
use async_trait::async_trait;
use serde::de::DeserializeOwned;
use std::sync::Arc;

/// Trait for services that support CRUD operations.
#[async_trait]
pub trait CrudService<M: DeserializeOwned + Clone + Send + Sync>: Send + Sync {
    /// Returns the base path for CRUD operations.
    fn base_crud_path(&self) -> String;

    /// Returns a reference to the client.
    fn client(&self) -> &Arc<Client>;

    /// Returns all list items in batches.
    async fn get_full_list(
        &self,
        options: Option<FullListOptions>,
    ) -> Result<Vec<M>, ClientResponseError>;

    /// Returns paginated items list.
    async fn get_list(
        &self,
        page: u32,
        per_page: u32,
        options: Option<ListOptions>,
    ) -> Result<ListResult<M>, ClientResponseError>;

    /// Returns the first found item by the specified filter.
    async fn get_first_list_item(
        &self,
        filter: &str,
        options: Option<ListOptions>,
    ) -> Result<M, ClientResponseError> {
        let mut opts = options.unwrap_or_default();
        opts.filter = Some(filter.to_string());
        opts.skip_total = Some(true);

        let result = self.get_list(1, 1, Some(opts)).await?;

        if result.items.is_empty() {
            return Err(ClientResponseError::not_found(
                &self.client().build_url(&self.base_crud_path()),
                "The requested resource wasn't found.",
            ));
        }

        Ok(result.items.into_iter().next().unwrap())
    }

    /// Returns single item by its ID.
    async fn get_one(&self, id: &str) -> Result<M, ClientResponseError>;

    /// Creates a new item.
    async fn create(&self, body_params: serde_json::Value) -> Result<M, ClientResponseError>;

    /// Updates an existing item by its ID.
    async fn update(
        &self,
        id: &str,
        body_params: serde_json::Value,
    ) -> Result<M, ClientResponseError>;

    /// Deletes an existing item by its ID.
    async fn delete(&self, id: &str) -> Result<bool, ClientResponseError>;

    /// Internal method to fetch all items in batches.
    async fn _get_full_list(
        &self,
        batch_size: u32,
        options: Option<ListOptions>,
    ) -> Result<Vec<M>, ClientResponseError> {
        let mut result: Vec<M> = Vec::new();
        let mut page = 1u32;

        loop {
            let mut opts = options.clone().unwrap_or_default();
            opts.skip_total = Some(true);

            let list = self.get_list(page, batch_size, Some(opts)).await?;
            let items_count = list.items.len();

            result.extend(list.items);

            if (items_count as u32) < batch_size {
                break;
            }

            page += 1;
        }

        Ok(result)
    }
}

/// Helper function to encode URL path segments.
pub fn encode_uri_component(s: &str) -> String {
    let mut result = String::new();
    for c in s.chars() {
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
