//! Window manager for managing multiple windows

use bridge_core::BridgeResult;
use std::collections::HashMap;
use tracing::info;

use super::types::Window;

/// Window manager for managing multiple windows
pub struct WindowManager {
    windows: HashMap<String, Box<dyn Window>>,
}

impl WindowManager {
    /// Create new window manager
    pub fn new() -> Self {
        Self {
            windows: HashMap::new(),
        }
    }

    /// Add window
    pub fn add_window(&mut self, window: Box<dyn Window>) {
        self.windows.insert(window.id().to_string(), window);
    }

    /// Remove window
    pub fn remove_window(&mut self, window_id: &str) -> Option<Box<dyn Window>> {
        self.windows.remove(window_id)
    }

    /// Get window
    pub fn get_window(&self, window_id: &str) -> Option<&dyn Window> {
        self.windows.get(window_id).map(|w| w.as_ref())
    }

    /// Get all window IDs
    pub fn get_window_ids(&self) -> Vec<String> {
        self.windows.keys().cloned().collect()
    }

    /// Get active windows
    pub fn get_active_windows(&self) -> Vec<&dyn Window> {
        self.windows
            .values()
            .filter(|w| w.is_active())
            .map(|w| w.as_ref())
            .collect()
    }

    /// Clean up expired windows
    pub async fn cleanup_expired_windows(&mut self) -> BridgeResult<()> {
        let expired_ids: Vec<String> = self
            .windows
            .iter()
            .filter(|(_, window)| !window.is_active())
            .map(|(id, _)| id.clone())
            .collect();

        for id in expired_ids {
            if let Some(mut window) = self.windows.remove(&id) {
                window.close().await?;
                info!("Closed expired window: {}", id);
            }
        }

        Ok(())
    }
}
