//! Cookie parsing and serialization utilities.
//!
//! Simple cookie parse and serialize utilities mostly based on the
//! node module <https://github.com/jshttp/cookie>.

use std::collections::HashMap;

/// Options for parsing cookies.
#[derive(Debug, Clone, Default)]
pub struct ParseOptions {
    /// Custom decode function. If not provided, URL decoding is used.
    pub decode: Option<fn(&str) -> String>,
}

/// Options for serializing cookies.
#[derive(Debug, Clone, Default)]
pub struct SerializeOptions {
    /// Custom encode function. If not provided, URL encoding is used.
    pub encode: Option<fn(&str) -> String>,
    /// Max-Age in seconds.
    pub max_age: Option<i64>,
    /// Domain attribute.
    pub domain: Option<String>,
    /// Path attribute.
    pub path: Option<String>,
    /// Expires date (RFC 7231 format).
    pub expires: Option<chrono::DateTime<chrono::Utc>>,
    /// HttpOnly flag.
    pub http_only: bool,
    /// Secure flag.
    pub secure: bool,
    /// Priority attribute (Low, Medium, High).
    pub priority: Option<CookiePriority>,
    /// SameSite attribute.
    pub same_site: Option<SameSite>,
}

/// Cookie priority levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CookiePriority {
    Low,
    Medium,
    High,
}

/// SameSite attribute values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SameSite {
    Strict,
    Lax,
    None,
}

/// Parses the given cookie header string into a HashMap.
///
/// The HashMap has the various cookies as keys (names) => values.
///
/// # Arguments
///
/// * `str` - The cookie header string to parse
/// * `options` - Optional parsing options
///
/// # Returns
///
/// A HashMap containing the parsed cookie name-value pairs.
///
/// # Example
///
/// ```
/// use pocketbase_sdk::tools::cookie::cookie_parse;
///
/// let cookies = cookie_parse("foo=bar; baz=qux", None);
/// assert_eq!(cookies.get("foo"), Some(&"bar".to_string()));
/// assert_eq!(cookies.get("baz"), Some(&"qux".to_string()));
/// ```
pub fn cookie_parse(str: &str, options: Option<ParseOptions>) -> HashMap<String, String> {
    let mut result: HashMap<String, String> = HashMap::new();

    if str.is_empty() {
        return result;
    }

    let opts = options.unwrap_or_default();
    let decode = opts.decode.unwrap_or(default_decode);

    let mut index = 0;
    let chars: Vec<char> = str.chars().collect();
    let len = chars.len();

    while index < len {
        // Find the next '='
        let eq_idx = match str[index..].find('=') {
            Some(i) => index + i,
            None => break, // no more cookie pairs
        };

        // Find the next ';'
        let end_idx = match str[index..].find(';') {
            Some(i) => {
                let idx = index + i;
                if idx < eq_idx {
                    // Backtrack on prior semicolon
                    index = str[..eq_idx].rfind(';').map(|i| i + 1).unwrap_or(0);
                    continue;
                }
                idx
            }
            None => len,
        };

        let key = str[index..eq_idx].trim().to_string();

        // Only assign once
        if !result.contains_key(&key) && !key.is_empty() {
            let mut val = str[(eq_idx + 1)..end_idx].trim().to_string();

            // Remove quotes if present
            if val.starts_with('"') && val.ends_with('"') && val.len() >= 2 {
                val = val[1..val.len() - 1].to_string();
            }

            // Decode the value
            result.insert(key, decode(&val));
        }

        index = end_idx + 1;
    }

    result
}

