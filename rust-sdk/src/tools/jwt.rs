//! JWT token utilities

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use serde::Deserialize;
use std::collections::HashMap;

/// JWT token payload.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct TokenPayload {
    /// Token type (e.g., "auth").
    #[serde(default, rename = "type")]
    pub token_type: Option<String>,

    /// Expiration timestamp.
    #[serde(default)]
    pub exp: Option<u64>,

    /// Collection ID.
    #[serde(default, rename = "collectionId")]
    pub collection_id: Option<String>,

    /// Record/User ID.
    #[serde(default)]
    pub id: Option<String>,

    /// Additional claims.
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Returns JWT token's payload data.
pub fn get_token_payload(token: &str) -> TokenPayload {
    if token.is_empty() {
        return TokenPayload::default();
    }

    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return TokenPayload::default();
    }

    let payload_part = parts[1];

    // Decode base64
    let decoded = match URL_SAFE_NO_PAD.decode(payload_part) {
        Ok(d) => d,
        Err(_) => {
            // Try with standard base64 (replacing + and /)
            let fixed = payload_part
                .replace('-', "+")
                .replace('_', "/");
            let padded = match fixed.len() % 4 {
                2 => format!("{}==", fixed),
                3 => format!("{}=", fixed),
                _ => fixed,
            };
            match base64::engine::general_purpose::STANDARD.decode(&padded) {
                Ok(d) => d,
                Err(_) => return TokenPayload::default(),
            }
        }
    };

    // Parse JSON
    serde_json::from_slice(&decoded).unwrap_or_default()
}

/// Checks whether a JWT token is expired or not.
/// Tokens without `exp` payload key are considered valid.
/// Tokens with empty payload (eg. invalid token strings) are considered expired.
pub fn is_token_expired(token: &str, expiration_threshold: u64) -> bool {
    let payload = get_token_payload(token);

    // Empty payload means invalid/expired token
    if payload.exp.is_none() && payload.token_type.is_none() && payload.id.is_none() {
        return true;
    }

    // No exp claim means token doesn't expire
    match payload.exp {
        None => false,
        Some(exp) => {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            
            exp.saturating_sub(expiration_threshold) <= now
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_token_payload_empty() {
        let payload = get_token_payload("");
        assert!(payload.exp.is_none());
    }

    #[test]
    fn test_get_token_payload_invalid() {
        let payload = get_token_payload("invalid.token");
        assert!(payload.exp.is_none());
    }

    #[test]
    fn test_is_token_expired_empty() {
        assert!(is_token_expired("", 0));
    }
}
