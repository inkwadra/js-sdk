//! PocketBase Client

use crate::client_response_error::ClientResponseError;
use crate::services::{
    BackupService, BatchService, CollectionService, CronService, FileService, HealthService,
    LogService, RecordService, SettingsService,
};
use crate::stores::BaseAuthStore;
use crate::tools::options::{serialize_query_params, SendOptions};
use chrono::{DateTime, Utc};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue, ACCEPT_LANGUAGE, AUTHORIZATION, CONTENT_TYPE};
use serde::de::DeserializeOwned;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::{Arc, RwLock};

/// PocketBase Client for making API requests.
pub struct Client {
    /// The base PocketBase backend URL address.
    base_url: String,

    /// Language code for Accept-Language header.
    lang: String,

    /// Auth store for managing authentication state.
    auth_store: Arc<BaseAuthStore>,

    /// HTTP client for making requests.
    http_client: reqwest::Client,

    /// Record services cache.
    record_services: RwLock<HashMap<String, Arc<RecordService>>>,

    /// Whether auto cancellation is enabled.
    enable_auto_cancellation: RwLock<bool>,
}

impl Client {
    /// Creates a new PocketBase client.
    pub fn new(base_url: &str) -> Arc<Self> {
        Self::with_auth_store(base_url, BaseAuthStore::new(), "en-US")
    }

