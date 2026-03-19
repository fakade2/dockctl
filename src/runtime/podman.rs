//! Podman backend — implements [`ContainerRuntime`] using `bollard` pointed at the
//! Podman-compatible Docker API socket, with pod operations delegated to the
//! `podman` CLI via `tokio::process::Command`.

use std::path::PathBuf;

use anyhow::{Context, Result};
use async_trait::async_trait;
use tokio::process::Command;

use super::docker::DockerRuntime;
use super::{
    ContainerDetail, ContainerRuntime, ContainerStats, ContainerSummary, ImageSummary,
    NetworkDetail, NetworkSummary, PodContainerRef, PodSummary, RuntimeType, SystemInfo,
    VolumeSummary,
};

// ---------------------------------------------------------------------------
// PodmanRuntime
// ---------------------------------------------------------------------------

/// A [`ContainerRuntime`] backed by Podman.
///
/// Podman exposes a Docker-compatible API, so most operations are handled by
/// the inner [`DockerRuntime`] connected to the Podman socket. Pod operations
/// — which have no Docker equivalent — shell out to the `podman` CLI.
pub struct PodmanRuntime {
    /// Delegate for Docker-compatible API calls.
    inner: DockerRuntime,
    /// Path to the `podman` binary used for pod operations.
    podman_bin: PathBuf,
}

impl PodmanRuntime {
    /// Connect to the default Podman socket for the current user.
    ///
    /// The socket is expected at `/run/user/{uid}/podman/podman.sock` (rootless)
    /// or `/run/podman/podman.sock` (rootful). On macOS the socket may be at
    /// `$HOME/.local/share/containers/podman/machine/podman.sock`.
    pub fn connect_default() -> Result<Self> {
        let socket_path = default_podman_socket_path()
            .context("unable to determine Podman socket path")?;

        Self::connect_with_socket(&socket_path)
    }

    /// Connect to a Podman-compatible socket at an explicit path.
    pub fn connect_with_socket(path: &str) -> Result<Self> {
        let inner = DockerRuntime::connect_with_socket(path)
            .with_context(|| format!("failed to connect to Podman socket at {path}"))?;

        let podman_bin = which_podman().unwrap_or_else(|| PathBuf::from("podman"));

        Ok(Self { inner, podman_bin })
    }

    // -- CLI helpers --------------------------------------------------------

