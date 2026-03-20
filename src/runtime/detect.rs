//! Auto-detection of available container runtimes.
//!
//! On startup, dockctl probes for Docker and Podman sockets and CLI binaries
//! to decide which backend to use. The user can override with `--runtime`.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use tracing::{debug, info, warn};

use super::docker::DockerRuntime;
use super::podman::PodmanRuntime;
use super::{ContainerRuntime, RuntimeType};

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Auto-detect an available container runtime, or use the explicit override.
///
/// When `override_type` is `Some`, only that runtime is tried. Otherwise the
/// function probes for Docker first, then Podman.
///
/// Returns a boxed trait object ready for use.
pub async fn detect_runtime(
    override_type: Option<RuntimeType>,
) -> Result<Box<dyn ContainerRuntime>> {
    if let Some(rt) = override_type {
        return match rt {
            RuntimeType::Docker => {
                connect_docker().map(|r| -> Box<dyn ContainerRuntime> { Box::new(r) })
            }
            RuntimeType::Podman => {
                connect_podman().map(|r| -> Box<dyn ContainerRuntime> { Box::new(r) })
            }
        };
    }

    // Probe Docker first — it is the more common default.
    if let Some(docker) = try_docker() {
        info!("detected Docker runtime");
        return Ok(Box::new(docker));
    }

    // Then try Podman.
    if let Some(podman) = try_podman() {
        info!("detected Podman runtime");
        return Ok(Box::new(podman));
    }

    anyhow::bail!(
        "no container runtime detected. Ensure Docker or Podman is installed and their \
         socket is accessible, or specify --runtime explicitly."
    )
}

// ---------------------------------------------------------------------------
// Docker detection
// ---------------------------------------------------------------------------

/// Well-known Docker socket paths.
const DOCKER_SOCKET_PATHS: &[&str] = &[
    "/var/run/docker.sock",
    // Docker Desktop on macOS
    "/Users/Shared/docker.sock",
];

/// Try to connect to Docker via its socket. Returns `None` on failure.
fn try_docker() -> Option<DockerRuntime> {
    // Check well-known socket paths.
    for path in DOCKER_SOCKET_PATHS {
        if Path::new(path).exists() {
            debug!(path, "found Docker socket");
            match DockerRuntime::connect_with_socket(path) {
                Ok(rt) => return Some(rt),
                Err(e) => {
                    warn!(path, error = %e, "Docker socket exists but connection failed");
                }
            }
        }
    }

    // Also honour $DOCKER_HOST if set.
    if let Ok(host) = std::env::var("DOCKER_HOST")
        && host.starts_with("unix://")
    {
        let path = host.trim_start_matches("unix://");
        if Path::new(path).exists() {
            debug!(path, "found Docker socket via DOCKER_HOST");
            if let Ok(rt) = DockerRuntime::connect_with_socket(path) {
                return Some(rt);
            }
        }
    }

    // Fallback: check if `docker` binary is in PATH (the daemon may still be
    // reachable via the default socket even if we didn't find the socket file).
    if binary_in_path("docker") {
        debug!("docker binary found in PATH, trying default connection");
        if let Ok(rt) = DockerRuntime::connect_default() {
            return Some(rt);
        }
    }

    None
}

/// Force-connect to Docker; return an error if it fails.
fn connect_docker() -> Result<DockerRuntime> {
    // Try well-known sockets first.
    for path in DOCKER_SOCKET_PATHS {
        if Path::new(path).exists() {
            return DockerRuntime::connect_with_socket(path);
        }
    }
    // Fall back to bollard defaults.
    DockerRuntime::connect_default()
        .context("failed to connect to Docker — is the Docker daemon running?")
}

// ---------------------------------------------------------------------------
// Podman detection
// ---------------------------------------------------------------------------

/// Try to connect to Podman. Returns `None` on failure.
fn try_podman() -> Option<PodmanRuntime> {
    // Try the default socket path helper.
    match PodmanRuntime::connect_default() {
        Ok(rt) => {
            debug!("connected to Podman via default socket");
            return Some(rt);
        }
        Err(e) => {
            debug!(error = %e, "Podman default socket not available");
        }
    }

    // macOS: check several common Podman machine socket locations.
    if let Some(home) = dirs::home_dir() {
        let candidates = podman_socket_candidates(&home);
        for path in &candidates {
            if path.exists() {
                let path_str = path.to_string_lossy();
                debug!(%path_str, "found Podman socket candidate");
                match PodmanRuntime::connect_with_socket(&path_str) {
                    Ok(rt) => return Some(rt),
                    Err(e) => {
                        warn!(%path_str, error = %e, "Podman socket exists but connection failed");
                    }
                }
            }
        }
    }

    // Fallback: check if `podman` binary is in PATH.
    if binary_in_path("podman") {
        debug!("podman binary found in PATH");
        // The binary is present but the socket may not be activated yet.
        // We cannot use the compat API without a socket, so log a warning.
        warn!(
            "podman CLI is available but no socket was found. \
             Run `podman system service --time=0` or start a Podman machine."
        );
    }

    None
}

/// Force-connect to Podman; return an error if it fails.
fn connect_podman() -> Result<PodmanRuntime> {
    PodmanRuntime::connect_default().context(
        "failed to connect to Podman — is the Podman socket activated? \
         Try `podman system service --time=0` or start a Podman machine.",
    )
}

// ---------------------------------------------------------------------------
// Utility helpers
// ---------------------------------------------------------------------------

/// Return extra Podman socket candidates based on the user's home directory.
fn podman_socket_candidates(home: &Path) -> Vec<PathBuf> {
    vec![
        home.join(".local/share/containers/podman/machine/podman.sock"),
        home.join(".local/share/containers/podman/machine/podman-machine-default/podman.sock"),
        home.join(".local/share/containers/podman/machine/qemu/podman.sock"),
    ]
}

/// Check whether a binary with the given name exists somewhere on `$PATH`.
fn binary_in_path(name: &str) -> bool {
    std::env::var_os("PATH")
        .map(|paths| {
            std::env::split_paths(&paths).any(|dir| {
                let candidate = dir.join(name);
                candidate.is_file()
            })
        })
        .unwrap_or(false)
}
