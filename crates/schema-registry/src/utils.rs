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
    fn test_escape_unescape_string() {
        let original = "Hello\nWorld\t\"Test\"";
        let escaped = escape_string(original);
        let unescaped = unescape_string(&escaped);

        assert_eq!(unescaped, original);
    }
}
