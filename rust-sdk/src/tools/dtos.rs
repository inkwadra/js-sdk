//! Data Transfer Objects (DTOs)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Generic list result wrapper for paginated responses.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListResult<T> {
    pub page: u32,
    pub per_page: u32,
    pub total_items: u32,
    pub total_pages: u32,
    pub items: Vec<T>,
}

impl<T> Default for ListResult<T> {
    fn default() -> Self {
        Self {
            page: 1,
            per_page: 30,
            total_items: 0,
            total_pages: 0,
            items: Vec::new(),
        }
    }
}

/// Base model trait for all database models.
pub trait BaseModel {
    fn id(&self) -> &str;
}

/// Log model representing a log entry.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LogModel {
    pub id: String,
    pub level: String,
    pub message: String,
    pub created: String,
    pub updated: String,
    #[serde(default)]
    pub data: HashMap<String, serde_json::Value>,
}

impl BaseModel for LogModel {
    fn id(&self) -> &str {
        &self.id
    }
}

/// Record model representing a collection record.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RecordModel {
    pub id: String,
    pub collection_id: String,
    pub collection_name: String,
    #[serde(default)]
    pub expand: Option<HashMap<String, serde_json::Value>>,
    #[serde(flatten)]
    pub data: HashMap<String, serde_json::Value>,
}

impl BaseModel for RecordModel {
    fn id(&self) -> &str {
        &self.id
    }
}

impl RecordModel {
    /// Gets a field value from the record data.
    pub fn get<T: for<'de> Deserialize<'de>>(&self, key: &str) -> Option<T> {
        self.data.get(key).and_then(|v| serde_json::from_value(v.clone()).ok())
    }

    /// Gets a string field value from the record data.
    pub fn get_string(&self, key: &str) -> Option<String> {
        self.data.get(key).and_then(|v| v.as_str()).map(|s| s.to_string())
    }

    /// Gets an i64 field value from the record data.
    pub fn get_i64(&self, key: &str) -> Option<i64> {
        self.data.get(key).and_then(|v| v.as_i64())
    }

    /// Gets a bool field value from the record data.
    pub fn get_bool(&self, key: &str) -> Option<bool> {
        self.data.get(key).and_then(|v| v.as_bool())
    }
}

/// Collection field definition.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CollectionField {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub field_type: String,
    pub system: bool,
    pub hidden: bool,
    pub presentable: bool,
    #[serde(flatten)]
    pub options: HashMap<String, serde_json::Value>,
}

/// Token configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TokenConfig {
    pub duration: u64,
    pub secret: Option<String>,
}

/// Email template configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EmailTemplate {
    pub subject: String,
    pub body: String,
}

/// Auth alert configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AuthAlertConfig {
    pub enabled: bool,
    pub email_template: EmailTemplate,
}

/// OTP configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct OTPConfig {
    pub enabled: bool,
    pub duration: u64,
    pub length: u32,
    pub email_template: EmailTemplate,
}

/// MFA configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MFAConfig {
    pub enabled: bool,
    pub duration: u64,
    pub rule: String,
}

/// Password auth configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PasswordAuthConfig {
    pub enabled: bool,
    pub identity_fields: Vec<String>,
}

/// OAuth2 provider configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct OAuth2Provider {
    pub pkce: Option<bool>,
    pub client_id: String,
    pub name: String,
    pub client_secret: String,
    pub auth_url: String,
    pub token_url: String,
    pub user_info_url: String,
    pub display_name: String,
    #[serde(default)]
    pub extra: Option<HashMap<String, serde_json::Value>>,
}

/// OAuth2 configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct OAuth2Config {
    pub enabled: bool,
    pub mapped_fields: HashMap<String, String>,
    pub providers: Vec<OAuth2Provider>,
}

/// Base collection model.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BaseCollectionModel {
    pub id: String,
    pub name: String,
    pub fields: Vec<CollectionField>,
    pub indexes: Vec<String>,
    pub system: bool,
    pub list_rule: Option<String>,
    pub view_rule: Option<String>,
    pub create_rule: Option<String>,
    pub update_rule: Option<String>,
    pub delete_rule: Option<String>,
}

