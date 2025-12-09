//! FormData and file detection utilities
//!
//! Provides utilities for detecting file types and handling multipart form data
//! in a way similar to the JavaScript SDK.

use serde_json::Value;
use std::path::Path;

/// Represents a file to be uploaded.
#[derive(Debug, Clone)]
pub struct FileData {
    /// The file name.
    pub name: String,
    /// The file content as bytes.
    pub data: Vec<u8>,
    /// The MIME type of the file.
    pub mime_type: Option<String>,
}

impl FileData {
    /// Creates a new FileData instance.
    pub fn new(name: &str, data: Vec<u8>, mime_type: Option<&str>) -> Self {
        Self {
            name: name.to_string(),
            data,
            mime_type: mime_type.map(|s| s.to_string()),
        }
    }

    /// Creates a FileData from a file path.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read.
    pub fn from_path<P: AsRef<Path>>(path: P) -> std::io::Result<Self> {
        let path = path.as_ref();
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("file")
            .to_string();
        let data = std::fs::read(path)?;
        let mime_type = guess_mime_type(&name);

        Ok(Self {
            name,
            data,
            mime_type,
        })
    }
}

/// Guesses the MIME type based on the file extension.
pub fn guess_mime_type(filename: &str) -> Option<String> {
    let extension = Path::new(filename)
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_lowercase());

    match extension.as_deref() {
        // Images
        Some("jpg") | Some("jpeg") => Some("image/jpeg".to_string()),
        Some("png") => Some("image/png".to_string()),
        Some("gif") => Some("image/gif".to_string()),
        Some("webp") => Some("image/webp".to_string()),
        Some("svg") => Some("image/svg+xml".to_string()),
        Some("ico") => Some("image/x-icon".to_string()),
        Some("bmp") => Some("image/bmp".to_string()),
        Some("tiff") | Some("tif") => Some("image/tiff".to_string()),
        Some("avif") => Some("image/avif".to_string()),
        // Documents
        Some("pdf") => Some("application/pdf".to_string()),
        Some("doc") => Some("application/msword".to_string()),
        Some("docx") => Some(
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document".to_string(),
        ),
        Some("xls") => Some("application/vnd.ms-excel".to_string()),
        Some("xlsx") => {
            Some("application/vnd.openxmlformats-officedocument.spreadsheetml.sheet".to_string())
        }
        Some("ppt") => Some("application/vnd.ms-powerpoint".to_string()),
        Some("pptx") => Some(
            "application/vnd.openxmlformats-officedocument.presentationml.presentation".to_string(),
        ),
        // Text
        Some("txt") => Some("text/plain".to_string()),
        Some("html") | Some("htm") => Some("text/html".to_string()),
        Some("css") => Some("text/css".to_string()),
        Some("js") => Some("text/javascript".to_string()),
        Some("json") => Some("application/json".to_string()),
        Some("xml") => Some("application/xml".to_string()),
        Some("csv") => Some("text/csv".to_string()),
        Some("md") => Some("text/markdown".to_string()),
        // Archives
        Some("zip") => Some("application/zip".to_string()),
        Some("tar") => Some("application/x-tar".to_string()),
        Some("gz") | Some("gzip") => Some("application/gzip".to_string()),
        Some("rar") => Some("application/vnd.rar".to_string()),
        Some("7z") => Some("application/x-7z-compressed".to_string()),
        // Audio
        Some("mp3") => Some("audio/mpeg".to_string()),
        Some("wav") => Some("audio/wav".to_string()),
        Some("ogg") => Some("audio/ogg".to_string()),
        Some("m4a") => Some("audio/mp4".to_string()),
        Some("flac") => Some("audio/flac".to_string()),
        // Video
        Some("mp4") => Some("video/mp4".to_string()),
        Some("webm") => Some("video/webm".to_string()),
        Some("avi") => Some("video/x-msvideo".to_string()),
        Some("mov") => Some("video/quicktime".to_string()),
        Some("mkv") => Some("video/x-matroska".to_string()),
        // Default
        _ => Some("application/octet-stream".to_string()),
    }
}

/// Represents a form field value that can be either a simple value or a file.
#[derive(Debug, Clone)]
pub enum FormValue {
    /// A simple text/JSON value.
    Text(String),
    /// A file to upload.
    File(FileData),
    /// Multiple files to upload.
    Files(Vec<FileData>),
}

impl From<String> for FormValue {
    fn from(value: String) -> Self {
        FormValue::Text(value)
    }
}

impl From<&str> for FormValue {
    fn from(value: &str) -> Self {
        FormValue::Text(value.to_string())
    }
}

impl From<FileData> for FormValue {
    fn from(value: FileData) -> Self {
        FormValue::File(value)
    }
}

impl From<Vec<FileData>> for FormValue {
    fn from(value: Vec<FileData>) -> Self {
        FormValue::Files(value)
    }
}

impl From<Value> for FormValue {
    fn from(value: Value) -> Self {
        match value {
            Value::String(s) => FormValue::Text(s),
            other => FormValue::Text(serde_json::to_string(&other).unwrap_or_default()),
        }
    }
}

/// A builder for multipart form data.
#[derive(Debug, Default)]
pub struct FormDataBuilder {
    fields: Vec<(String, FormValue)>,
}

impl FormDataBuilder {
    /// Creates a new FormDataBuilder.
    pub fn new() -> Self {
        Self { fields: Vec::new() }
    }

