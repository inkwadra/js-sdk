//! Client Response Error type

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use thiserror::Error;

/// ClientResponseError is a custom Error type that wraps and normalizes
/// any error thrown by `Client::send()`.
#[derive(Debug, Clone, Error, Serialize, Deserialize)]
pub struct ClientResponseError {
    /// The URL of the request that failed.
    pub url: String,

    /// HTTP status code (0 if the request was aborted/failed).
    pub status: u16,

    /// The response data from the server.
    #[serde(default)]
    pub response: HashMap<String, serde_json::Value>,

    /// Whether the request was aborted.
    #[serde(default)]
    pub is_abort: bool,

    /// The error message.
    #[serde(default)]
    pub message: String,
}

impl Default for ClientResponseError {
    fn default() -> Self {
        Self {
            url: String::new(),
            status: 0,
            response: HashMap::new(),
            is_abort: false,
            message: "Something went wrong.".to_string(),
        }
    }
}

impl ClientResponseError {
    /// Creates a new ClientResponseError from the given error data.
    pub fn new(url: &str, status: u16, response: HashMap<String, serde_json::Value>) -> Self {
        let message = response
            .get("message")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "Something went wrong.".to_string());

        Self {
            url: url.to_string(),
            status,
            response,
            is_abort: false,
            message,
        }
    }

    /// Creates an abort error.
    pub fn abort() -> Self {
        Self {
            is_abort: true,
            message: "The request was cancelled.".to_string(),
            ..Default::default()
        }
    }

    /// Creates a 404 not found error.
    pub fn not_found(url: &str, message: &str) -> Self {
        let mut response = HashMap::new();
        response.insert("code".to_string(), serde_json::json!(404));
        response.insert("message".to_string(), serde_json::json!(message));
        response.insert("data".to_string(), serde_json::json!({}));

        Self {
            url: url.to_string(),
            status: 404,
            response,
            is_abort: false,
            message: message.to_string(),
        }
    }

    /// Gets the response data (alias for response for backward compatibility).
    pub fn data(&self) -> &HashMap<String, serde_json::Value> {
        &self.response
    }

    /// Converts the error to a JSON-serializable format.
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "url": self.url,
            "status": self.status,
            "response": self.response,
            "isAbort": self.is_abort,
            "message": self.message
        })
    }
}

impl fmt::Display for ClientResponseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ClientResponseError {}: {}",
            self.status, self.message
        )
    }
}

impl From<reqwest::Error> for ClientResponseError {
    fn from(err: reqwest::Error) -> Self {
        let is_abort = err.is_timeout();
        let status = err.status().map(|s| s.as_u16()).unwrap_or(0);
        let url = err.url().map(|u| u.to_string()).unwrap_or_default();

        Self {
            url,
            status,
            response: HashMap::new(),
            is_abort,
            message: err.to_string(),
        }
    }
}

impl From<serde_json::Error> for ClientResponseError {
    fn from(err: serde_json::Error) -> Self {
        Self {
            message: format!("JSON error: {}", err),
            ..Default::default()
        }
    }
}

impl From<url::ParseError> for ClientResponseError {
    fn from(err: url::ParseError) -> Self {
        Self {
            message: format!("URL parse error: {}", err),
            ..Default::default()
        }
    }
}
