//! Schema version management
//!
//! This module contains the SchemaVersion struct and version-related functionality.

use serde::{Deserialize, Serialize};

/// Schema version
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct SchemaVersion {
    /// Major version
    pub major: u32,

    /// Minor version
    pub minor: u32,

    /// Patch version
    pub patch: u32,

    /// Pre-release identifier
    pub pre_release: Option<String>,

    /// Build metadata
    pub build: Option<String>,
}

impl SchemaVersion {
    /// Create a new schema version
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self {
            major,
            minor,
            patch,
            pre_release: None,
            build: None,
        }
    }

    /// Create a version with pre-release
    pub fn with_pre_release(major: u32, minor: u32, patch: u32, pre_release: String) -> Self {
        Self {
            major,
            minor,
            patch,
            pre_release: Some(pre_release),
            build: None,
        }
    }

    /// Create a version with build metadata
    pub fn with_build(major: u32, minor: u32, patch: u32, build: String) -> Self {
        Self {
            major,
            minor,
            patch,
            pre_release: None,
            build: Some(build),
        }
    }

    /// Check if this version is compatible with another version
    pub fn is_compatible_with(&self, other: &SchemaVersion) -> bool {
        // Major version must match for compatibility
        self.major == other.major
    }

    /// Check if this version is newer than another version
    pub fn is_newer_than(&self, other: &SchemaVersion) -> bool {
        self > other
    }

    /// Check if this version is older than another version
    pub fn is_older_than(&self, other: &SchemaVersion) -> bool {
        self < other
    }
}

impl std::fmt::Display for SchemaVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)?;

        if let Some(pre_release) = &self.pre_release {
            write!(f, "-{}", pre_release)?;
        }

        if let Some(build) = &self.build {
            write!(f, "+{}", build)?;
        }

        Ok(())
    }
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

impl std::str::FromStr for SchemaVersion {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Validate the version string first
        validate_version_string(s)?;
        
        // Simple version parsing - can be enhanced
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() != 3 {
            return Err("Version must be in format X.Y.Z".to_string());
        }

        let major = parts[0]
            .parse::<u32>()
            .map_err(|_| "Invalid major version".to_string())?;
        let minor = parts[1]
            .parse::<u32>()
            .map_err(|_| "Invalid minor version".to_string())?;
        let patch = parts[2]
            .parse::<u32>()
            .map_err(|_| "Invalid patch version".to_string())?;

        Ok(SchemaVersion::new(major, minor, patch))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_version_parsing() {
        let version: SchemaVersion = "1.2.3".parse().unwrap();
        assert_eq!(version.major, 1);
        assert_eq!(version.minor, 2);
        assert_eq!(version.patch, 3);
        assert_eq!(version.to_string(), "1.2.3");
    }

    #[test]
    fn test_schema_version_compatibility() {
        let v1 = SchemaVersion::new(1, 0, 0);
        let v2 = SchemaVersion::new(1, 1, 0);
        let v3 = SchemaVersion::new(2, 0, 0);

        assert!(v1.is_compatible_with(&v2));
        assert!(v2.is_compatible_with(&v1));
        assert!(!v1.is_compatible_with(&v3));
        assert!(!v3.is_compatible_with(&v1));
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
}
