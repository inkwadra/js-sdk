//! Record Service

use crate::client_response_error::ClientResponseError;
use crate::services::base_service::BaseService;
use crate::services::crud_service::{encode_uri_component, CrudService};
use crate::tools::dtos::{
    AuthMethodsList, ListResult, OTPResponse, RecordAuthResponse, RecordModel,
};
use crate::tools::options::{
    FullListOptions, ListOptions, RecordOptions, SendOptions,
};
use crate::Client;
use async_trait::async_trait;
use serde_json::json;
use std::sync::Arc;

/// Service for record API endpoints.
pub struct RecordService {
    client: Arc<Client>,
    collection_id_or_name: String,
}

impl RecordService {
    /// Creates a new RecordService for the specified collection.
    pub fn new(client: Arc<Client>, collection_id_or_name: &str) -> Self {
        Self {
            client,
            collection_id_or_name: collection_id_or_name.to_string(),
        }
    }

    /// Returns the current collection service base path.
    pub fn base_collection_path(&self) -> String {
        format!(
            "/api/collections/{}",
            encode_uri_component(&self.collection_id_or_name)
        )
    }

    /// Returns whether the current service collection is superusers.
    pub fn is_superusers(&self) -> bool {
        self.collection_id_or_name == "_superusers"
            || self.collection_id_or_name == "_pbc_2773867675"
    }

    // ---------------------------------------------------------------
    // Auth handlers
    // ---------------------------------------------------------------

    /// Returns all available collection auth methods.
    pub async fn list_auth_methods(&self) -> Result<AuthMethodsList, ClientResponseError> {
        let options = SendOptions::get().with_query("fields", "mfa,otp,password,oauth2");
        self.client
            .send(&format!("{}/auth-methods", self.base_collection_path()), options)
            .await
    }

    /// Authenticate a single auth collection record via its username/email and password.
    pub async fn auth_with_password(
        &self,
        username_or_email: &str,
        password: &str,
        options: Option<RecordOptions>,
    ) -> Result<RecordAuthResponse, ClientResponseError> {
        let mut opts: SendOptions = options.unwrap_or_default().into();
        opts.method = "POST".to_string();
        opts.body = Some(json!({
            "identity": username_or_email,
            "password": password
        }));

        let auth_data: RecordAuthResponse = self
            .client
            .send(&format!("{}/auth-with-password", self.base_collection_path()), opts)
            .await?;

        // Save to auth store
        self.client
            .auth_store()
            .save(&auth_data.token, Some(auth_data.record.clone()));

        Ok(auth_data)
    }

    /// Authenticate a single auth collection record with OAuth2 code.
    pub async fn auth_with_oauth2_code(
        &self,
        provider: &str,
        code: &str,
        code_verifier: &str,
        redirect_url: &str,
        create_data: Option<serde_json::Value>,
        options: Option<RecordOptions>,
    ) -> Result<RecordAuthResponse, ClientResponseError> {
        let mut opts: SendOptions = options.unwrap_or_default().into();
        opts.method = "POST".to_string();
        opts.body = Some(json!({
            "provider": provider,
            "code": code,
            "codeVerifier": code_verifier,
            "redirectURL": redirect_url,
            "createData": create_data
        }));

        let auth_data: RecordAuthResponse = self
            .client
            .send(&format!("{}/auth-with-oauth2", self.base_collection_path()), opts)
            .await?;

        self.client
            .auth_store()
            .save(&auth_data.token, Some(auth_data.record.clone()));

        Ok(auth_data)
    }

    /// Refreshes the current authenticated record instance and returns a new token and record data.
    pub async fn auth_refresh(
        &self,
        options: Option<RecordOptions>,
    ) -> Result<RecordAuthResponse, ClientResponseError> {
        let mut opts: SendOptions = options.unwrap_or_default().into();
        opts.method = "POST".to_string();

        let auth_data: RecordAuthResponse = self
            .client
            .send(&format!("{}/auth-refresh", self.base_collection_path()), opts)
            .await?;

        self.client
            .auth_store()
            .save(&auth_data.token, Some(auth_data.record.clone()));

        Ok(auth_data)
    }