/// Serializes a name-value pair into a cookie string suitable for HTTP headers.
///
/// # Arguments
///
/// * `name` - The cookie name
/// * `val` - The cookie value
/// * `options` - Optional serialization options
///
/// # Returns
///
/// A `Result` containing the serialized cookie string or an error.
///
/// # Example
///
/// ```
/// use pocketbase_sdk::tools::cookie::{cookie_serialize, SerializeOptions};
///
/// let cookie = cookie_serialize("foo", "bar", None).unwrap();
/// assert_eq!(cookie, "foo=bar");
///
/// let opts = SerializeOptions {
///     http_only: true,
///     secure: true,
///     ..Default::default()
/// };
/// let cookie = cookie_serialize("foo", "bar", Some(opts)).unwrap();
/// assert_eq!(cookie, "foo=bar; HttpOnly; Secure");
/// ```
pub fn cookie_serialize(
    name: &str,
    val: &str,
    options: Option<SerializeOptions>,
) -> Result<String, CookieError> {
    let opts = options.unwrap_or_default();
    let encode = opts.encode.unwrap_or(default_encode);

    // Validate name
    if !is_valid_field_content(name) {
        return Err(CookieError::InvalidName(name.to_string()));
    }

    let value = encode(val);

    // Validate encoded value
    if !is_valid_field_content(&value) {
        return Err(CookieError::InvalidValue(val.to_string()));
    }

    let mut result = format!("{}={}", name, value);

    // Max-Age
    if let Some(max_age) = opts.max_age {
        if max_age.is_nan() {
            return Err(CookieError::InvalidMaxAge);
        }
        result.push_str(&format!("; Max-Age={}", max_age));
    }

    // Domain
    if let Some(ref domain) = opts.domain {
        if !is_valid_field_content(domain) {
            return Err(CookieError::InvalidDomain(domain.clone()));
        }
        result.push_str(&format!("; Domain={}", domain));
    }

    // Path
    if let Some(ref path) = opts.path {
        if !is_valid_field_content(path) {
            return Err(CookieError::InvalidPath(path.clone()));
        }
        result.push_str(&format!("; Path={}", path));
    }

    // Expires
    if let Some(expires) = opts.expires {
        result.push_str(&format!(
            "; Expires={}",
            expires.format("%a, %d %b %Y %H:%M:%S GMT")
        ));
    }

    // HttpOnly
    if opts.http_only {
        result.push_str("; HttpOnly");
    }

    // Secure
    if opts.secure {
        result.push_str("; Secure");
    }

    // Priority
    if let Some(priority) = opts.priority {
        match priority {
            CookiePriority::Low => result.push_str("; Priority=Low"),
            CookiePriority::Medium => result.push_str("; Priority=Medium"),
            CookiePriority::High => result.push_str("; Priority=High"),
        }
    }

    // SameSite
    if let Some(same_site) = opts.same_site {
        match same_site {
            SameSite::Strict => result.push_str("; SameSite=Strict"),
            SameSite::Lax => result.push_str("; SameSite=Lax"),
            SameSite::None => result.push_str("; SameSite=None"),
        }
    }

    Ok(result)
}

/// Cookie serialization errors.
#[derive(Debug, Clone, thiserror::Error)]
pub enum CookieError {
    #[error("invalid cookie name: {0}")]
    InvalidName(String),
    #[error("invalid cookie value: {0}")]
    InvalidValue(String),
    #[error("invalid max-age value")]
    InvalidMaxAge,
    #[error("invalid domain: {0}")]
    InvalidDomain(String),
    #[error("invalid path: {0}")]
    InvalidPath(String),
    #[error("invalid expires date")]
    InvalidExpires,
    #[error("invalid priority")]
    InvalidPriority,
    #[error("invalid same-site value")]
    InvalidSameSite,
}

/// Checks if a string is valid field content per RFC 7230 sec 3.2.
///
/// field-content = field-vchar [ 1*( SP / HTAB ) field-vchar ]
/// field-vchar   = VCHAR / obs-text
/// obs-text      = %x80-FF
fn is_valid_field_content(s: &str) -> bool {
    if s.is_empty() {
        return true;
    }

    for c in s.chars() {
        let code = c as u32;
        // Valid: tab (0x09), space (0x20), printable ASCII (0x21-0x7E), or obs-text (0x80-0xFF)
        if !(code == 0x09 || (code >= 0x20 && code <= 0x7E) || (code >= 0x80 && code <= 0xFF)) {
            return false;
        }
    }

    true
}

/// Default URL-decode function.
/// Optimized to skip decoding when no '%' is present.
fn default_decode(val: &str) -> String {
    if !val.contains('%') {
        return val.to_string();
    }

    // URL decode the value
    percent_decode(val)
}