/// View collection model.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ViewCollectionModel {
    #[serde(flatten)]
    pub base: BaseCollectionModel,
    pub view_query: String,
}

/// Auth collection model.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthCollectionModel {
    #[serde(flatten)]
    pub base: BaseCollectionModel,
    pub auth_rule: Option<String>,
    pub manage_rule: Option<String>,
    pub auth_alert: AuthAlertConfig,
    pub oauth2: OAuth2Config,
    pub password_auth: PasswordAuthConfig,
    pub mfa: MFAConfig,
    pub otp: OTPConfig,
    pub auth_token: TokenConfig,
    pub password_reset_token: TokenConfig,
    pub email_change_token: TokenConfig,
    pub verification_token: TokenConfig,
    pub file_token: TokenConfig,
    pub verification_template: EmailTemplate,
    pub reset_password_template: EmailTemplate,
    pub confirm_email_change_template: EmailTemplate,
}

/// Collection model enum.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
#[allow(clippy::large_enum_variant)]
pub enum CollectionModel {
    Base(BaseCollectionModel),
    View(ViewCollectionModel),
    Auth(AuthCollectionModel),
}

impl CollectionModel {
    /// Gets the ID of the collection.
    pub fn id(&self) -> &str {
        match self {
            CollectionModel::Base(c) => &c.id,
            CollectionModel::View(c) => &c.base.id,
            CollectionModel::Auth(c) => &c.base.id,
        }
    }

    /// Gets the name of the collection.
    pub fn name(&self) -> &str {
        match self {
            CollectionModel::Base(c) => &c.name,
            CollectionModel::View(c) => &c.base.name,
            CollectionModel::Auth(c) => &c.base.name,
        }
    }
}

/// Health check response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckResponse {
    pub code: u16,
    pub message: String,
    #[serde(default)]
    pub data: HashMap<String, serde_json::Value>,
}

/// Hourly statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HourlyStats {
    pub total: u64,
    pub date: String,
}

/// Backup file info.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupFileInfo {
    pub key: String,
    pub size: u64,
    pub modified: String,
}

/// Cron job info.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CronJob {
    pub id: String,
    pub expression: String,
}

/// Auth provider info for OAuth2.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthProviderInfo {
    pub name: String,
    pub display_name: String,
    pub state: String,
    pub auth_url: String,
    pub code_verifier: String,
    pub code_challenge: String,
    pub code_challenge_method: String,
}

/// MFA info in auth methods list.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MfaInfo {
    pub enabled: bool,
    pub duration: u64,
}

/// OTP info in auth methods list.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OtpInfo {
    pub enabled: bool,
    pub duration: u64,
}

/// Password info in auth methods list.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PasswordInfo {
    pub enabled: bool,
    pub identity_fields: Vec<String>,
}

/// OAuth2 info in auth methods list.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuth2Info {
    pub enabled: bool,
    pub providers: Vec<AuthProviderInfo>,
}

/// Available auth methods for a collection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthMethodsList {
    pub mfa: MfaInfo,
    pub otp: OtpInfo,
    pub password: PasswordInfo,
    pub oauth2: OAuth2Info,
}

/// Record auth response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordAuthResponse {
    /// The signed PocketBase auth record.
    pub record: RecordModel,
    /// The PocketBase record auth token.
    pub token: String,
    /// Auth meta data usually filled when OAuth2 is used.
    #[serde(default)]
    pub meta: Option<HashMap<String, serde_json::Value>>,
}

/// OTP response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OTPResponse {
    pub otp_id: String,
}

/// Apple client secret response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppleClientSecret {
    pub secret: String,
}

/// Batch request definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchRequest {
    pub method: String,
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub json: Option<HashMap<String, serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<HashMap<String, String>>,
}

/// Batch request result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchRequestResult {
    pub status: u16,
    pub body: serde_json::Value,
}
