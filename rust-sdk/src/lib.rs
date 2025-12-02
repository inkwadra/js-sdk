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
pub use stores::{AuthRecord, BaseAuthStore};
pub use tools::dtos::{
    CollectionModel, ListResult, LogModel, RecordModel,
};
pub use tools::options::{
    CommonOptions, FileOptions, FullListOptions, ListOptions, RecordFullListOptions,
    RecordListOptions, RecordOptions, SendOptions,
};