/// Default URL-encode function.
fn default_encode(val: &str) -> String {
    percent_encode(val)
}

/// Simple percent-decoding implementation.
fn percent_decode(input: &str) -> String {
    let mut result = String::new();
    let mut chars = input.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '%' {
            let hex: String = chars.by_ref().take(2).collect();
            if hex.len() == 2 {
                if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                    result.push(byte as char);
                    continue;
                }
            }
            // Invalid encoding, keep original
            result.push('%');
            result.push_str(&hex);
        } else {
            result.push(c);
        }
    }

    result
}

/// Simple percent-encoding implementation for cookie values.
fn percent_encode(input: &str) -> String {
    let mut result = String::new();

    for c in input.chars() {
        match c {
            // Unreserved characters (don't need encoding)
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => {
                result.push(c);
            }
            // Special cookie-safe characters
            '!' | '#' | '$' | '&' | '\'' | '(' | ')' | '*' | '+' | '/' | ':' | '<' | '=' | '>'
            | '?' | '@' | '[' | ']' | '^' | '`' | '{' | '|' | '}' => {
                result.push(c);
            }
            // Everything else gets encoded
            _ => {
                for byte in c.to_string().as_bytes() {
                    result.push_str(&format!("%{:02X}", byte));
                }
            }
        }
    }

    result
}

/// Trait extension for i64 to provide is_nan check (always false for integers).
trait IsNan {
    fn is_nan(&self) -> bool;
}

impl IsNan for i64 {
    fn is_nan(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cookie_parse_simple() {
        let cookies = cookie_parse("foo=bar", None);
        assert_eq!(cookies.get("foo"), Some(&"bar".to_string()));
    }

    #[test]
    fn test_cookie_parse_multiple() {
        let cookies = cookie_parse("foo=bar; baz=qux", None);
        assert_eq!(cookies.get("foo"), Some(&"bar".to_string()));
        assert_eq!(cookies.get("baz"), Some(&"qux".to_string()));
    }

    #[test]
    fn test_cookie_parse_quoted() {
        let cookies = cookie_parse("foo=\"bar baz\"", None);
        assert_eq!(cookies.get("foo"), Some(&"bar baz".to_string()));
    }

    #[test]
    fn test_cookie_parse_empty() {
        let cookies = cookie_parse("", None);
        assert!(cookies.is_empty());
    }

    #[test]
    fn test_cookie_parse_encoded() {
        let cookies = cookie_parse("foo=hello%20world", None);
        assert_eq!(cookies.get("foo"), Some(&"hello world".to_string()));
    }

    #[test]
    fn test_cookie_serialize_simple() {
        let cookie = cookie_serialize("foo", "bar", None).unwrap();
        assert_eq!(cookie, "foo=bar");
    }

    #[test]
    fn test_cookie_serialize_with_options() {
        let opts = SerializeOptions {
            http_only: true,
            secure: true,
            path: Some("/".to_string()),
            same_site: Some(SameSite::Strict),
            ..Default::default()
        };
        let cookie = cookie_serialize("foo", "bar", Some(opts)).unwrap();
        assert!(cookie.contains("foo=bar"));
        assert!(cookie.contains("; HttpOnly"));
        assert!(cookie.contains("; Secure"));
        assert!(cookie.contains("; Path=/"));
        assert!(cookie.contains("; SameSite=Strict"));
    }

    #[test]
    fn test_cookie_serialize_max_age() {
        let opts = SerializeOptions {
            max_age: Some(3600),
            ..Default::default()
        };
        let cookie = cookie_serialize("foo", "bar", Some(opts)).unwrap();
        assert!(cookie.contains("; Max-Age=3600"));
    }

    #[test]
    fn test_cookie_serialize_priority() {
        let opts = SerializeOptions {
            priority: Some(CookiePriority::High),
            ..Default::default()
        };
        let cookie = cookie_serialize("foo", "bar", Some(opts)).unwrap();
        assert!(cookie.contains("; Priority=High"));
    }

    #[test]
    fn test_percent_encode_decode() {
        let original = "hello world!";
        let encoded = percent_encode(original);
        let decoded = percent_decode(&encoded);
        assert_eq!(decoded, original);
    }
}
