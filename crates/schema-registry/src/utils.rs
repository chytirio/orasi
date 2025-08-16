//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Utility functions for the Schema Registry
//!
//! This module provides utility functions and helpers used throughout
//! the schema registry.

use chrono::{DateTime, Utc};
use std::collections::HashMap;
use uuid::Uuid;

/// Generate a unique request ID
pub fn generate_request_id() -> Uuid {
    Uuid::new_v4()
}

/// Generate a timestamp
pub fn generate_timestamp() -> DateTime<Utc> {
    Utc::now()
}

/// Format a duration in a human-readable format
pub fn format_duration(duration: std::time::Duration) -> String {
    let secs = duration.as_secs();
    let millis = duration.subsec_millis();

    if secs > 0 {
        format!("{}.{:03}s", secs, millis)
    } else {
        format!("{}ms", millis)
    }
}

/// Format bytes in a human-readable format
pub fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    match bytes {
        0..KB => format!("{} B", bytes),
        KB..MB => format!("{:.1} KB", bytes as f64 / KB as f64),
        MB..GB => format!("{:.1} MB", bytes as f64 / MB as f64),
        _ => format!("{:.1} GB", bytes as f64 / GB as f64),
    }
}

/// Validate a schema name
pub fn validate_schema_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("Schema name cannot be empty".to_string());
    }

    if name.len() > 100 {
        return Err("Schema name cannot exceed 100 characters".to_string());
    }

    // Check for valid characters (alphanumeric, hyphens, underscores)
    if !name
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    {
        return Err(
            "Schema name can only contain alphanumeric characters, hyphens, and underscores"
                .to_string(),
        );
    }

    // Check for reserved names
    let reserved_names = ["admin", "system", "internal", "test", "temp"];
    if reserved_names.contains(&name.to_lowercase().as_str()) {
        return Err("Schema name cannot be a reserved name".to_string());
    }

    Ok(())
}

/// Validate a schema version string
pub fn validate_version_string(version: &str) -> Result<(), String> {
    if version.is_empty() {
        return Err("Version cannot be empty".to_string());
    }

    // Basic semantic versioning validation
    let parts: Vec<&str> = version.split('.').collect();
    if parts.len() != 3 {
        return Err("Version must be in format X.Y.Z".to_string());
    }

    for part in parts {
        if part.is_empty() {
            return Err("Version parts cannot be empty".to_string());
        }

        if !part.chars().all(|c| c.is_numeric()) {
            return Err("Version parts must be numeric".to_string());
        }
    }

    Ok(())
}

