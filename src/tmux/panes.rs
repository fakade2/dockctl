use anyhow::{Context, Result};
use tokio::process::Command;

/// Create a horizontal (bottom) split pane running `command` and return the
/// new pane's ID (e.g. `%42`).
pub async fn create_horizontal_split(command: &str) -> Result<String> {
    let output = Command::new("tmux")
        .args(["split-window", "-v", "-P", "-F", "#{pane_id}", command])
        .output()
        .await
        .context("failed to run tmux split-window (horizontal)")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("tmux split-window failed: {}", stderr.trim());
    }

    let pane_id = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(pane_id)
}

/// Create a vertical (right-side) split pane running `command` and return the
/// new pane's ID.
pub async fn create_vertical_split(command: &str) -> Result<String> {
    let output = Command::new("tmux")
        .args(["split-window", "-h", "-P", "-F", "#{pane_id}", command])
        .output()
        .await
        .context("failed to run tmux split-window (vertical)")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("tmux split-window failed: {}", stderr.trim());
    }

    let pane_id = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(pane_id)
}

/// Close (kill) the pane identified by `pane_id`.
pub async fn close_pane(pane_id: &str) -> Result<()> {
    let output = Command::new("tmux")
        .args(["kill-pane", "-t", pane_id])
        .output()
        .await
        .context("failed to run tmux kill-pane")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("tmux kill-pane failed: {}", stderr.trim());
    }

    Ok(())
}
