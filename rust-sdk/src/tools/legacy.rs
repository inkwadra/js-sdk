//! Legacy options normalization helper
//!
//! This module provides utilities for handling legacy API options arguments.

use super::options::SendOptions;
use serde_json::Value;
use std::collections::HashMap;

/// Normalizes legacy options arguments for backward compatibility.
///
/// This function handles the case where older API patterns passed body and query
/// as separate arguments instead of as part of a unified options object.
///
/// # Arguments
///
/// * `legacy_warn` - Warning message to print when legacy pattern is detected
/// * `base_options` - The base SendOptions to merge into
/// * `body_or_options` - Either a body object or a new-style options object
/// * `query` - Optional query parameters (indicates legacy usage if present)
///
/// # Returns
///
/// Merged `SendOptions` with all parameters properly applied.
///
/// # Example
///
/// ```rust
/// use pocketbase_sdk::tools::legacy::normalize_legacy_options_args;
/// use pocketbase_sdk::tools::options::SendOptions;
/// use serde_json::json;
///
/// // Modern usage (single options object)
/// let base = SendOptions::post();
/// let options = Some(json!({"body": {"name": "test"}}));
/// let result = normalize_legacy_options_args("", base, options, None);
///
/// // Legacy usage (separate body and query)
/// let base = SendOptions::post();
/// let body = Some(json!({"name": "test"}));
/// let query = Some(json!({"expand": "user"}));
/// let result = normalize_legacy_options_args(
///     "Deprecated: use options object instead",
///     base,
///     body,
///     query,
/// );
/// ```
pub fn normalize_legacy_options_args(
    legacy_warn: &str,
    mut base_options: SendOptions,
    body_or_options: Option<Value>,
    query: Option<Value>,
) -> SendOptions {
    let has_body_or_options = body_or_options.is_some();
    let has_query = query.is_some();

    if !has_query && !has_body_or_options {
        return base_options;
    }

    if has_query {
        // Legacy pattern detected
        if !legacy_warn.is_empty() {
            eprintln!("{}", legacy_warn);
        }

        // Merge body
        if let Some(body) = body_or_options {
            if let Some(existing) = base_options.body.as_mut() {
                if let (Some(existing_obj), Some(body_obj)) =
                    (existing.as_object_mut(), body.as_object())
                {
                    for (k, v) in body_obj {
                        existing_obj.insert(k.clone(), v.clone());
                    }
                }
            } else {
                base_options.body = Some(body);
            }
        }

        // Merge query
        if let Some(query_val) = query {
            if let Some(query_obj) = query_val.as_object() {
                for (k, v) in query_obj {
                    base_options.query.insert(k.clone(), v.clone());
                }
            }
        }

        return base_options;
    }

    // Modern pattern: body_or_options is the new options object
    if let Some(options) = body_or_options {
        merge_send_options(&mut base_options, &options);
    }

    base_options
}

/// Merges a JSON value representing options into a SendOptions struct.
fn merge_send_options(base: &mut SendOptions, options: &Value) {
    if let Some(obj) = options.as_object() {
        // Merge method
        if let Some(Value::String(method)) = obj.get("method") {
            base.method = method.clone();
        }

        // Merge headers
        if let Some(Value::Object(headers)) = obj.get("headers") {
            for (k, v) in headers {
                if let Value::String(val) = v {
                    base.headers.insert(k.clone(), val.clone());
                }
            }
        }

        // Merge body
        if let Some(body) = obj.get("body") {
            if let Some(existing) = base.body.as_mut() {
                if let (Some(existing_obj), Some(body_obj)) =
                    (existing.as_object_mut(), body.as_object())
                {
                    for (k, v) in body_obj {
                        existing_obj.insert(k.clone(), v.clone());
                    }
                }
            } else {
                base.body = Some(body.clone());
            }
        }

        // Merge query
        if let Some(Value::Object(query)) = obj.get("query") {
            for (k, v) in query {
                base.query.insert(k.clone(), v.clone());
            }
        }

        // Merge request_key
        if let Some(Value::String(request_key)) = obj.get("requestKey") {
            base.request_key = Some(request_key.clone());
        }
    }
}

/// Normalizes unknown query parameters by converting them to a standard format.
///
/// This is useful for handling options that may contain mixed parameter formats.
pub fn normalize_unknown_query_params(options: &mut SendOptions) {
    // Convert any non-standard query params to proper format
    let mut normalized: HashMap<String, Value> = HashMap::new();

    for (key, value) in &options.query {
        // Ensure arrays are properly formatted
        if value.is_array() {
            normalized.insert(key.clone(), value.clone());
        } else {
            normalized.insert(key.clone(), value.clone());
        }
    }

    options.query = normalized;
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_normalize_no_options() {
        let base = SendOptions::get();
        let result = normalize_legacy_options_args("", base, None, None);

        assert_eq!(result.method, "GET");
        assert!(result.body.is_none());
    }

    #[test]
    fn test_normalize_modern_options() {
        let base = SendOptions::post();
        let options = json!({
            "body": {"name": "test"},
            "query": {"expand": "user"}
        });

        let result = normalize_legacy_options_args("", base, Some(options), None);

        assert!(result.body.is_some());
        assert_eq!(result.body.unwrap()["name"], "test");
        assert_eq!(result.query["expand"], "user");
    }

    #[test]
    fn test_normalize_legacy_options() {
        let base = SendOptions::post();
        let body = json!({"name": "test"});
        let query = json!({"expand": "user"});

        let result = normalize_legacy_options_args("", base, Some(body), Some(query));

        assert!(result.body.is_some());
        assert_eq!(result.body.unwrap()["name"], "test");
        assert_eq!(result.query["expand"], "user");
    }

    #[test]
    fn test_merge_send_options() {
        let mut base = SendOptions::get();
        base.headers
            .insert("X-Custom".to_string(), "original".to_string());

        let options = json!({
            "method": "POST",
            "headers": {
                "Authorization": "Bearer token"
            },
            "body": {"data": 123}
        });

        merge_send_options(&mut base, &options);

        assert_eq!(base.method, "POST");
        assert_eq!(base.headers.get("X-Custom").unwrap(), "original");
        assert_eq!(base.headers.get("Authorization").unwrap(), "Bearer token");
        assert!(base.body.is_some());
    }
}