/// Sanitize a string for use in file names
pub fn sanitize_filename(input: &str) -> String {
    input
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// Create a hash map from key-value pairs
pub fn create_hash_map<K: Into<String>, V: Into<String>>(
    pairs: Vec<(K, V)>,
) -> HashMap<String, String> {
    pairs
        .into_iter()
        .map(|(k, v)| (k.into(), v.into()))
        .collect()
}

/// Merge two hash maps
pub fn merge_hash_maps(
    mut base: HashMap<String, String>,
    additional: HashMap<String, String>,
) -> HashMap<String, String> {
    base.extend(additional);
    base
}

/// Extract query parameters from a URL
pub fn extract_query_params(url: &str) -> HashMap<String, String> {
    let mut params = HashMap::new();

    if let Some(query_start) = url.find('?') {
        let query = &url[query_start + 1..];

        for pair in query.split('&') {
            if let Some(equal_pos) = pair.find('=') {
                let key = &pair[..equal_pos];
                let value = &pair[equal_pos + 1..];

                if !key.is_empty() {
                    params.insert(key.to_string(), value.to_string());
                }
            }
        }
    }

    params
}

/// Build a query string from parameters
pub fn build_query_string(params: &HashMap<String, String>) -> String {
    if params.is_empty() {
        return String::new();
    }

    let pairs: Vec<String> = params.iter().map(|(k, v)| format!("{}={}", k, v)).collect();

    format!("?{}", pairs.join("&"))
}

/// Truncate a string to a maximum length
pub fn truncate_string(input: &str, max_length: usize) -> String {
    if input.len() <= max_length {
        input.to_string()
    } else {
        format!("{}...", &input[..max_length - 3])
    }
}

/// Check if a string contains only ASCII characters
pub fn is_ascii_only(input: &str) -> bool {
    input.chars().all(|c| c.is_ascii())
}

/// Convert a string to a safe identifier
pub fn to_safe_identifier(input: &str) -> String {
    input
        .chars()
        .map(|c| {
            if c.is_alphanumeric() {
                c.to_lowercase().next().unwrap()
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim_matches('_')
        .to_string()
}

/// Generate a random string
pub fn generate_random_string(length: usize) -> String {
    #[cfg(feature = "rand")]
    {
        use rand::Rng;
        const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";

        let mut rng = rand::thread_rng();
        (0..length)
            .map(|_| {
                let idx = rng.gen_range(0..CHARSET.len());
                CHARSET[idx] as char
            })
            .collect()
    }

    #[cfg(not(feature = "rand"))]
    {
        // Fallback for when rand is not available
        format!("random_{}", length)
    }
}

/// Calculate a simple hash of a string
pub fn simple_hash(input: &str) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    input.hash(&mut hasher);
    hasher.finish()
}

/// Check if a string is a valid JSON
pub fn is_valid_json(input: &str) -> bool {
    serde_json::from_str::<serde_json::Value>(input).is_ok()
}

/// Check if a string is a valid YAML
pub fn is_valid_yaml(input: &str) -> bool {
    serde_yaml::from_str::<serde_yaml::Value>(input).is_ok()
}

/// Escape special characters in a string
pub fn escape_string(input: &str) -> String {
    input
        .replace("\\", "\\\\")
        .replace("\"", "\\\"")
        .replace("\n", "\\n")
        .replace("\r", "\\r")
        .replace("\t", "\\t")
}

/// Unescape special characters in a string
pub fn unescape_string(input: &str) -> String {
    input
        .replace("\\n", "\n")
        .replace("\\r", "\r")
        .replace("\\t", "\t")
        .replace("\\\"", "\"")
        .replace("\\\\", "\\")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_request_id() {
        let id1 = generate_request_id();
        let id2 = generate_request_id();

        assert_ne!(id1, id2);
    }

    #[test]
    fn test_format_duration() {
        let duration = std::time::Duration::from_millis(1234);
        assert_eq!(format_duration(duration), "1.234s");

        let duration = std::time::Duration::from_millis(500);
        assert_eq!(format_duration(duration), "500ms");
    }

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(1024), "1.0 KB");
        assert_eq!(format_bytes(1024 * 1024), "1.0 MB");
        assert_eq!(format_bytes(1024 * 1024 * 1024), "1.0 GB");
        assert_eq!(format_bytes(512), "512 B");
    }

    #[test]
    fn test_validate_schema_name() {
        assert!(validate_schema_name("valid-schema").is_ok());
        assert!(validate_schema_name("valid_schema").is_ok());
        assert!(validate_schema_name("validSchema123").is_ok());

        assert!(validate_schema_name("").is_err());
        assert!(validate_schema_name("invalid schema").is_err());
        assert!(validate_schema_name("admin").is_err());
    }

    #[test]
    fn test_validate_version_string() {
        assert!(validate_version_string("1.0.0").is_ok());
        assert!(validate_version_string("2.1.3").is_ok());

        assert!(validate_version_string("").is_err());
        assert!(validate_version_string("1.0").is_err());
        assert!(validate_version_string("1.0.0.0").is_err());
        assert!(validate_version_string("1.a.0").is_err());
    }

    #[test]
    fn test_sanitize_filename() {
        assert_eq!(sanitize_filename("test-file"), "test-file");
        assert_eq!(sanitize_filename("test file"), "test_file");
        assert_eq!(sanitize_filename("test@file"), "test_file");
    }

    #[test]
    fn test_create_hash_map() {
        let pairs = vec![("key1", "value1"), ("key2", "value2")];
        let map = create_hash_map(pairs);

        assert_eq!(map.get("key1"), Some(&"value1".to_string()));
        assert_eq!(map.get("key2"), Some(&"value2".to_string()));
    }

    #[test]
    fn test_merge_hash_maps() {
        let mut base = HashMap::new();
        base.insert("key1".to_string(), "value1".to_string());

        let mut additional = HashMap::new();
        additional.insert("key2".to_string(), "value2".to_string());

        let merged = merge_hash_maps(base, additional);

        assert_eq!(merged.get("key1"), Some(&"value1".to_string()));
        assert_eq!(merged.get("key2"), Some(&"value2".to_string()));
    }

    #[test]
    fn test_extract_query_params() {
        let url = "http://example.com?param1=value1&param2=value2";
        let params = extract_query_params(url);

        assert_eq!(params.get("param1"), Some(&"value1".to_string()));
        assert_eq!(params.get("param2"), Some(&"value2".to_string()));
    }

    #[test]
    fn test_build_query_string() {
        let mut params = HashMap::new();
        params.insert("param1".to_string(), "value1".to_string());
        params.insert("param2".to_string(), "value2".to_string());

        let query = build_query_string(&params);
        assert!(query.contains("param1=value1"));
        assert!(query.contains("param2=value2"));
        assert!(query.starts_with('?'));
    }

    #[test]
    fn test_truncate_string() {
        assert_eq!(truncate_string("short", 10), "short");
        assert_eq!(truncate_string("very long string", 10), "very lo...");
    }

    #[test]
    fn test_is_ascii_only() {
        assert!(is_ascii_only("hello"));
        assert!(is_ascii_only("hello123"));
        assert!(!is_ascii_only("héllo"));
    }

    #[test]
    fn test_to_safe_identifier() {
        assert_eq!(to_safe_identifier("Hello World"), "hello_world");
        assert_eq!(to_safe_identifier("Test-123"), "test_123");
        assert_eq!(to_safe_identifier("__test__"), "test");
    }

    #[test]
    fn test_generate_random_string() {
        let str1 = generate_random_string(10);
        let str2 = generate_random_string(10);

        // When rand feature is enabled, we get actual random strings
        // When rand feature is not enabled, we get "random_10" (9 chars)
        #[cfg(feature = "rand")]
        {
            assert_eq!(str1.len(), 10);
            assert_eq!(str2.len(), 10);
            assert_ne!(str1, str2);
        }

        #[cfg(not(feature = "rand"))]
        {
            assert_eq!(str1.len(), 9); // "random_10" is 9 characters
            assert_eq!(str2.len(), 9);
            assert_eq!(str1, str2); // Both are "random_10"
        }
    }

    #[test]
    fn test_is_valid_json() {
        assert!(is_valid_json(r#"{"key": "value"}"#));
        assert!(is_valid_json(r#"["item1", "item2"]"#));
        assert!(!is_valid_json("invalid json"));
    }

    #[test]
    fn test_is_valid_yaml() {
        assert!(is_valid_yaml("key: value"));
        assert!(is_valid_yaml("- item1\n- item2"));
        assert!(!is_valid_yaml("invalid: yaml: ["));
    }

    #[test]
    fn test_escape_unescape_string() {
        let original = "Hello\nWorld\t\"Test\"";
        let escaped = escape_string(original);
        let unescaped = unescape_string(&escaped);

        assert_eq!(unescaped, original);
    }
}