    /// Sends auth record password reset request.
    pub async fn request_password_reset(&self, email: &str) -> Result<bool, ClientResponseError> {
        let options = SendOptions::post().with_body(json!({ "email": email }));
        self.client
            .send::<serde_json::Value>(
                &format!("{}/request-password-reset", self.base_collection_path()),
                options,
            )
            .await?;
        Ok(true)
    }

    /// Confirms auth record password reset request.
    pub async fn confirm_password_reset(
        &self,
        password_reset_token: &str,
        password: &str,
        password_confirm: &str,
    ) -> Result<bool, ClientResponseError> {
        let options = SendOptions::post().with_body(json!({
            "token": password_reset_token,
            "password": password,
            "passwordConfirm": password_confirm
        }));
        self.client
            .send::<serde_json::Value>(
                &format!("{}/confirm-password-reset", self.base_collection_path()),
                options,
            )
            .await?;
        Ok(true)
    }

    /// Sends auth record verification email request.
    pub async fn request_verification(&self, email: &str) -> Result<bool, ClientResponseError> {
        let options = SendOptions::post().with_body(json!({ "email": email }));
        self.client
            .send::<serde_json::Value>(
                &format!("{}/request-verification", self.base_collection_path()),
                options,
            )
            .await?;
        Ok(true)
    }

    /// Confirms auth record email verification request.
    pub async fn confirm_verification(
        &self,
        verification_token: &str,
    ) -> Result<bool, ClientResponseError> {
        let options = SendOptions::post().with_body(json!({ "token": verification_token }));
        self.client
            .send::<serde_json::Value>(
                &format!("{}/confirm-verification", self.base_collection_path()),
                options,
            )
            .await?;
        Ok(true)
    }

    /// Sends an email change request to the authenticated record model.
    pub async fn request_email_change(&self, new_email: &str) -> Result<bool, ClientResponseError> {
        let options = SendOptions::post().with_body(json!({ "newEmail": new_email }));
        self.client
            .send::<serde_json::Value>(
                &format!("{}/request-email-change", self.base_collection_path()),
                options,
            )
            .await?;
        Ok(true)
    }

    /// Confirms auth record's new email address.
    pub async fn confirm_email_change(
        &self,
        email_change_token: &str,
        password: &str,
    ) -> Result<bool, ClientResponseError> {
        let options = SendOptions::post().with_body(json!({
            "token": email_change_token,
            "password": password
        }));
        self.client
            .send::<serde_json::Value>(
                &format!("{}/confirm-email-change", self.base_collection_path()),
                options,
            )
            .await?;
        Ok(true)
    }

    /// Sends auth record OTP to the provided email.
    pub async fn request_otp(&self, email: &str) -> Result<OTPResponse, ClientResponseError> {
        let options = SendOptions::post().with_body(json!({ "email": email }));
        self.client
            .send(&format!("{}/request-otp", self.base_collection_path()), options)
            .await
    }

    /// Authenticate a single auth collection record via OTP.
    pub async fn auth_with_otp(
        &self,
        otp_id: &str,
        password: &str,
        options: Option<RecordOptions>,
    ) -> Result<RecordAuthResponse, ClientResponseError> {
        let mut opts: SendOptions = options.unwrap_or_default().into();
        opts.method = "POST".to_string();
        opts.body = Some(json!({
            "otpId": otp_id,
            "password": password
        }));

        let auth_data: RecordAuthResponse = self
            .client
            .send(&format!("{}/auth-with-otp", self.base_collection_path()), opts)
            .await?;

        self.client
            .auth_store()
            .save(&auth_data.token, Some(auth_data.record.clone()));

        Ok(auth_data)
    }

