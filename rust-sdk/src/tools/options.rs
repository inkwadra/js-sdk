//! Send options and query parameters

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Options for sending HTTP requests.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendOptions {
    /// HTTP method (GET, POST, PUT, PATCH, DELETE).
    #[serde(default = "default_method")]
    pub method: String,

    /// Custom headers to send with the request.
    #[serde(default)]
    pub headers: HashMap<String, String>,

    /// The body of the request.
    #[serde(default)]
    pub body: Option<serde_json::Value>,

    /// Query parameters that will be appended to the request URL.
    #[serde(default)]
    pub query: HashMap<String, serde_json::Value>,

    /// The request identifier that can be used to cancel pending requests.
    #[serde(default)]
    pub request_key: Option<String>,
}

fn default_method() -> String {
    "GET".to_string()
}

impl SendOptions {
    /// Creates new SendOptions with GET method.
    pub fn get() -> Self {
        Self {
            method: "GET".to_string(),
            ..Default::default()
        }
    }

    /// Creates new SendOptions with POST method.
    pub fn post() -> Self {
        Self {
            method: "POST".to_string(),
            ..Default::default()
        }
    }

    /// Creates new SendOptions with PATCH method.
    pub fn patch() -> Self {
        Self {
            method: "PATCH".to_string(),
            ..Default::default()
        }
    }

    /// Creates new SendOptions with PUT method.
    pub fn put() -> Self {
        Self {
            method: "PUT".to_string(),
            ..Default::default()
        }
    }

    /// Creates new SendOptions with DELETE method.
    pub fn delete() -> Self {
        Self {
            method: "DELETE".to_string(),
            ..Default::default()
        }
    }

    /// Sets the body of the request.
    pub fn with_body(mut self, body: serde_json::Value) -> Self {
        self.body = Some(body);
        self
    }

    /// Sets a query parameter.
    pub fn with_query(mut self, key: &str, value: impl Into<serde_json::Value>) -> Self {
        self.query.insert(key.to_string(), value.into());
        self
    }

    /// Sets a header.
    pub fn with_header(mut self, key: &str, value: &str) -> Self {
        self.headers.insert(key.to_string(), value.to_string());
        self
    }
}

/// Common options extending SendOptions with fields parameter.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommonOptions {
    #[serde(flatten)]
    pub send: SendOptions,

    /// Comma-separated list of fields to include in the response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fields: Option<String>,
}

impl CommonOptions {
    /// Creates new CommonOptions with GET method.
    pub fn get() -> Self {
        Self {
            send: SendOptions::get(),
            fields: None,
        }
    }

    /// Sets the fields to include in the response.
    pub fn with_fields(mut self, fields: &str) -> Self {
        self.fields = Some(fields.to_string());
        self
    }
}

impl From<CommonOptions> for SendOptions {
    fn from(opts: CommonOptions) -> Self {
        let mut send = opts.send;
        if let Some(fields) = opts.fields {
            send.query.insert("fields".to_string(), serde_json::json!(fields));
        }
        send
    }
}

/// Options for list operations.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListOptions {
    #[serde(flatten)]
    pub common: CommonOptions,

    /// Page number (1-based).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<u32>,

    /// Number of items per page.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub per_page: Option<u32>,

    /// Sort expression.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort: Option<String>,

    /// Filter expression.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<String>,

    /// Whether to skip counting total items.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skip_total: Option<bool>,
}

impl ListOptions {
    /// Creates new ListOptions.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the page number.
    pub fn page(mut self, page: u32) -> Self {
        self.page = Some(page);
        self
    }

    /// Sets the number of items per page.
    pub fn per_page(mut self, per_page: u32) -> Self {
        self.per_page = Some(per_page);
        self
    }

    /// Sets the sort expression.
    pub fn sort(mut self, sort: &str) -> Self {
        self.sort = Some(sort.to_string());
        self
    }

    /// Sets the filter expression.
    pub fn filter(mut self, filter: &str) -> Self {
        self.filter = Some(filter.to_string());
        self
    }

    /// Skips counting total items.
    pub fn skip_total(mut self) -> Self {
        self.skip_total = Some(true);
        self
    }
}

impl From<ListOptions> for SendOptions {
    fn from(opts: ListOptions) -> Self {
        let mut send: SendOptions = opts.common.into();
        if let Some(page) = opts.page {
            send.query.insert("page".to_string(), serde_json::json!(page));
        }
        if let Some(per_page) = opts.per_page {
            send.query.insert("perPage".to_string(), serde_json::json!(per_page));
        }
        if let Some(sort) = opts.sort {
            send.query.insert("sort".to_string(), serde_json::json!(sort));
        }
        if let Some(filter) = opts.filter {
            send.query.insert("filter".to_string(), serde_json::json!(filter));
        }
        if let Some(skip_total) = opts.skip_total {
            send.query.insert("skipTotal".to_string(), serde_json::json!(skip_total));
        }
        send
    }
}

/// Options for full list operations.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FullListOptions {
    #[serde(flatten)]
    pub list: ListOptions,

    /// Batch size for fetching.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub batch: Option<u32>,
}

