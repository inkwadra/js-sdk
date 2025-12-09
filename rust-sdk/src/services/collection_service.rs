//! Collection Service

use crate::client_response_error::ClientResponseError;
use crate::services::base_service::BaseService;
use crate::services::crud_service::CrudService;
use crate::tools::dtos::{CollectionModel, ListResult};
use crate::tools::options::{FullListOptions, ListOptions, SendOptions};
use crate::Client;
use async_trait::async_trait;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;

/// Service for collection API endpoints.
pub struct CollectionService {
    client: Arc<Client>,
}

impl CollectionService {
    /// Creates a new CollectionService.
    pub fn new(client: Arc<Client>) -> Self {
        Self { client }
    }

    fn base_crud_path(&self) -> &str {
        "/api/collections"
    }

    /// Imports the provided collections.
    /// If `delete_missing` is true, all local collections and their fields,
    /// that are not present in the imported configuration, WILL BE DELETED
    /// (including their related records data)!
    pub async fn import(
        &self,
        collections: Vec<CollectionModel>,
        delete_missing: bool,
    ) -> Result<bool, ClientResponseError> {
        let options = SendOptions::put().with_body(json!({
            "collections": collections,
            "deleteMissing": delete_missing
        }));
        self.client
            .send::<serde_json::Value>(&format!("{}/import", self.base_crud_path()), options)
            .await?;
        Ok(true)
    }

    /// Returns type indexed map with scaffolded collection models
    /// populated with their default field values.
    pub async fn get_scaffolds(
        &self,
    ) -> Result<HashMap<String, CollectionModel>, ClientResponseError> {
        let options = SendOptions::get();
        self.client
            .send(&format!("{}/meta/scaffolds", self.base_crud_path()), options)
            .await
    }

    /// Deletes all records associated with the specified collection.
    pub async fn truncate(&self, collection_id_or_name: &str) -> Result<bool, ClientResponseError> {
        let options = SendOptions::delete();
        let path = format!(
            "{}/{}/truncate",
            self.base_crud_path(),
            urlencoding::encode(collection_id_or_name)
        );
        self.client
            .send::<serde_json::Value>(&path, options)
            .await?;
        Ok(true)
    }
}

#[async_trait]
impl CrudService<CollectionModel> for CollectionService {
    fn base_crud_path(&self) -> String {
        "/api/collections".to_string()
    }

    fn client(&self) -> &Arc<Client> {
        &self.client
    }

    async fn get_full_list(
        &self,
        options: Option<FullListOptions>,
    ) -> Result<Vec<CollectionModel>, ClientResponseError> {
        let batch = options.as_ref().and_then(|o| o.batch).unwrap_or(500);
        self._get_full_list(batch, options.map(|o| o.list)).await
    }

    async fn get_list(
        &self,
        page: u32,
        per_page: u32,
        options: Option<ListOptions>,
    ) -> Result<ListResult<CollectionModel>, ClientResponseError> {
        let mut opts: SendOptions = options.unwrap_or_default().into();
        opts.method = "GET".to_string();
        opts.query.insert("page".to_string(), serde_json::json!(page));
        opts.query.insert("perPage".to_string(), serde_json::json!(per_page));
        self.client.send(self.base_crud_path(), opts).await
    }

    async fn get_one(&self, id: &str) -> Result<CollectionModel, ClientResponseError> {
        if id.is_empty() {
            return Err(ClientResponseError::not_found(
                &self.client.build_url(&format!("{}/", self.base_crud_path())),
                "Missing required record id.",
            ));
        }

        let options = SendOptions::get();
        let path = format!("{}/{}", self.base_crud_path(), urlencoding::encode(id));
        self.client.send(&path, options).await
    }

    async fn create(
        &self,
        body_params: serde_json::Value,
    ) -> Result<CollectionModel, ClientResponseError> {
        let options = SendOptions::post().with_body(body_params);
        self.client.send(self.base_crud_path(), options).await
    }

    async fn update(
        &self,
        id: &str,
        body_params: serde_json::Value,
    ) -> Result<CollectionModel, ClientResponseError> {
        let options = SendOptions::patch().with_body(body_params);
        let path = format!("{}/{}", self.base_crud_path(), urlencoding::encode(id));
        self.client.send(&path, options).await
    }

    async fn delete(&self, id: &str) -> Result<bool, ClientResponseError> {
        let options = SendOptions::delete();
        let path = format!("{}/{}", self.base_crud_path(), urlencoding::encode(id));
        self.client.send::<serde_json::Value>(&path, options).await?;
        Ok(true)
    }
}

impl BaseService for CollectionService {
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