    /// Impersonate authenticates with the specified recordId and
    /// returns a new client with the received auth token in a memory store.
    pub async fn impersonate(
        &self,
        record_id: &str,
        duration: u64,
    ) -> Result<Arc<Client>, ClientResponseError> {
        let mut options = SendOptions::post().with_body(json!({ "duration": duration }));

        // Set the Authorization header from the current auth store
        let token = self.client.auth_store().token();
        if !token.is_empty() {
            options = options.with_header("Authorization", &token);
        }

        let path = format!(
            "{}/impersonate/{}",
            self.base_collection_path(),
            encode_uri_component(record_id)
        );

        let auth_data: RecordAuthResponse = self.client.send(&path, options).await?;

        // Create a new client with the impersonated auth state
        let new_client = Client::new(self.client.base_url());
        new_client
            .auth_store()
            .save(&auth_data.token, Some(auth_data.record));

        Ok(new_client)
    }
}

#[async_trait]
impl CrudService<RecordModel> for RecordService {
    fn base_crud_path(&self) -> String {
        format!("{}/records", self.base_collection_path())
    }

    fn client(&self) -> &Arc<Client> {
        &self.client
    }

    async fn get_full_list(
        &self,
        options: Option<FullListOptions>,
    ) -> Result<Vec<RecordModel>, ClientResponseError> {
        let batch = options.as_ref().and_then(|o| o.batch).unwrap_or(500);
        self._get_full_list(batch, options.map(|o| o.list))
            .await
    }

    async fn get_list(
        &self,
        page: u32,
        per_page: u32,
        options: Option<ListOptions>,
    ) -> Result<ListResult<RecordModel>, ClientResponseError> {
        let mut opts: SendOptions = options.unwrap_or_default().into();
        opts.method = "GET".to_string();
        opts.query
            .insert("page".to_string(), serde_json::json!(page));
        opts.query
            .insert("perPage".to_string(), serde_json::json!(per_page));
        self.client.send(&self.base_crud_path(), opts).await
    }

    async fn get_one(&self, id: &str) -> Result<RecordModel, ClientResponseError> {
        if id.is_empty() {
            return Err(ClientResponseError::not_found(
                &self.client.build_url(&format!("{}/", self.base_crud_path())),
                "Missing required record id.",
            ));
        }

        let options = SendOptions::get();
        let path = format!("{}/{}", self.base_crud_path(), encode_uri_component(id));
        self.client.send(&path, options).await
    }

    async fn create(
        &self,
        body_params: serde_json::Value,
    ) -> Result<RecordModel, ClientResponseError> {
        let options = SendOptions::post().with_body(body_params);
        self.client.send(&self.base_crud_path(), options).await
    }

    async fn update(
        &self,
        id: &str,
        body_params: serde_json::Value,
    ) -> Result<RecordModel, ClientResponseError> {
        let options = SendOptions::patch().with_body(body_params);
        let path = format!("{}/{}", self.base_crud_path(), encode_uri_component(id));
        let item: RecordModel = self.client.send(&path, options).await?;

        // Update auth store if the updated record matches the current auth record
        let auth_record = self.client.auth_store().record();
        if let Some(ref auth) = auth_record {
            if auth.id == item.id
                && (auth.collection_id == self.collection_id_or_name
                    || auth.collection_name == self.collection_id_or_name)
            {
                self.client
                    .auth_store()
                    .save(&self.client.auth_store().token(), Some(item.clone()));
            }
        }

        Ok(item)
    }

    async fn delete(&self, id: &str) -> Result<bool, ClientResponseError> {
        let options = SendOptions::delete();
        let path = format!("{}/{}", self.base_crud_path(), encode_uri_component(id));
        self.client
            .send::<serde_json::Value>(&path, options)
            .await?;

        // Clear auth store if the deleted record matches the current auth record
        let auth_record = self.client.auth_store().record();
        if let Some(ref auth) = auth_record {
            if auth.id == id
                && (auth.collection_id == self.collection_id_or_name
                    || auth.collection_name == self.collection_id_or_name)
            {
                self.client.auth_store().clear();
            }
        }

        Ok(true)
    }
}

impl BaseService for RecordService {
    fn client(&self) -> &Arc<Client> {
        &self.client
    }
}