impl FullListOptions {
    /// Creates new FullListOptions.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the batch size.
    pub fn batch(mut self, batch: u32) -> Self {
        self.batch = Some(batch);
        self
    }
}

/// Options for record operations.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RecordOptions {
    #[serde(flatten)]
    pub common: CommonOptions,

    /// Comma-separated list of relations to expand.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expand: Option<String>,
}

impl RecordOptions {
    /// Creates new RecordOptions.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the expand parameter.
    pub fn expand(mut self, expand: &str) -> Self {
        self.expand = Some(expand.to_string());
        self
    }
}

impl From<RecordOptions> for SendOptions {
    fn from(opts: RecordOptions) -> Self {
        let mut send: SendOptions = opts.common.into();
        if let Some(expand) = opts.expand {
            send.query.insert("expand".to_string(), serde_json::json!(expand));
        }
        send
    }
}

/// Options for record list operations.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RecordListOptions {
    #[serde(flatten)]
    pub list: ListOptions,

    /// Comma-separated list of relations to expand.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expand: Option<String>,
}

impl RecordListOptions {
    /// Creates new RecordListOptions.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the expand parameter.
    pub fn expand(mut self, expand: &str) -> Self {
        self.expand = Some(expand.to_string());
        self
    }

    /// Sets the filter expression.
    pub fn filter(mut self, filter: &str) -> Self {
        self.list.filter = Some(filter.to_string());
        self
    }

    /// Sets the sort expression.
    pub fn sort(mut self, sort: &str) -> Self {
        self.list.sort = Some(sort.to_string());
        self
    }
}

impl From<RecordListOptions> for SendOptions {
    fn from(opts: RecordListOptions) -> Self {
        let mut send: SendOptions = opts.list.into();
        if let Some(expand) = opts.expand {
            send.query.insert("expand".to_string(), serde_json::json!(expand));
        }
        send
    }
}

/// Options for full record list operations.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RecordFullListOptions {
    #[serde(flatten)]
    pub full_list: FullListOptions,

    /// Comma-separated list of relations to expand.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expand: Option<String>,
}

impl RecordFullListOptions {
    /// Creates new RecordFullListOptions.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the batch size.
    pub fn batch(mut self, batch: u32) -> Self {
        self.full_list.batch = Some(batch);
        self
    }

    /// Sets the expand parameter.
    pub fn expand(mut self, expand: &str) -> Self {
        self.expand = Some(expand.to_string());
        self
    }
}

/// Options for file operations.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FileOptions {
    #[serde(flatten)]
    pub common: CommonOptions,

    /// Thumbnail size (e.g., "100x100").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thumb: Option<String>,

    /// Whether to force download.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub download: Option<bool>,
}

impl FileOptions {
    /// Creates new FileOptions.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the thumbnail size.
    pub fn thumb(mut self, thumb: &str) -> Self {
        self.thumb = Some(thumb.to_string());
        self
    }

    /// Forces download.
    pub fn download(mut self) -> Self {
        self.download = Some(true);
        self
    }
}

/// Options for log stats.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LogStatsOptions {
    #[serde(flatten)]
    pub common: CommonOptions,

    /// Filter expression.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<String>,
}

impl LogStatsOptions {
    /// Creates new LogStatsOptions.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the filter expression.
    pub fn filter(mut self, filter: &str) -> Self {
        self.filter = Some(filter.to_string());
        self
    }
}

impl From<LogStatsOptions> for SendOptions {
    fn from(opts: LogStatsOptions) -> Self {
        let mut send: SendOptions = opts.common.into();
        if let Some(filter) = opts.filter {
            send.query.insert("filter".to_string(), serde_json::json!(filter));
        }
        send
    }
}

/// Serializes query parameters into a URL query string.
pub fn serialize_query_params(params: &HashMap<String, serde_json::Value>) -> String {
    let mut result: Vec<String> = Vec::new();

    for (key, value) in params {
        let encoded_key = urlencoding::encode(key);
        
        if let Some(arr) = value.as_array() {
            for v in arr {
                if let Some(s) = prepare_query_param_value(v) {
                    result.push(format!("{}={}", encoded_key, s));
                }
            }
        } else if let Some(s) = prepare_query_param_value(value) {
            result.push(format!("{}={}", encoded_key, s));
        }
    }

    result.join("&")
}

/// Prepares a query parameter value for URL encoding.
fn prepare_query_param_value(value: &serde_json::Value) -> Option<String> {
    match value {
        serde_json::Value::Null => None,
        serde_json::Value::Bool(b) => Some(urlencoding::encode(&b.to_string()).to_string()),
        serde_json::Value::Number(n) => Some(urlencoding::encode(&n.to_string()).to_string()),
        serde_json::Value::String(s) => Some(urlencoding::encode(s).to_string()),
        serde_json::Value::Array(_) | serde_json::Value::Object(_) => {
            Some(urlencoding::encode(&serde_json::to_string(value).unwrap_or_default()).to_string())
        }
    }
}

// Re-export urlencoding for internal use
mod urlencoding {
    pub fn encode(input: &str) -> String {
        let mut result = String::new();
        for c in input.chars() {
            match c {
                'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' | '.' | '~' => {
                    result.push(c);
                }
                ' ' => {
                    result.push_str("%20");
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
