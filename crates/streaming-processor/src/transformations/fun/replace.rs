//! Transformation function implementations

use async_trait::async_trait;
use bridge_core::BridgeResult;

use super::trait_def::TransformationFunction;

/// Replace transformation function
pub struct ReplaceTransformation {
  from: String,
  to: String,
}

impl ReplaceTransformation {
  /// Create new replace transformation
  pub fn new(from: String, to: String) -> Self {
      Self { from, to }
  }
}

#[async_trait]
impl TransformationFunction for ReplaceTransformation {
  fn name(&self) -> &str {
      "replace"
  }

  fn function_type(&self) -> &str {
      "string"
  }

  async fn apply(&self, value: String) -> BridgeResult<String> {
      Ok(value.replace(&self.from, &self.to))
  }

  fn description(&self) -> &str {
      "Replace substring in string"
  }
}
