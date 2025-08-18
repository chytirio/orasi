//! Validation rules types

/// Validation rule
#[derive(Debug, Clone)]
pub struct ValidationRule {
    /// Rule name
    pub name: String,

    /// Rule description
    pub description: String,

    /// Rule type
    pub rule_type: ValidationRuleType,

    /// Whether the rule is enabled
    pub enabled: bool,
}

/// Validation rule type
#[derive(Debug, Clone)]
pub enum ValidationRuleType {
    /// Schema format validation
    SchemaFormat,

    /// Schema content validation
    SchemaContent,

    /// Schema metadata validation
    SchemaMetadata,

    /// Custom validation rule
    Custom,
}