    /// Run a `podman` subcommand and return its stdout as a trimmed String.
    async fn podman_cmd(&self, args: &[&str]) -> Result<String> {
        let output = Command::new(&self.podman_bin)
            .args(args)
            .output()
            .await
            .with_context(|| {
                format!(
                    "failed to execute: {} {}",
                    self.podman_bin.display(),
                    args.join(" ")
                )
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!(
                "podman {} failed (exit {}): {}",
                args.join(" "),
                output.status,
                stderr.trim()
            );
        }

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Resolve the default Podman socket path.
fn default_podman_socket_path() -> Option<String> {
    // macOS: Podman machine socket
    #[cfg(target_os = "macos")]
    {
        if let Some(home) = dirs::home_dir() {
            let machine_sock = home
                .join(".local/share/containers/podman/machine/podman.sock");
            if machine_sock.exists() {
                return Some(machine_sock.to_string_lossy().into_owned());
            }
            // Podman machine default (newer layout)
            let machine_default = home
                .join(".local/share/containers/podman/machine/podman-machine-default/podman.sock");
            if machine_default.exists() {
                return Some(machine_default.to_string_lossy().into_owned());
            }
        }
    }

    // Linux: rootless socket
    #[cfg(target_os = "linux")]
    {
        let uid = unsafe { libc::getuid() };
        let rootless = format!("/run/user/{uid}/podman/podman.sock");
        if std::path::Path::new(&rootless).exists() {
            return Some(rootless);
        }
        // Rootful fallback
        let rootful = "/run/podman/podman.sock";
        if std::path::Path::new(rootful).exists() {
            return Some(rootful.to_string());
        }
    }

    // Generic fallback: check XDG_RUNTIME_DIR
    if let Ok(xdg) = std::env::var("XDG_RUNTIME_DIR") {
        let sock = format!("{xdg}/podman/podman.sock");
        if std::path::Path::new(&sock).exists() {
            return Some(sock);
        }
    }

    None
}

/// Find the `podman` binary on `$PATH`.
fn which_podman() -> Option<PathBuf> {
    std::env::var_os("PATH").and_then(|paths| {
        std::env::split_paths(&paths).find_map(|dir| {
            let candidate = dir.join("podman");
            if candidate.is_file() {
                Some(candidate)
            } else {
                None
            }
        })
    })
}

// ---------------------------------------------------------------------------
// ContainerRuntime implementation
// ---------------------------------------------------------------------------

#[async_trait]
impl ContainerRuntime for PodmanRuntime {
    fn runtime_type(&self) -> RuntimeType {
        RuntimeType::Podman
    }

    fn supports_pods(&self) -> bool {
        true
    }

    // -- Delegated to inner DockerRuntime via Podman compat API --------------

    async fn list_containers(&self, all: bool) -> Result<Vec<ContainerSummary>> {
        self.inner.list_containers(all).await
    }

    async fn inspect_container(&self, id: &str) -> Result<ContainerDetail> {
        self.inner.inspect_container(id).await
    }

    async fn start_container(&self, id: &str) -> Result<()> {
        self.inner.start_container(id).await
    }

    async fn stop_container(&self, id: &str) -> Result<()> {
        self.inner.stop_container(id).await
    }

    async fn restart_container(&self, id: &str) -> Result<()> {
        self.inner.restart_container(id).await
    }

    async fn remove_container(&self, id: &str, force: bool) -> Result<()> {
        self.inner.remove_container(id, force).await
    }

    async fn container_logs(&self, id: &str, tail: Option<usize>) -> Result<String> {
        self.inner.container_logs(id, tail).await
    }

    async fn container_stats(&self, id: &str) -> Result<ContainerStats> {
        self.inner.container_stats(id).await
    }

    async fn list_images(&self) -> Result<Vec<ImageSummary>> {
        self.inner.list_images().await
    }

    async fn pull_image(&self, reference: &str) -> Result<()> {
        self.inner.pull_image(reference).await
    }

    async fn remove_image(&self, id: &str, force: bool) -> Result<()> {
        self.inner.remove_image(id, force).await
    }

    async fn inspect_image(&self, id: &str) -> Result<ImageSummary> {
        self.inner.inspect_image(id).await
    }

    async fn list_volumes(&self) -> Result<Vec<VolumeSummary>> {
        self.inner.list_volumes().await
    }

    async fn create_volume(&self, name: &str) -> Result<VolumeSummary> {
        self.inner.create_volume(name).await
    }

    async fn remove_volume(&self, name: &str, force: bool) -> Result<()> {
        self.inner.remove_volume(name, force).await
    }

    async fn inspect_volume(&self, name: &str) -> Result<VolumeSummary> {
        self.inner.inspect_volume(name).await
    }

    async fn list_networks(&self) -> Result<Vec<NetworkSummary>> {
        self.inner.list_networks().await
    }

    async fn create_network(&self, name: &str, driver: &str) -> Result<NetworkSummary> {
        self.inner.create_network(name, driver).await
    }

    async fn remove_network(&self, id: &str) -> Result<()> {
        self.inner.remove_network(id).await
    }

    async fn inspect_network(&self, id: &str) -> Result<NetworkDetail> {
        self.inner.inspect_network(id).await
    }

    async fn system_info(&self) -> Result<SystemInfo> {
        let mut info = self.inner.system_info().await?;
        // Override the runtime type — the inner reports Docker since it uses
        // the compat API.
        info.runtime_type = RuntimeType::Podman;
        Ok(info)
    }

    // -- Pod operations (Podman-only, via CLI) ------------------------------

    async fn list_pods(&self) -> Result<Vec<PodSummary>> {
        let json = self
            .podman_cmd(&[
                "pod", "ps", "--format", "json", "--no-trunc",
            ])
            .await
            .context("failed to list pods")?;

        if json.is_empty() {
            return Ok(Vec::new());
        }

        let raw: Vec<PodmanPodListEntry> = serde_json::from_str(&json)
            .context("failed to parse `podman pod ps` JSON output")?;

        Ok(raw.into_iter().map(PodmanPodListEntry::into_domain).collect())
    }

    async fn create_pod(&self, name: &str) -> Result<PodSummary> {
        let id = self
            .podman_cmd(&["pod", "create", "--name", name])
            .await
            .with_context(|| format!("failed to create pod {name}"))?;

        // Return a summary by inspecting the newly created pod.
        self.inspect_pod(&id).await
    }

    async fn remove_pod(&self, id: &str, force: bool) -> Result<()> {
        let mut args = vec!["pod", "rm"];
        if force {
            args.push("--force");
        }
        args.push(id);

        self.podman_cmd(&args)
            .await
            .with_context(|| format!("failed to remove pod {id}"))?;
        Ok(())
    }

    async fn start_pod(&self, id: &str) -> Result<()> {
        self.podman_cmd(&["pod", "start", id])
            .await
            .with_context(|| format!("failed to start pod {id}"))?;
        Ok(())
    }

    async fn stop_pod(&self, id: &str) -> Result<()> {
        self.podman_cmd(&["pod", "stop", id])
            .await
            .with_context(|| format!("failed to stop pod {id}"))?;
        Ok(())
    }

    async fn inspect_pod(&self, id: &str) -> Result<PodSummary> {
        let json = self
            .podman_cmd(&["pod", "inspect", id, "--format", "json"])
            .await
            .with_context(|| format!("failed to inspect pod {id}"))?;

        let raw: PodmanPodInspect = serde_json::from_str(&json)
            .with_context(|| format!("failed to parse `podman pod inspect {id}` JSON"))?;

        Ok(raw.into_domain())
    }
}

// ---------------------------------------------------------------------------
// Podman CLI JSON types (deserialization helpers)
// ---------------------------------------------------------------------------

/// Shape of a single entry in `podman pod ps --format json` output.
#[derive(serde::Deserialize)]
struct PodmanPodListEntry {
    #[serde(alias = "Id")]
    id: String,
    #[serde(alias = "Name")]
    name: String,
    #[serde(alias = "Status")]
    status: String,
    #[serde(alias = "Created", default)]
    created: String,
    #[serde(alias = "InfraId", default)]
    infra_id: String,
    #[serde(alias = "Containers", default)]
    containers: Option<Vec<PodmanPodContainerRef>>,
}

#[derive(serde::Deserialize)]
struct PodmanPodContainerRef {
    #[serde(alias = "Id")]
    id: String,
    #[serde(alias = "Name", default)]
    name: String,
    #[serde(alias = "Status", default)]
    status: String,
}

impl PodmanPodListEntry {
    fn into_domain(self) -> PodSummary {
        PodSummary {
            id: self.id,
            name: self.name,
            status: self.status,
            created: self.created,
            infra_id: self.infra_id,
            containers: self
                .containers
                .unwrap_or_default()
                .into_iter()
                .map(|c| PodContainerRef {
                    id: c.id,
                    name: c.name,
                    status: c.status,
                })
                .collect(),
        }
    }
}

/// Shape of `podman pod inspect --format json` output.
#[derive(serde::Deserialize)]
struct PodmanPodInspect {
    #[serde(alias = "Id")]
    id: String,
    #[serde(alias = "Name")]
    name: String,
    #[serde(alias = "State")]
    state: String,
    #[serde(alias = "Created", default)]
    created: String,
    #[serde(alias = "InfraContainerId", default)]
    infra_container_id: String,
    #[serde(alias = "Containers", default)]
    containers: Option<Vec<PodmanPodContainerRef>>,
}

impl PodmanPodInspect {
    fn into_domain(self) -> PodSummary {
        PodSummary {
            id: self.id,
            name: self.name,
            status: self.state,
            created: self.created,
            infra_id: self.infra_container_id,
            containers: self
                .containers
                .unwrap_or_default()
                .into_iter()
                .map(|c| PodContainerRef {
                    id: c.id,
                    name: c.name,
                    status: c.status,
                })
                .collect(),
        }
    }
}