    /// Adds a text field to the form.
    pub fn text(mut self, name: &str, value: impl Into<String>) -> Self {
        self.fields
            .push((name.to_string(), FormValue::Text(value.into())));
        self
    }

    /// Adds a JSON value field to the form.
    /// Complex objects will be serialized and sent as @jsonPayload.
    pub fn json(mut self, name: &str, value: Value) -> Self {
        match &value {
            Value::Object(_) | Value::Array(_) => {
                // For complex types, use @jsonPayload format like the JS SDK
                let mut payload = serde_json::Map::new();
                payload.insert(name.to_string(), value);
                self.fields.push((
                    "@jsonPayload".to_string(),
                    FormValue::Text(serde_json::to_string(&payload).unwrap_or_default()),
                ));
            }
            _ => {
                self.fields.push((name.to_string(), value.into()));
            }
        }
        self
    }

    /// Adds a file field to the form.
    pub fn file(mut self, name: &str, file: FileData) -> Self {
        self.fields.push((name.to_string(), FormValue::File(file)));
        self
    }

    /// Adds multiple files to a field.
    pub fn files(mut self, name: &str, files: Vec<FileData>) -> Self {
        self.fields
            .push((name.to_string(), FormValue::Files(files)));
        self
    }

    /// Returns the fields for building a multipart request.
    pub fn build(self) -> Vec<(String, FormValue)> {
        self.fields
    }

    /// Checks if any field contains a file.
    pub fn has_files(&self) -> bool {
        self.fields
            .iter()
            .any(|(_, v)| matches!(v, FormValue::File(_) | FormValue::Files(_)))
    }
}

/// Checks if a JSON body contains any file-like fields.
///
/// In Rust, we don't have the same File/Blob detection as in JavaScript,
/// so this function checks for specific patterns in the JSON that might
/// indicate file data (e.g., base64 encoded strings with data URI scheme).
pub fn has_file_field(body: &Value) -> bool {
    match body {
        Value::Object(map) => {
            for value in map.values() {
                if is_file_like_value(value) {
                    return true;
                }
                if let Value::Array(arr) = value {
                    for v in arr {
                        if is_file_like_value(v) {
                            return true;
                        }
                    }
                }
            }
            false
        }
        _ => false,
    }
}

/// Checks if a value looks like file data (e.g., data URI).
fn is_file_like_value(value: &Value) -> bool {
    if let Value::String(s) = value {
        // Check for data URI scheme which is often used for file uploads
        s.starts_with("data:")
    } else {
        false
    }
}

/// Infers the type of a form data value, similar to the JS SDK's inferFormDataValue.
///
/// Converts string values to their appropriate JSON types:
/// - "true" becomes `true`
/// - "false" becomes `false`
/// - Numeric strings become numbers (if exact representation matches)
/// - Other strings remain as strings
pub fn infer_form_data_value(value: &str) -> Value {
    if value == "true" {
        return Value::Bool(true);
    }

    if value == "false" {
        return Value::Bool(false);
    }

    // Check if it's a number
    if let Some(first_char) = value.chars().next() {
        if first_char == '-' || first_char.is_ascii_digit() {
            // Only allow characters that could be in a number
            if value
                .chars()
                .all(|c| c == '-' || c == '.' || c.is_ascii_digit())
            {
                // Try parsing as a number
                if let Ok(num) = value.parse::<i64>() {
                    // Verify exact string representation matches
                    if num.to_string() == value {
                        return Value::Number(num.into());
                    }
                }
                if let Ok(num) = value.parse::<f64>() {
                    // Verify exact string representation matches
                    if num.to_string() == value {
                        if let Some(n) = serde_json::Number::from_f64(num) {
                            return Value::Number(n);
                        }
                    }
                }
            }
        }
    }

    Value::String(value.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_guess_mime_type() {
        assert_eq!(guess_mime_type("test.jpg"), Some("image/jpeg".to_string()));
        assert_eq!(guess_mime_type("test.png"), Some("image/png".to_string()));
        assert_eq!(
            guess_mime_type("test.pdf"),
            Some("application/pdf".to_string())
        );
        assert_eq!(
            guess_mime_type("unknown.xyz"),
            Some("application/octet-stream".to_string())
        );
    }

    #[test]
    fn test_infer_form_data_value() {
        assert_eq!(infer_form_data_value("true"), Value::Bool(true));
        assert_eq!(infer_form_data_value("false"), Value::Bool(false));
        assert_eq!(infer_form_data_value("42"), Value::Number(42.into()));
        assert_eq!(infer_form_data_value("-123"), Value::Number((-123).into()));
        assert_eq!(
            infer_form_data_value("hello"),
            Value::String("hello".to_string())
        );
        // Scientific notation should remain as string
        assert_eq!(
            infer_form_data_value("1e10"),
            Value::String("1e10".to_string())
        );
        // Leading zeros should remain as string
        assert_eq!(
            infer_form_data_value("0001"),
            Value::String("0001".to_string())
        );
    }

    #[test]
    fn test_form_data_builder() {
        let form = FormDataBuilder::new()
            .text("name", "test")
            .json("count", serde_json::json!(42))
            .build();

        assert_eq!(form.len(), 2);
    }

    #[test]
    fn test_has_files() {
        let form_without_files = FormDataBuilder::new().text("name", "test");
        assert!(!form_without_files.has_files());

        let form_with_files = FormDataBuilder::new().text("name", "test").file(
            "avatar",
            FileData::new("avatar.png", vec![1, 2, 3], Some("image/png")),
        );
        assert!(form_with_files.has_files());
    }
}
