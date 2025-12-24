//! # Application Settings Module
//!
//! This module manages the persistent configuration for Heimdall-rs.
//! It defines the `AppSettings` struct which holds user preferences such as:
//! - Visual Theme (Dark Mode)
//! - CPU Color Mode (Uniform vs Per-Core)
//! - Custom Chart Colors (CPU, RAM, GPU, Network)
//!
//! It handles serialization and deserialization (via `serde`) to a JSON file stored in the
//! standard system configuration directory using the `directories` crate.

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Persistent application settings.
/// Stores user preferences such as theme (dark mode), chart colors, and per-core CPU colors.
/// Serialized to `config.json` in the system's standard configuration directory.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AppSettings {
    pub dark_mode: bool,
    pub use_uniform_cpu: bool,
    pub cpu_color: String,
    pub ram_color: String,
    pub gpu_color: String,
    pub net_color: String,
    pub cpu_core_colors: Vec<String>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            dark_mode: false,
            use_uniform_cpu: false,
            cpu_color: "#3498db".to_string(), // Blue
            ram_color: "#2ecc71".to_string(), // Green
            gpu_color: "#9b59b6".to_string(), // Purple
            net_color: "#e67e22".to_string(), // Orange
            cpu_core_colors: Vec::new(),
        }
    }
}

impl AppSettings {
    fn get_path() -> PathBuf {
        if let Some(proj_dirs) = ProjectDirs::from("com", "heimdall", "heimdall-rs") {
            let config_dir = proj_dirs.config_dir();
            if !config_dir.exists() {
                let _ = fs::create_dir_all(config_dir);
            }
            config_dir.join("config.json")
        } else {
            PathBuf::from("config.json")
        }
    }

    pub fn load() -> Self {
        let path = Self::get_path();
        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(settings) = serde_json::from_str(&content) {
                return settings;
            }
        }
        Self::default()
    }

    pub fn save(&self) {
        let path = Self::get_path();
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = fs::write(path, json);
        }
    }
}
