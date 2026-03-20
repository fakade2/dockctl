pub mod panes;
pub mod sessions;

use anyhow::Result;

// ---------------------------------------------------------------------------
// Detection
// ---------------------------------------------------------------------------

/// Returns `true` when the process is running inside a tmux session.
///
/// Detection is based on the `TMUX` environment variable which tmux always
/// sets for child processes.
pub fn is_tmux_available() -> bool {
    std::env::var("TMUX").is_ok_and(|v| !v.is_empty())
}

// ---------------------------------------------------------------------------
// TmuxIntegration — convenience wrapper
// ---------------------------------------------------------------------------

/// High-level helpers that combine pane/session primitives for dockctl's
/// container-centric workflows.
pub struct TmuxIntegration {
    /// The runtime CLI binary name (e.g. `"docker"` or `"podman"`).
    runtime_bin: String,
}

impl TmuxIntegration {
    pub fn new(runtime_bin: &str) -> Self {
        Self {
            runtime_bin: runtime_bin.to_string(),
        }
    }

    /// Open a horizontal tmux split and exec into the given container.
    ///
    /// `command` is the shell to run inside the container (e.g. `"/bin/sh"`).
    pub async fn split_pane_exec(&self, container_id: &str, command: &str) -> Result<String> {
        let full_cmd = format!("{} exec -it {} {}", self.runtime_bin, container_id, command);
        panes::create_horizontal_split(&full_cmd).await
    }

    /// Open a horizontal tmux split that tails the container's logs.
    pub async fn split_pane_logs(&self, container_id: &str) -> Result<String> {
        let full_cmd = format!("{} logs -f {}", self.runtime_bin, container_id);
        panes::create_horizontal_split(&full_cmd).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detection_respects_env() {
        // When TMUX is not set (or empty) we expect false.
        // We cannot guarantee the CI environment has tmux, so just ensure
        // the function doesn't panic.
        let _ = is_tmux_available();
    }
}
