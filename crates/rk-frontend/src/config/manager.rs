//! Configuration manager for loading, saving, and managing app configuration

use std::path::PathBuf;
use std::sync::Arc;

use parking_lot::RwLock;

use super::AppConfig;

/// Shared configuration manager type
pub type SharedConfig = Arc<RwLock<ConfigManager>>;

/// Configuration error types
#[derive(Debug, Clone)]
pub enum ConfigError {
    /// IO error during file operations
    Io(String),
    /// Error during serialization
    Serialize(String),
    /// Error during deserialization
    Deserialize(String),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::Io(msg) => write!(f, "IO error: {}", msg),
            ConfigError::Serialize(msg) => write!(f, "Serialization error: {}", msg),
            ConfigError::Deserialize(msg) => write!(f, "Deserialization error: {}", msg),
        }
    }
}

impl std::error::Error for ConfigError {}

/// Configuration manager handles loading, saving, and accessing app configuration
pub struct ConfigManager {
    config: AppConfig,
    config_path: PathBuf,
    dirty: bool,
}

impl ConfigManager {
    /// Create a new configuration manager, loading from disk if available
    pub fn new() -> Self {
        let config_path = Self::config_path();
        let config = Self::load_from_path(&config_path).unwrap_or_else(|| {
            tracing::info!("No config file found, using defaults");
            AppConfig::new()
        });

        Self {
            config,
            config_path,
            dirty: false,
        }
    }

    /// Get the OS-standard configuration directory
    #[cfg(not(target_arch = "wasm32"))]
    fn config_dir() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("rk-editor")
    }

    #[cfg(target_arch = "wasm32")]
    fn config_dir() -> PathBuf {
        PathBuf::from(".")
    }

    /// Get the configuration file path
    fn config_path() -> PathBuf {
        Self::config_dir().join("config.rk")
    }

    /// Load configuration from a file path
    #[cfg(not(target_arch = "wasm32"))]
    fn load_from_path(path: &PathBuf) -> Option<AppConfig> {
        let content = std::fs::read_to_string(path).ok()?;
        match ron::from_str(&content) {
            Ok(config) => {
                tracing::info!("Loaded config from {:?}", path);
                Some(config)
            }
            Err(e) => {
                tracing::warn!("Failed to parse config file: {}", e);
                None
            }
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn load_from_path(_path: &PathBuf) -> Option<AppConfig> {
        // WASM uses localStorage via eframe, not filesystem
        None
    }

    /// Get a reference to the current configuration
    pub fn config(&self) -> &AppConfig {
        &self.config
    }

    /// Get a mutable reference to the configuration (marks as dirty)
    pub fn config_mut(&mut self) -> &mut AppConfig {
        self.dirty = true;
        &mut self.config
    }

    /// Check if the configuration has unsaved changes
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Save the configuration to disk
    #[cfg(not(target_arch = "wasm32"))]
    pub fn save(&mut self) -> Result<(), ConfigError> {
        if !self.dirty {
            return Ok(());
        }

        // Ensure config directory exists
        if let Some(parent) = self.config_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| ConfigError::Io(e.to_string()))?;
        }

        let content = ron::ser::to_string_pretty(&self.config, ron::ser::PrettyConfig::default())
            .map_err(|e| ConfigError::Serialize(e.to_string()))?;

        std::fs::write(&self.config_path, &content).map_err(|e| ConfigError::Io(e.to_string()))?;

        tracing::info!("Saved config to {:?}", self.config_path);
        self.dirty = false;
        Ok(())
    }

    #[cfg(target_arch = "wasm32")]
    pub fn save(&mut self) -> Result<(), ConfigError> {
        // WASM uses localStorage via eframe
        self.dirty = false;
        Ok(())
    }

    /// Reset configuration to defaults
    pub fn reset_to_defaults(&mut self) {
        self.config = AppConfig::new();
        self.dirty = true;
    }

    /// Get the config file path (for display purposes)
    pub fn config_file_path(&self) -> &PathBuf {
        &self.config_path
    }
}

impl Default for ConfigManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Create a new shared configuration manager
pub fn create_shared_config() -> SharedConfig {
    Arc::new(RwLock::new(ConfigManager::new()))
}
