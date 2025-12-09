//! Auto-refresh token management
//!
//! This module provides utilities for automatically refreshing authentication tokens
//! before they expire, ensuring seamless authenticated API requests.

use crate::client::Client;
use crate::tools::jwt::is_token_expired;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

/// Type alias for async refresh/reauthenticate functions.
pub type AsyncAuthFunc = Box<
    dyn Fn() -> Pin<
            Box<dyn Future<Output = Result<(), Box<dyn std::error::Error + Send + Sync>>> + Send>,
        > + Send
        + Sync,
>;

/// Configuration for auto-refresh behavior.
#[derive(Debug, Clone)]
pub struct AutoRefreshConfig {
    /// The number of seconds before token expiration to trigger a refresh.
    pub threshold: u64,
}

impl Default for AutoRefreshConfig {
    fn default() -> Self {
        Self {
            threshold: 300, // 5 minutes before expiration
        }
    }
}

/// Auto-refresh state holder.
pub struct AutoRefreshState {
    /// Original model ID that was authenticated.
    pub original_model_id: Option<String>,

    /// Original collection ID.
    pub original_collection_id: Option<String>,

    /// Whether auto-refresh is active.
    pub is_active: bool,
}

impl Default for AutoRefreshState {
    fn default() -> Self {
        Self {
            original_model_id: None,
            original_collection_id: None,
            is_active: false,
        }
    }
}

/// Resets any previous auto-refresh registration for the client.
///
/// This function should be called when you want to disable auto-refresh
/// or before registering a new auto-refresh configuration.
pub fn reset_auto_refresh(state: &mut AutoRefreshState) {
    state.is_active = false;
    state.original_model_id = None;
    state.original_collection_id = None;
}

/// Checks if the current token needs to be refreshed based on the threshold.
///
/// # Arguments
///
/// * `token` - The JWT token to check.
/// * `threshold` - The number of seconds before expiration to consider the token as needing refresh.
///
/// # Returns
///
/// Returns `true` if the token is going to expire within the threshold period.
pub fn needs_refresh(token: &str, threshold: u64) -> bool {
    if token.is_empty() {
        return false;
    }

    // Check if token is still valid but going to expire soon
    let is_valid = !is_token_expired(token, 0);
    let expires_soon = is_token_expired(token, threshold);

    is_valid && expires_soon
}

/// Determines if a refresh or reauthentication is required before making a request.
///
/// # Arguments
///
/// * `client` - The PocketBase client.
/// * `threshold` - The number of seconds before expiration to trigger refresh.
///
/// # Returns
///
/// A tuple of (needs_refresh, needs_reauthenticate).
pub fn check_auth_status(client: &Arc<Client>, threshold: u64) -> (bool, bool) {
    let token = client.auth_store().token();

    if token.is_empty() {
        return (false, false);
    }

    let is_valid = !is_token_expired(&token, 0);

    if !is_valid {
        // Token is completely expired, need to reauthenticate
        return (false, true);
    }

    // Check if token is going to expire soon
    if is_token_expired(&token, threshold) {
        return (true, false);
    }

    (false, false)
}

/// Validates that the current auth store state matches the original authenticated user.
///
/// This is used to detect if a different user has been authenticated, which should
/// trigger a reset of the auto-refresh.
///
/// # Arguments
///
/// * `client` - The PocketBase client.
/// * `state` - The auto-refresh state.
///
/// # Returns
///
/// Returns `true` if the current auth matches the original, `false` otherwise.
pub fn validate_auth_matches(client: &Arc<Client>, state: &AutoRefreshState) -> bool {
    let record = client.auth_store().record();
    let token = client.auth_store().token();

    // If token is empty, auth has been cleared
    if token.is_empty() {
        return false;
    }

    match &record {
        Some(model) => {
            // Check if model ID matches
            if let Some(ref original_id) = state.original_model_id {
                if &model.id != original_id {
                    return false;
                }
            }

            // Check if collection ID matches (if both are set)
            if let (Some(ref original_coll), collection_id) =
                (&state.original_collection_id, &model.collection_id)
            {
                if original_coll != collection_id {
                    return false;
                }
            }

            true
        }
        None => state.original_model_id.is_none(),
    }
}

/// Creates an auto-refresh state from the current client auth state.
///
/// # Arguments
///
/// * `client` - The PocketBase client.
///
/// # Returns
///
/// A new `AutoRefreshState` initialized from the current auth state.
pub fn create_auto_refresh_state(client: &Arc<Client>) -> AutoRefreshState {
    let record = client.auth_store().record();

    AutoRefreshState {
        original_model_id: record.as_ref().map(|r| r.id.clone()),
        original_collection_id: record.as_ref().map(|r| r.collection_id.clone()),
        is_active: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_needs_refresh_empty_token() {
        assert!(!needs_refresh("", 300));
    }

    #[test]
    fn test_reset_auto_refresh() {
        let mut state = AutoRefreshState {
            original_model_id: Some("test".to_string()),
            original_collection_id: Some("users".to_string()),
            is_active: true,
        };

        reset_auto_refresh(&mut state);

        assert!(!state.is_active);
        assert!(state.original_model_id.is_none());
        assert!(state.original_collection_id.is_none());
    }

    #[test]
    fn test_auto_refresh_state_default() {
        let state = AutoRefreshState::default();
        assert!(!state.is_active);
        assert!(state.original_model_id.is_none());
        assert!(state.original_collection_id.is_none());
    }

    #[test]
    fn test_auto_refresh_config_default() {
        let config = AutoRefreshConfig::default();
        assert_eq!(config.threshold, 300);
    }
}
