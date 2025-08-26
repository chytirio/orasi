//! Transformation function implementations

use async_trait::async_trait;
use bridge_core::BridgeResult;

use super::trait_def::TransformationFunction;

/// Substring transformation function
pub struct SubstringTransformation {
  start: usize,
  end: Option<usize>,
}

impl SubstringTransformation {
  /// Create new substring transformation
  pub fn new(start: usize, end: Option<usize>) -> Self {
      Self { start, end }
  }
}

#[async_trait]
impl TransformationFunction for SubstringTransformation {
  fn name(&self) -> &str {
      "substring"
  }

  fn function_type(&self) -> &str {
      "string"
  }

  async fn apply(&self, value: String) -> BridgeResult<String> {
      if self.start >= value.len() {
          return Ok(String::new());
      }

      let end = self.end.unwrap_or(value.len()).min(value.len());
      let start = self.start.min(end);

      Ok(value[start..end].to_string())
  }

  fn description(&self) -> &str {
      "Extract substring from string"
  }
}
