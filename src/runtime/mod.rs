//! Container runtime abstraction layer.
//!
//! Provides a unified [`ContainerRuntime`] trait that abstracts over Docker and Podman,
//! along with domain types decoupled from any specific backend crate.

pub mod detect;
pub mod docker;
pub mod podman;

pub use detect::detect_runtime;
pub use docker::DockerRuntime;
pub use podman::PodmanRuntime;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

// ---------------------------------------------------------------------------
// RuntimeType
// ---------------------------------------------------------------------------

/// Which container engine is backing this runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RuntimeType {
    Docker,
    Podman,
}

impl fmt::Display for RuntimeType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RuntimeType::Docker => write!(f, "Docker"),
            RuntimeType::Podman => write!(f, "Podman"),
        }
    }
}

impl std::str::FromStr for RuntimeType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "docker" => Ok(RuntimeType::Docker),
            "podman" => Ok(RuntimeType::Podman),
            other => Err(anyhow::anyhow!("unknown runtime: {other}")),
        }
    }
}

// ---------------------------------------------------------------------------
// Domain types
// ---------------------------------------------------------------------------

/// Lightweight summary of a container, suitable for list views.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerSummary {
    pub id: String,
    /// Primary display name (first name with leading `/` stripped).
    pub name: String,
    /// All names assigned to the container.
    pub names: Vec<String>,
    pub image: String,
    pub image_id: String,
    pub command: String,
    /// Unix timestamp (seconds since epoch) when the container was created.
    pub created: i64,
    pub state: String,
    pub status: String,
    pub ports: Vec<PortMapping>,
    pub labels: HashMap<String, String>,
}

/// A single port mapping on a container.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortMapping {
    pub ip: Option<String>,
    pub private_port: u16,
    pub public_port: Option<u16>,
    pub port_type: String,
}

/// Full detail for a single container (inspect).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerDetail {
    pub id: String,
    pub name: String,
    pub image: String,
    pub created: String,
    pub state: ContainerState,
    pub path: String,
    pub args: Vec<String>,
    pub restart_count: i64,
    pub driver: String,
    pub platform: String,
    pub log_path: String,
    pub labels: HashMap<String, String>,
    pub env: Vec<String>,
}

/// Subset of container state fields.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerState {
    pub status: String,
    pub running: bool,
    pub paused: bool,
    pub restarting: bool,
    pub oom_killed: bool,
    pub dead: bool,
    pub pid: i64,
    pub exit_code: i64,
    pub started_at: String,
    pub finished_at: String,
}

/// Snapshot of live resource usage for a container.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerStats {
    pub container_id: String,
    pub name: String,
    pub cpu_percent: f64,
    pub memory_usage: u64,
    pub memory_limit: u64,
    pub memory_percent: f64,
    pub network_rx: u64,
    pub network_tx: u64,
    pub block_read: u64,
    pub block_write: u64,
    pub pids: u64,
}

/// Lightweight image summary for list views.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageSummary {
    pub id: String,
    pub repo_tags: Vec<String>,
    pub repo_digests: Vec<String>,
    /// Unix timestamp (seconds since epoch) when the image was created.
    pub created: i64,
    /// Total image size in bytes.
    pub size: u64,
    pub labels: HashMap<String, String>,
}

/// Full detail for a single image (inspect).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageDetail {
    pub id: String,
    pub repo_tags: Vec<String>,
    pub repo_digests: Vec<String>,
    pub created: String,
    pub size: u64,
    pub architecture: String,
    pub os: String,
    pub author: String,
    pub layers: Vec<String>,
}

/// Lightweight volume summary for list views.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeSummary {
    pub name: String,
    pub driver: String,
    pub mountpoint: String,
    pub labels: HashMap<String, String>,
    pub scope: String,
    pub created_at: String,
}

/// Lightweight network summary for list views.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkSummary {
    pub id: String,
    pub name: String,
    pub driver: String,
    pub scope: String,
    pub internal: bool,
    pub ipv6_enabled: bool,
    pub labels: HashMap<String, String>,
    /// Map of container ID to container name for containers connected to this network.
    pub containers: HashMap<String, String>,
}

/// Full detail for a single network (inspect).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkDetail {
    pub id: String,
    pub name: String,
    pub driver: String,
    pub scope: String,
    pub internal: bool,
    pub ipv6_enabled: bool,
    pub labels: HashMap<String, String>,
    pub containers: HashMap<String, String>,
    pub options: HashMap<String, String>,
    pub subnet: String,
    pub gateway: String,
}

/// Pod summary (Podman only).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PodSummary {
    pub id: String,
    pub name: String,
    pub status: String,
    pub created: String,
    pub infra_id: String,
    pub containers: Vec<PodContainerRef>,
}

/// A reference to a container inside a pod.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PodContainerRef {
    pub id: String,
    pub name: String,
    pub status: String,
}

/// High-level system / engine information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    pub runtime_type: RuntimeType,
    pub server_version: String,
    pub api_version: String,
    pub os: String,
    pub arch: String,
    pub kernel_version: String,
    pub memory_total: u64,
    pub cpus: u64,
    pub containers_total: i64,
    pub containers_running: i64,
    pub containers_paused: i64,
    pub containers_stopped: i64,
    pub images: i64,
    pub storage_driver: String,
}

