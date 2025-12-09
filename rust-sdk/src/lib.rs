//! PocketBase Rust SDK
//!
//! A Rust implementation of the PocketBase SDK providing similar functionality
//! to the official JavaScript SDK.

pub mod client;
pub mod client_response_error;
pub mod services;
pub mod stores;
pub mod tools;

pub use client::Client;
pub use client_response_error::ClientResponseError;
pub use services::{RealtimeMessage, RealtimeService};
pub use stores::{
    AsyncAuthStore, AsyncAuthStoreError, AsyncClearFunc, AsyncSaveFunc, AuthRecord, BaseAuthStore,
    LocalAuthStore, SerializedAuthState,
};
pub use tools::cookie::{
    cookie_parse, cookie_serialize, CookieError, CookiePriority, ParseOptions, SameSite,
    SerializeOptions,
};
pub use tools::dtos::{CollectionModel, ListResult, LogModel, RecordModel};
pub use tools::formdata::{FileData, FormDataBuilder, FormValue};
pub use tools::options::{
    CommonOptions, FileOptions, FullListOptions, ListOptions, RecordFullListOptions,
    RecordListOptions, RecordOptions, SendOptions,
};
pub use tools::refresh::{
    check_auth_status, create_auto_refresh_state, needs_refresh, reset_auto_refresh,
    AutoRefreshConfig, AutoRefreshState,
};
