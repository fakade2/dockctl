use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Top-level config
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct DockctlConfig {
    pub general: GeneralConfig,
    pub keybindings: KeybindingsConfig,
    pub layout: LayoutConfig,
}

impl Default for DockctlConfig {
    fn default() -> Self {
        Self {
            general: GeneralConfig::default(),
            keybindings: KeybindingsConfig::default(),
            layout: LayoutConfig::default(),
        }
    }
}

// ---------------------------------------------------------------------------
// GeneralConfig
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct GeneralConfig {
    /// `"auto"` | `"docker"` | `"podman"`
    pub default_runtime: String,
    /// How often the TUI refreshes data from the container engine (ms).
    pub refresh_interval_ms: u64,
    /// Name of the colour theme.
    pub theme: String,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            default_runtime: "auto".to_string(),
            refresh_interval_ms: 1000,
            theme: "default".to_string(),
        }
    }
}

// ---------------------------------------------------------------------------
// KeybindingsConfig
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct KeybindingsConfig {
    pub stop: String,
    pub restart: String,
    pub remove: String,
    pub logs: String,
    pub exec: String,
    pub search: String,
}

impl Default for KeybindingsConfig {
    fn default() -> Self {
        Self {
            stop: "s".to_string(),
            restart: "r".to_string(),
            remove: "x".to_string(),
            logs: "l".to_string(),
            exec: "e".to_string(),
            search: "/".to_string(),
        }
    }
}

// ---------------------------------------------------------------------------
// LayoutConfig
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct LayoutConfig {
    /// Position of sidebar: "left" or "right"
    pub sidebar_position: String,
    /// Width of sidebar in columns
    pub sidebar_width: u16,
    /// Position of detail panel: "right" or "bottom"
    pub detail_position: String,
    /// Position of logs panel: "bottom" or "right"
    pub logs_position: String,
    /// Percentage of main area used by list vs detail (when side-by-side)
    pub main_split_percent: u16,
    /// Percentage of vertical space for main vs bottom panels
    pub vertical_split_percent: u16,
}

impl Default for LayoutConfig {
    fn default() -> Self {
        Self {
            sidebar_position: "left".to_string(),
            sidebar_width: 28,
            detail_position: "right".to_string(),
            logs_position: "bottom".to_string(),
            main_split_percent: 55,
            vertical_split_percent: 60,
        }
    }
}

// ---------------------------------------------------------------------------
// Loading
// ---------------------------------------------------------------------------

/// Return the canonical path for the config file:
/// `~/.config/dockctl/config.toml`
fn config_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("dockctl").join("config.toml"))
}

/// Load configuration from `~/.config/dockctl/config.toml`.
///
/// If the file does not exist the built-in defaults are returned.
/// If the file exists but is malformed, an error is returned so the user
/// gets immediate feedback.
pub fn load() -> Result<DockctlConfig> {
    let path = match config_path() {
        Some(p) => p,
        None => return Ok(DockctlConfig::default()),
    };

    if !path.exists() {
        return Ok(DockctlConfig::default());
    }

    let contents = std::fs::read_to_string(&path)
        .with_context(|| format!("failed to read config file at {}", path.display()))?;

    let config: DockctlConfig = toml::from_str(&contents)
        .with_context(|| format!("failed to parse config file at {}", path.display()))?;

    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_sensible() {
        let cfg = DockctlConfig::default();
        assert_eq!(cfg.general.default_runtime, "auto");
        assert_eq!(cfg.general.refresh_interval_ms, 1000);
        assert_eq!(cfg.keybindings.stop, "s");
        assert_eq!(cfg.keybindings.search, "/");
        assert_eq!(cfg.layout.sidebar_width, 28);
        assert_eq!(cfg.layout.sidebar_position, "left");
        assert_eq!(cfg.layout.main_split_percent, 55);
    }

    #[test]
    fn partial_toml_fills_defaults() {
        let toml_str = r#"
[general]
theme = "dark"
"#;
        let cfg: DockctlConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(cfg.general.theme, "dark");
        assert_eq!(cfg.general.default_runtime, "auto");
        assert_eq!(cfg.general.refresh_interval_ms, 1000);
        assert_eq!(cfg.keybindings.restart, "r");
        assert_eq!(cfg.layout.sidebar_width, 28);
    }

    #[test]
    fn layout_config_from_toml() {
        let toml_str = r#"
[layout]
sidebar_position = "right"
sidebar_width = 32
detail_position = "bottom"
main_split_percent = 70
vertical_split_percent = 50
"#;
        let cfg: DockctlConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(cfg.layout.sidebar_position, "right");
        assert_eq!(cfg.layout.sidebar_width, 32);
        assert_eq!(cfg.layout.detail_position, "bottom");
        assert_eq!(cfg.layout.main_split_percent, 70);
        assert_eq!(cfg.layout.vertical_split_percent, 50);
        // Other sections still default
        assert_eq!(cfg.general.refresh_interval_ms, 1000);
    }

    #[test]
    fn partial_layout_fills_defaults() {
        let toml_str = r#"
[layout]
sidebar_width = 40
"#;
        let cfg: DockctlConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(cfg.layout.sidebar_width, 40);
        assert_eq!(cfg.layout.sidebar_position, "left");
        assert_eq!(cfg.layout.detail_position, "right");
        assert_eq!(cfg.layout.main_split_percent, 55);
    }
}