// ---------------------------------------------------------------------------
// ContainerRuntime trait
// ---------------------------------------------------------------------------

/// Unified async interface for container engines (Docker, Podman).
///
/// All methods return domain types defined in this module so that callers
/// are decoupled from any backend crate such as `bollard`.
#[async_trait]
pub trait ContainerRuntime: Send + Sync {
    /// Which engine backs this instance.
    fn runtime_type(&self) -> RuntimeType;

    /// Whether this runtime natively supports pods.
    fn supports_pods(&self) -> bool {
        false
    }

    // -- Container operations -----------------------------------------------

    /// List all containers (running + stopped when `all` is true).
    async fn list_containers(&self, all: bool) -> anyhow::Result<Vec<ContainerSummary>>;

    /// Full inspect of a single container by ID or name.
    async fn inspect_container(&self, id: &str) -> anyhow::Result<ContainerDetail>;

    /// Start a stopped container.
    async fn start_container(&self, id: &str) -> anyhow::Result<()>;

    /// Stop a running container.
    async fn stop_container(&self, id: &str) -> anyhow::Result<()>;

    /// Restart a container.
    async fn restart_container(&self, id: &str) -> anyhow::Result<()>;

    /// Remove a container (force-remove if `force` is true).
    async fn remove_container(&self, id: &str, force: bool) -> anyhow::Result<()>;

    // -- Logs ---------------------------------------------------------------

    /// Fetch the last `tail` lines of logs for a container.
    /// When `tail` is `None`, a backend-chosen default is used.
    async fn container_logs(&self, id: &str, tail: Option<usize>) -> anyhow::Result<String>;

    // -- Stats --------------------------------------------------------------

    /// Snapshot of live resource usage for a container.
    async fn container_stats(&self, id: &str) -> anyhow::Result<ContainerStats>;

    // -- Image operations ---------------------------------------------------

    /// List local images.
    async fn list_images(&self) -> anyhow::Result<Vec<ImageSummary>>;

    /// Pull an image by reference (e.g. `nginx:latest`).
    async fn pull_image(&self, reference: &str) -> anyhow::Result<()>;

    /// Remove a local image by ID or name. When `force` is true the image is
    /// removed even if it is referenced by stopped containers.
    async fn remove_image(&self, id: &str, force: bool) -> anyhow::Result<()>;

    /// Inspect a single image, returning its summary metadata.
    async fn inspect_image(&self, id: &str) -> anyhow::Result<ImageSummary>;

    // -- Volume operations --------------------------------------------------

    /// List all volumes.
    async fn list_volumes(&self) -> anyhow::Result<Vec<VolumeSummary>>;

    /// Create a volume with the given name.
    async fn create_volume(&self, name: &str) -> anyhow::Result<VolumeSummary>;

    /// Remove a volume by name. When `force` is true, the volume is removed
    /// even if it is in use.
    async fn remove_volume(&self, name: &str, force: bool) -> anyhow::Result<()>;

    /// Full inspect of a single volume.
    async fn inspect_volume(&self, name: &str) -> anyhow::Result<VolumeSummary>;

    // -- Network operations -------------------------------------------------

    /// List all networks.
    async fn list_networks(&self) -> anyhow::Result<Vec<NetworkSummary>>;

    /// Create a network with the given name and driver. Returns a summary of
    /// the newly created network.
    async fn create_network(&self, name: &str, driver: &str) -> anyhow::Result<NetworkSummary>;

    /// Remove a network by ID or name.
    async fn remove_network(&self, id: &str) -> anyhow::Result<()>;

    /// Full inspect of a single network.
    async fn inspect_network(&self, id: &str) -> anyhow::Result<NetworkDetail>;

    // -- System -------------------------------------------------------------

    /// High-level system / engine information.
    async fn system_info(&self) -> anyhow::Result<SystemInfo>;

    // -- Pod operations (Podman only) ---------------------------------------

    /// List pods. Returns an error on runtimes that do not support pods.
    async fn list_pods(&self) -> anyhow::Result<Vec<PodSummary>> {
        anyhow::bail!(
            "pods are not supported by the {} runtime",
            self.runtime_type()
        )
    }

    /// Create a pod with the given name.
    async fn create_pod(&self, name: &str) -> anyhow::Result<PodSummary> {
        let _ = name;
        anyhow::bail!(
            "pods are not supported by the {} runtime",
            self.runtime_type()
        )
    }

    /// Remove a pod by ID or name.
    async fn remove_pod(&self, id: &str, force: bool) -> anyhow::Result<()> {
        let _ = (id, force);
        anyhow::bail!(
            "pods are not supported by the {} runtime",
            self.runtime_type()
        )
    }

    /// Start a pod.
    async fn start_pod(&self, id: &str) -> anyhow::Result<()> {
        let _ = id;
        anyhow::bail!(
            "pods are not supported by the {} runtime",
            self.runtime_type()
        )
    }

    /// Stop a pod.
    async fn stop_pod(&self, id: &str) -> anyhow::Result<()> {
        let _ = id;
        anyhow::bail!(
            "pods are not supported by the {} runtime",
            self.runtime_type()
        )
    }

    /// Inspect a pod.
    async fn inspect_pod(&self, id: &str) -> anyhow::Result<PodSummary> {
        let _ = id;
        anyhow::bail!(
            "pods are not supported by the {} runtime",
            self.runtime_type()
        )
    }
}
