use anyhow::{Context, Result};
use std::path::PathBuf;
use tokio::process::Command;

/// Directory where dockctl persists tmux layout snapshots.
fn layouts_dir() -> Result<PathBuf> {
    let dir = dirs::config_dir()
        .context("could not determine config directory")?
        .join("dockctl")
        .join("layouts");
    Ok(dir)
}

/// Ensure the layouts directory exists.
async fn ensure_layouts_dir() -> Result<PathBuf> {
    let dir = layouts_dir()?;
    tokio::fs::create_dir_all(&dir)
        .await
        .with_context(|| format!("failed to create layouts dir at {}", dir.display()))?;
    Ok(dir)
}

/// Save the current tmux window layout under the given `name`.
///
/// The layout string is captured via `tmux display-message -p '#{window_layout}'`
/// and written to `~/.config/dockctl/layouts/<name>.layout`.
pub async fn save_layout(name: &str) -> Result<()> {
    let dir = ensure_layouts_dir().await?;

    let output = Command::new("tmux")
        .args(["display-message", "-p", "#{window_layout}"])
        .output()
        .await
        .context("failed to capture current tmux layout")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("tmux display-message failed: {}", stderr.trim());
    }

    let layout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let file = dir.join(format!("{}.layout", name));
    tokio::fs::write(&file, &layout)
        .await
        .with_context(|| format!("failed to write layout file {}", file.display()))?;

    Ok(())
}

/// Restore a previously saved layout by `name`.
///
/// Reads the layout string from disk and applies it via
/// `tmux select-layout <layout>`.
pub async fn restore_layout(name: &str) -> Result<()> {
    let dir = layouts_dir()?;
    let file = dir.join(format!("{}.layout", name));

    let layout = tokio::fs::read_to_string(&file)
        .await
        .with_context(|| format!("layout '{}' not found at {}", name, file.display()))?;

    let layout = layout.trim();

    let output = Command::new("tmux")
        .args(["select-layout", layout])
        .output()
        .await
        .context("failed to apply tmux layout")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("tmux select-layout failed: {}", stderr.trim());
    }

    Ok(())
}

/// List the names of all saved layouts (without the `.layout` extension).
pub async fn list_saved_layouts() -> Result<Vec<String>> {
    let dir = layouts_dir()?;

    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut names = Vec::new();
    let mut entries = tokio::fs::read_dir(&dir)
        .await
        .with_context(|| format!("failed to read layouts dir {}", dir.display()))?;

    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("layout")
            && let Some(stem) = path.file_stem().and_then(|s| s.to_str())
        {
            names.push(stem.to_string());
        }
    }

    names.sort();
    Ok(names)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layouts_dir_is_under_config() {
        let dir = layouts_dir().unwrap();
        assert!(dir.ends_with("dockctl/layouts"));
    }
}
