use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Top-level config
// ---------------------------------------------------------------------------

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct DockctlConfig {
    pub general: GeneralConfig,
    pub keybindings: KeybindingsConfig,
    pub layout: LayoutConfig,
    pub columns: ColumnsConfig,
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
            // Valid values: "dashboard" (lazygit-style), "operator" (k9s-style)
            theme: "dashboard".to_string(),
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
    /// Percentage of horizontal space used by the left navigation panel.
    pub left_panel_percent: u16,
    /// Percentage of the main area used by the preview split.
    pub preview_split_percent: u16,
    /// Position of the logs panel: "bottom" or "right".
    pub logs_position: String,
}

impl Default for LayoutConfig {
    fn default() -> Self {
        Self {
            left_panel_percent: 22,
            preview_split_percent: 60,
            logs_position: "bottom".to_string(),
        }
    }
}

// ---------------------------------------------------------------------------
// ColumnsConfig — configurable columns per panel
// ---------------------------------------------------------------------------

/// Defines which columns are shown in each right-panel table.
/// Users can reorder or remove columns via config. Left/Right arrows
/// scroll through columns when the table is wider than the terminal.
#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct ColumnsConfig {
    pub containers: Vec<String>,
    pub images: Vec<String>,
    pub volumes: Vec<String>,
    pub networks: Vec<String>,
}

impl Default for ColumnsConfig {
    fn default() -> Self {
        Self {
            containers: vec![
                "Name".into(),
                // "Image".into(),
                "State".into(),
                "Ports".into(),
                "CPU".into(),
                "Memory".into(),
                "ID".into(),
                "Status".into(),
            ],
            images: vec![
                "Repository".into(),
                "Tag".into(),
                "ID".into(),
                "Size".into(),
                "Created".into(),
            ],
            volumes: vec![
                "Name".into(),
                "Driver".into(),
                "Mountpoint".into(),
                "Created".into(),
            ],
            networks: vec![
                "Name".into(),
                "ID".into(),
                "Driver".into(),
                "Scope".into(),
                "Containers".into(),
            ],
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
        assert_eq!(cfg.layout.left_panel_percent, 22);
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
        assert_eq!(cfg.layout.left_panel_percent, 22);
    }

    #[test]
    fn layout_config_from_toml() {
        let toml_str = r#"
[layout]
left_panel_percent = 25
preview_split_percent = 70
logs_position = "right"
"#;
        let cfg: DockctlConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(cfg.layout.left_panel_percent, 25);
        assert_eq!(cfg.layout.preview_split_percent, 70);
        assert_eq!(cfg.layout.logs_position, "right");
        // Other sections still default
        assert_eq!(cfg.general.refresh_interval_ms, 1000);
    }

    #[test]
    fn partial_layout_fills_defaults() {
        let toml_str = r#"
[layout]
left_panel_percent = 40
"#;
        let cfg: DockctlConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(cfg.layout.left_panel_percent, 40);
        assert_eq!(cfg.layout.preview_split_percent, 60);
        assert_eq!(cfg.layout.logs_position, "bottom");
    }
}