    /// Creates a new PocketBase client with custom auth store and language.
    pub fn with_auth_store(base_url: &str, auth_store: BaseAuthStore, lang: &str) -> Arc<Self> {
        Arc::new(Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            lang: lang.to_string(),
            auth_store: Arc::new(auth_store),
            http_client: reqwest::Client::new(),
            record_services: RwLock::new(HashMap::new()),
            enable_auto_cancellation: RwLock::new(true),
        })
    }

    /// Returns the base URL.
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Returns the language code.
    pub fn lang(&self) -> &str {
        &self.lang
    }

    /// Returns a reference to the auth store.
    pub fn auth_store(&self) -> &BaseAuthStore {
        &self.auth_store
    }

    /// Builds a full client URL by safely concatenating the provided path.
    pub fn build_url(&self, path: &str) -> String {
        let mut url = self.base_url.clone();

        if !path.is_empty() {
            if !url.ends_with('/') {
                url.push('/');
            }
            if let Some(stripped) = path.strip_prefix('/') {
                url.push_str(stripped);
            } else {
                url.push_str(path);
            }
        }

        url
    }

    /// Globally enable or disable auto cancellation for pending duplicated requests.
    pub fn auto_cancellation(&self, enable: bool) {
        *self.enable_auto_cancellation.write().unwrap() = enable;
    }

    /// Returns the RecordService for the specified collection.
    pub fn collection(self: &Arc<Self>, id_or_name: &str) -> Arc<RecordService> {
        let mut services = self.record_services.write().unwrap();

        if !services.contains_key(id_or_name) {
            services.insert(
                id_or_name.to_string(),
                Arc::new(RecordService::new(Arc::clone(self), id_or_name)),
            );
        }

        Arc::clone(services.get(id_or_name).unwrap())
    }

    /// Returns the HealthService.
    pub fn health(self: &Arc<Self>) -> HealthService {
        HealthService::new(Arc::clone(self))
    }

    /// Returns the SettingsService.
    pub fn settings(self: &Arc<Self>) -> SettingsService {
        SettingsService::new(Arc::clone(self))
    }

    /// Returns the LogService.
    pub fn logs(self: &Arc<Self>) -> LogService {
        LogService::new(Arc::clone(self))
    }

    /// Returns the FileService.
    pub fn files(self: &Arc<Self>) -> FileService {
        FileService::new(Arc::clone(self))
    }

    /// Returns the CollectionService.
    pub fn collections(self: &Arc<Self>) -> CollectionService {
        CollectionService::new(Arc::clone(self))
    }

    /// Returns the BackupService.
    pub fn backups(self: &Arc<Self>) -> BackupService {
        BackupService::new(Arc::clone(self))
    }

    /// Returns the CronService.
    pub fn crons(self: &Arc<Self>) -> CronService {
        CronService::new(Arc::clone(self))
    }

    /// Creates a new BatchService for batch operations.
    pub fn create_batch(self: &Arc<Self>) -> BatchService {
        BatchService::new(Arc::clone(self))
    }

    /// Constructs a filter expression with placeholders populated from a parameters object.
    /// Placeholder parameters are defined with the `{:paramName}` notation.
    pub fn filter(&self, raw: &str, params: Option<HashMap<String, serde_json::Value>>) -> String {
        let Some(params) = params else {
            return raw.to_string();
        };

        let mut result = raw.to_string();

        for (key, val) in params {
            let formatted = match val {
                serde_json::Value::Bool(b) => b.to_string(),
                serde_json::Value::Number(n) => n.to_string(),
                serde_json::Value::String(s) => format!("'{}'", s.replace('\'', "\\'")),
                serde_json::Value::Null => "null".to_string(),
                _ => {
                    // For Date handling (if it's a string that looks like a date)
                    if let Some(s) = val.as_str() {
                        if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
                            let utc: DateTime<Utc> = dt.into();
                            format!("'{}'", utc.format("%Y-%m-%d %H:%M:%S%.3fZ"))
                        } else {
                            format!(
                                "'{}'",
                                serde_json::to_string(&val)
                                    .unwrap_or_default()
                                    .replace('\'', "\\'")
                            )
                        }
                    } else {
                        format!(
                            "'{}'",
                            serde_json::to_string(&val)
                                .unwrap_or_default()
                                .replace('\'', "\\'")
                        )
                    }
                }
            };

            result = result.replace(&format!("{{:{}}}", key), &formatted);
        }

        result
    }

    /// Sends an API HTTP request.
    pub async fn send<T: DeserializeOwned>(
        &self,
        path: &str,
        options: SendOptions,
    ) -> Result<T, ClientResponseError> {
        let url = self.build_url(path);

        // Build the request
        let method = reqwest::Method::from_str(&options.method).unwrap_or(reqwest::Method::GET);
        let mut request_builder = self.http_client.request(method, &url);

        // Build headers
        let mut headers = HeaderMap::new();

        // Add Content-Type header for JSON if not already set
        let has_content_type = options.headers.keys().any(|k| k.eq_ignore_ascii_case("content-type"));
        if !has_content_type && options.body.is_some() {
            headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        }

        // Add Accept-Language header if not already set
        let has_accept_language = options.headers.keys().any(|k| k.eq_ignore_ascii_case("accept-language"));
        if !has_accept_language {
            if let Ok(val) = HeaderValue::from_str(&self.lang) {
                headers.insert(ACCEPT_LANGUAGE, val);
            }
        }

        // Add Authorization header if token is present and not already set
        let token = self.auth_store.token();
        let has_auth = options.headers.keys().any(|k| k.eq_ignore_ascii_case("authorization"));
        if !has_auth && !token.is_empty() {
            if let Ok(val) = HeaderValue::from_str(&token) {
                headers.insert(AUTHORIZATION, val);
            }
        }

        // Add custom headers
        for (key, value) in &options.headers {
            if let (Ok(name), Ok(val)) = (HeaderName::from_str(key), HeaderValue::from_str(value)) {
                headers.insert(name, val);
            }
        }

        request_builder = request_builder.headers(headers);

        // Add query parameters
        if !options.query.is_empty() {
            let query_string = serialize_query_params(&options.query);
            if !query_string.is_empty() {
                let full_url = if url.contains('?') {
                    format!("{}&{}", url, query_string)
                } else {
                    format!("{}?{}", url, query_string)
                };
                request_builder = self
                    .http_client
                    .request(reqwest::Method::from_str(&options.method).unwrap_or(reqwest::Method::GET), &full_url);
                
                // Re-add headers since we recreated the builder
                let mut headers = HeaderMap::new();
                if !has_content_type && options.body.is_some() {
                    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }
                if !has_accept_language {
                    if let Ok(val) = HeaderValue::from_str(&self.lang) {
                        headers.insert(ACCEPT_LANGUAGE, val);
                    }
                }
                if !has_auth && !token.is_empty() {
                    if let Ok(val) = HeaderValue::from_str(&token) {
                        headers.insert(AUTHORIZATION, val);
                    }
                }
                for (key, value) in &options.headers {
                    if let (Ok(name), Ok(val)) = (HeaderName::from_str(key), HeaderValue::from_str(value)) {
                        headers.insert(name, val);
                    }
                }
                request_builder = request_builder.headers(headers);
            }
        }

        // Add body
        if let Some(body) = options.body {
            request_builder = request_builder.json(&body);
        }

        // Send the request
        let response = request_builder.send().await?;

        let status = response.status().as_u16();
        let url = response.url().to_string();

        // Parse response
        let text = response.text().await.unwrap_or_default();

        if status >= 400 {
            let data: HashMap<String, serde_json::Value> =
                serde_json::from_str(&text).unwrap_or_default();
            return Err(ClientResponseError::new(&url, status, data));
        }

        // Handle empty response (204 No Content)
        if text.is_empty() {
            // Try to deserialize from empty JSON object
            return serde_json::from_str("{}").map_err(|e| e.into());
        }

        serde_json::from_str(&text).map_err(|e| e.into())
    }
}

impl std::fmt::Debug for Client {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Client")
            .field("base_url", &self.base_url)
            .field("lang", &self.lang)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_url() {
        let client = Client::new("http://localhost:8090");
        assert_eq!(client.build_url("/api/health"), "http://localhost:8090/api/health");
        assert_eq!(client.build_url("api/health"), "http://localhost:8090/api/health");
    }

    #[test]
    fn test_filter() {
        let client = Client::new("http://localhost:8090");
        
        let mut params = HashMap::new();
        params.insert("title".to_string(), serde_json::json!("example"));
        params.insert("count".to_string(), serde_json::json!(42));
        params.insert("active".to_string(), serde_json::json!(true));

        let result = client.filter(
            "title = {:title} && count > {:count} && active = {:active}",
            Some(params),
        );

        assert_eq!(
            result,
            "title = 'example' && count > 42 && active = true"
        );
    }
}
