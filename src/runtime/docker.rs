//! Docker backend — implements [`ContainerRuntime`] using the `bollard` crate
//! connected to the Docker daemon socket.

use std::collections::HashMap;

use anyhow::{Context, Result};
use async_trait::async_trait;
use bollard::Docker;
use bollard::container::{
    ListContainersOptions, LogsOptions, RemoveContainerOptions, StatsOptions,
};
use bollard::image::{CreateImageOptions, ListImagesOptions, RemoveImageOptions};
use bollard::network::{CreateNetworkOptions, InspectNetworkOptions, ListNetworksOptions};
use bollard::volume::{CreateVolumeOptions, ListVolumesOptions, RemoveVolumeOptions};
use futures_util::TryStreamExt;

use super::{
    ContainerDetail, ContainerRuntime, ContainerState, ContainerStats, ContainerSummary,
    ImageSummary, NetworkDetail, NetworkSummary, PortMapping, RuntimeType, SystemInfo,
    VolumeSummary,
};

// ---------------------------------------------------------------------------
// DockerRuntime
// ---------------------------------------------------------------------------

/// A [`ContainerRuntime`] backed by the Docker Engine API via `bollard`.
pub struct DockerRuntime {
    client: Docker,
}

impl DockerRuntime {
    /// Connect to the default Docker socket (`/var/run/docker.sock` on Linux/macOS).
    pub fn connect_default() -> Result<Self> {
        let client =
            Docker::connect_with_socket_defaults().context("failed to connect to Docker socket")?;
        Ok(Self { client })
    }

    /// Connect to a Docker-compatible socket at an explicit path.
    pub fn connect_with_socket(path: &str) -> Result<Self> {
        let client = Docker::connect_with_socket(path, 120, bollard::API_DEFAULT_VERSION)
            .with_context(|| format!("failed to connect to Docker socket at {path}"))?;
        Ok(Self { client })
    }

    /// Create from an already-connected `bollard::Docker` handle.
    pub fn from_client(client: Docker) -> Self {
        Self { client }
    }

    /// Borrow the underlying bollard client (useful for advanced / escape-hatch calls).
    pub fn client(&self) -> &Docker {
        &self.client
    }
}

// ---------------------------------------------------------------------------
// Helpers — bollard → domain conversions
// ---------------------------------------------------------------------------

fn convert_container_summary(c: bollard::models::ContainerSummary) -> ContainerSummary {
    let names: Vec<String> = c
        .names
        .unwrap_or_default()
        .into_iter()
        .map(|n| n.trim_start_matches('/').to_string())
        .collect();

    let primary_name = names.first().cloned().unwrap_or_default();

    let ports = c
        .ports
        .unwrap_or_default()
        .into_iter()
        .map(|p| PortMapping {
            ip: p.ip,
            private_port: p.private_port,
            public_port: p.public_port,
            port_type: p.typ.map(|t| t.to_string()).unwrap_or_default(),
        })
        .collect();

    ContainerSummary {
        id: c.id.unwrap_or_default(),
        name: primary_name,
        names,
        image: c.image.unwrap_or_default(),
        image_id: c.image_id.unwrap_or_default(),
        command: c.command.unwrap_or_default(),
        created: c.created.unwrap_or(0),
        state: c.state.unwrap_or_default(),
        status: c.status.unwrap_or_default(),
        ports,
        labels: c.labels.unwrap_or_default(),
    }
}

fn convert_container_detail(resp: bollard::models::ContainerInspectResponse) -> ContainerDetail {
    let state = resp.state.as_ref();
    let config = resp.config.as_ref();

    ContainerDetail {
        id: resp.id.unwrap_or_default(),
        name: resp
            .name
            .unwrap_or_default()
            .trim_start_matches('/')
            .to_string(),
        image: resp.image.unwrap_or_default(),
        created: resp.created.unwrap_or_default(),
        state: ContainerState {
            status: state
                .and_then(|s| s.status.as_ref())
                .map(|s| s.to_string())
                .unwrap_or_default(),
            running: state.and_then(|s| s.running).unwrap_or(false),
            paused: state.and_then(|s| s.paused).unwrap_or(false),
            restarting: state.and_then(|s| s.restarting).unwrap_or(false),
            oom_killed: state.and_then(|s| s.oom_killed).unwrap_or(false),
            dead: state.and_then(|s| s.dead).unwrap_or(false),
            pid: state.and_then(|s| s.pid).unwrap_or(0),
            exit_code: state.and_then(|s| s.exit_code).unwrap_or(0),
            started_at: state.and_then(|s| s.started_at.clone()).unwrap_or_default(),
            finished_at: state
                .and_then(|s| s.finished_at.clone())
                .unwrap_or_default(),
        },
        path: resp.path.unwrap_or_default(),
        args: resp.args.unwrap_or_default(),
        restart_count: resp.restart_count.unwrap_or(0),
        driver: resp.driver.unwrap_or_default(),
        platform: resp.platform.unwrap_or_default(),
        log_path: resp.log_path.unwrap_or_default(),
        labels: config.and_then(|c| c.labels.clone()).unwrap_or_default(),
        env: config.and_then(|c| c.env.clone()).unwrap_or_default(),
    }
}

fn calculate_cpu_percent(stats: &bollard::container::Stats) -> f64 {
    let cpu_delta = stats.cpu_stats.cpu_usage.total_usage as f64
        - stats.precpu_stats.cpu_usage.total_usage as f64;
    let system_delta = stats
        .cpu_stats
        .system_cpu_usage
        .unwrap_or(0)
        .wrapping_sub(stats.precpu_stats.system_cpu_usage.unwrap_or(0))
        as f64;
    let online_cpus = stats.cpu_stats.online_cpus.unwrap_or(1).max(1) as f64;

    if system_delta > 0.0 && cpu_delta >= 0.0 {
        (cpu_delta / system_delta) * online_cpus * 100.0
    } else {
        0.0
    }
}

fn aggregate_network_stats(stats: &bollard::container::Stats) -> (u64, u64) {
    let mut rx: u64 = 0;
    let mut tx: u64 = 0;

    if let Some(ref networks) = stats.networks {
        for net in networks.values() {
            rx = rx.saturating_add(net.rx_bytes);
            tx = tx.saturating_add(net.tx_bytes);
        }
    } else if let Some(ref net) = stats.network {
        rx = net.rx_bytes;
        tx = net.tx_bytes;
    }

    (rx, tx)
}

fn aggregate_blkio(stats: &bollard::container::Stats) -> (u64, u64) {
    let mut read: u64 = 0;
    let mut write: u64 = 0;

    if let Some(ref entries) = stats.blkio_stats.io_service_bytes_recursive {
        for entry in entries {
            match entry.op.to_lowercase().as_str() {
                "read" => read = read.saturating_add(entry.value),
                "write" => write = write.saturating_add(entry.value),
                _ => {}
            }
        }
    }
    (read, write)
}

fn convert_image_summary(img: bollard::models::ImageSummary) -> ImageSummary {
    ImageSummary {
        id: img.id,
        repo_tags: img.repo_tags,
        repo_digests: img.repo_digests,
        created: img.created,
        size: img.size as u64,
        labels: img.labels,
    }
}

fn convert_image_inspect_to_summary(img: bollard::models::ImageInspect) -> ImageSummary {
    // BollardDate is a String when neither chrono nor time features are enabled.
    // Parse the RFC 3339 timestamp to a unix epoch for our domain type.
    let created = img
        .created
        .as_deref()
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.timestamp())
        .unwrap_or(0);

    ImageSummary {
        id: img.id.unwrap_or_default(),
        repo_tags: img.repo_tags.unwrap_or_default(),
        repo_digests: img.repo_digests.unwrap_or_default(),
        created,
        size: img.size.unwrap_or(0) as u64,
        labels: img.config.and_then(|c| c.labels).unwrap_or_default(),
    }
}

fn convert_volume(vol: bollard::models::Volume) -> VolumeSummary {
    VolumeSummary {
        name: vol.name,
        driver: vol.driver,
        mountpoint: vol.mountpoint,
        labels: vol.labels,
        scope: vol
            .scope
            .map(|s| s.to_string())
            .unwrap_or_else(|| "local".to_string()),
        created_at: vol.created_at.map(|d| d.to_string()).unwrap_or_default(),
    }
}

fn convert_network_summary(net: &bollard::models::Network) -> NetworkSummary {
    let containers: HashMap<String, String> = net
        .containers
        .as_ref()
        .map(|c| {
            c.iter()
                .map(|(id, info)| (id.clone(), info.name.clone().unwrap_or_default()))
                .collect()
        })
        .unwrap_or_default();

    NetworkSummary {
        id: net.id.clone().unwrap_or_default(),
        name: net.name.clone().unwrap_or_default(),
        driver: net.driver.clone().unwrap_or_default(),
        scope: net.scope.clone().unwrap_or_default(),
        internal: net.internal.unwrap_or(false),
        ipv6_enabled: net.enable_ipv6.unwrap_or(false),
        labels: net.labels.clone().unwrap_or_default(),
        containers,
    }
}

fn convert_network_detail(net: bollard::models::Network) -> NetworkDetail {
    let containers: HashMap<String, String> = net
        .containers
        .as_ref()
        .map(|c| {
            c.iter()
                .map(|(id, info)| (id.clone(), info.name.clone().unwrap_or_default()))
                .collect()
        })
        .unwrap_or_default();

    // Extract subnet and gateway from first IPAM config entry.
    let (subnet, gateway) = net
        .ipam
        .as_ref()
        .and_then(|ipam| ipam.config.as_ref())
        .and_then(|cfgs| cfgs.first())
        .map(|cfg| {
            (
                cfg.subnet.clone().unwrap_or_default(),
                cfg.gateway.clone().unwrap_or_default(),
            )
        })
        .unwrap_or_default();

    NetworkDetail {
        id: net.id.unwrap_or_default(),
        name: net.name.unwrap_or_default(),
        driver: net.driver.unwrap_or_default(),
        scope: net.scope.unwrap_or_default(),
        internal: net.internal.unwrap_or(false),
        ipv6_enabled: net.enable_ipv6.unwrap_or(false),
        labels: net.labels.unwrap_or_default(),
        containers,
        options: net.options.unwrap_or_default(),
        subnet,
        gateway,
    }
}

// ---------------------------------------------------------------------------
// ContainerRuntime implementation
// ---------------------------------------------------------------------------

#[async_trait]
impl ContainerRuntime for DockerRuntime {
    fn runtime_type(&self) -> RuntimeType {
        RuntimeType::Docker
    }

    // -- Containers ---------------------------------------------------------

    async fn list_containers(&self, all: bool) -> Result<Vec<ContainerSummary>> {
        let opts = ListContainersOptions::<String> {
            all,
            ..Default::default()
        };
        let containers = self
            .client
            .list_containers(Some(opts))
            .await
            .context("failed to list containers")?;

        Ok(containers
            .into_iter()
            .map(convert_container_summary)
            .collect())
    }

    async fn inspect_container(&self, id: &str) -> Result<ContainerDetail> {
        let resp = self
            .client
            .inspect_container(id, None)
            .await
            .with_context(|| format!("failed to inspect container {id}"))?;

        Ok(convert_container_detail(resp))
    }

    async fn start_container(&self, id: &str) -> Result<()> {
        self.client
            .start_container::<String>(id, None)
            .await
            .with_context(|| format!("failed to start container {id}"))?;
        Ok(())
    }

    async fn stop_container(&self, id: &str) -> Result<()> {
        self.client
            .stop_container(id, None)
            .await
            .with_context(|| format!("failed to stop container {id}"))?;
        Ok(())
    }

    async fn restart_container(&self, id: &str) -> Result<()> {
        self.client
            .restart_container(id, None)
            .await
            .with_context(|| format!("failed to restart container {id}"))?;
        Ok(())
    }

    async fn remove_container(&self, id: &str, force: bool) -> Result<()> {
        let opts = RemoveContainerOptions {
            force,
            ..Default::default()
        };
        self.client
            .remove_container(id, Some(opts))
            .await
            .with_context(|| format!("failed to remove container {id}"))?;
        Ok(())
    }

    // -- Logs ---------------------------------------------------------------

    async fn container_logs(&self, id: &str, tail: Option<usize>) -> Result<String> {
        let tail_str = tail.unwrap_or(200).to_string();
        let opts = LogsOptions::<String> {
            stdout: true,
            stderr: true,
            tail: tail_str,
            ..Default::default()
        };

        let lines: Vec<String> = self
            .client
            .logs(id, Some(opts))
            .try_collect::<Vec<_>>()
            .await
            .with_context(|| format!("failed to fetch logs for container {id}"))?
            .into_iter()
            .map(|output| output.to_string().trim_end().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        Ok(lines.join("\n"))
    }

    // -- Stats --------------------------------------------------------------

    async fn container_stats(&self, id: &str) -> Result<ContainerStats> {
        let opts = StatsOptions {
            stream: false,
            one_shot: true,
        };

        let stat = self
            .client
            .stats(id, Some(opts))
            .try_next()
            .await
            .with_context(|| format!("failed to fetch stats for container {id}"))?
            .with_context(|| format!("no stats returned for container {id}"))?;

        let cpu_percent = calculate_cpu_percent(&stat);
        let mem_usage = stat.memory_stats.usage.unwrap_or(0);
        let mem_limit = stat.memory_stats.limit.unwrap_or(0);
        let mem_percent = if mem_limit > 0 {
            (mem_usage as f64 / mem_limit as f64) * 100.0
        } else {
            0.0
        };
        let (net_rx, net_tx) = aggregate_network_stats(&stat);
        let (blk_read, blk_write) = aggregate_blkio(&stat);

        Ok(ContainerStats {
            container_id: stat.id,
            name: stat.name,
            cpu_percent,
            memory_usage: mem_usage,
            memory_limit: mem_limit,
            memory_percent: mem_percent,
            network_rx: net_rx,
            network_tx: net_tx,
            block_read: blk_read,
            block_write: blk_write,
            pids: stat.pids_stats.current.unwrap_or(0),
        })
    }

    // -- Images -------------------------------------------------------------

    async fn list_images(&self) -> Result<Vec<ImageSummary>> {
        let opts = ListImagesOptions::<String> {
            all: false,
            ..Default::default()
        };
        let images = self
            .client
            .list_images(Some(opts))
            .await
            .context("failed to list images")?;

        Ok(images.into_iter().map(convert_image_summary).collect())
    }

    async fn pull_image(&self, reference: &str) -> Result<()> {
        // Split reference into image and tag (e.g. "nginx:latest").
        let (image, tag) = match reference.rsplit_once(':') {
            Some((img, t)) if !img.contains('/') || !t.contains('/') => {
                (img.to_string(), t.to_string())
            }
            _ => (reference.to_string(), "latest".to_string()),
        };

        let opts = CreateImageOptions {
            from_image: image,
            tag,
            ..Default::default()
        };

        // Consume the entire progress stream to completion.
        self.client
            .create_image(Some(opts), None, None)
            .try_collect::<Vec<_>>()
            .await
            .with_context(|| format!("failed to pull image {reference}"))?;

        Ok(())
    }

    async fn remove_image(&self, id: &str, force: bool) -> Result<()> {
        let opts = RemoveImageOptions {
            force,
            noprune: false,
        };
        self.client
            .remove_image(id, Some(opts), None)
            .await
            .with_context(|| format!("failed to remove image {id}"))?;
        Ok(())
    }

    async fn inspect_image(&self, id: &str) -> Result<ImageSummary> {
        let img = self
            .client
            .inspect_image(id)
            .await
            .with_context(|| format!("failed to inspect image {id}"))?;

        Ok(convert_image_inspect_to_summary(img))
    }

    // -- Volumes ------------------------------------------------------------

    async fn list_volumes(&self) -> Result<Vec<VolumeSummary>> {
        let resp = self
            .client
            .list_volumes(None::<ListVolumesOptions<String>>)
            .await
            .context("failed to list volumes")?;

        Ok(resp
            .volumes
            .unwrap_or_default()
            .into_iter()
            .map(convert_volume)
            .collect())
    }

    async fn create_volume(&self, name: &str) -> Result<VolumeSummary> {
        let opts = CreateVolumeOptions {
            name,
            ..Default::default()
        };
        let vol = self
            .client
            .create_volume(opts)
            .await
            .with_context(|| format!("failed to create volume {name}"))?;

        Ok(convert_volume(vol))
    }

    async fn remove_volume(&self, name: &str, force: bool) -> Result<()> {
        let opts = RemoveVolumeOptions { force };
        self.client
            .remove_volume(name, Some(opts))
            .await
            .with_context(|| format!("failed to remove volume {name}"))?;
        Ok(())
    }

    async fn inspect_volume(&self, name: &str) -> Result<VolumeSummary> {
        let vol = self
            .client
            .inspect_volume(name)
            .await
            .with_context(|| format!("failed to inspect volume {name}"))?;

        Ok(convert_volume(vol))
    }

    // -- Networks -----------------------------------------------------------

    async fn list_networks(&self) -> Result<Vec<NetworkSummary>> {
        let nets = self
            .client
            .list_networks(None::<ListNetworksOptions<String>>)
            .await
            .context("failed to list networks")?;

        Ok(nets.iter().map(convert_network_summary).collect())
    }

    async fn create_network(&self, name: &str, driver: &str) -> Result<NetworkSummary> {
        let opts = CreateNetworkOptions {
            name: name.to_string(),
            driver: driver.to_string(),
            ..Default::default()
        };
        let resp = self
            .client
            .create_network(opts)
            .await
            .with_context(|| format!("failed to create network {name}"))?;

        // Fetch the newly created network to return a full summary.
        let net = self
            .client
            .inspect_network(&resp.id, None::<InspectNetworkOptions<String>>)
            .await
            .with_context(|| format!("failed to inspect newly created network {name}"))?;

        Ok(convert_network_summary(&net))
    }

    async fn remove_network(&self, id: &str) -> Result<()> {
        self.client
            .remove_network(id)
            .await
            .with_context(|| format!("failed to remove network {id}"))?;
        Ok(())
    }

    async fn inspect_network(&self, id: &str) -> Result<NetworkDetail> {
        let net = self
            .client
            .inspect_network(id, None::<InspectNetworkOptions<String>>)
            .await
            .with_context(|| format!("failed to inspect network {id}"))?;

        Ok(convert_network_detail(net))
    }

    // -- System -------------------------------------------------------------

    async fn system_info(&self) -> Result<SystemInfo> {
        let version = self
            .client
            .version()
            .await
            .context("failed to fetch Docker version")?;

        let info = self
            .client
            .info()
            .await
            .context("failed to fetch Docker system info")?;

        Ok(SystemInfo {
            runtime_type: RuntimeType::Docker,
            server_version: version.version.unwrap_or_default(),
            api_version: version.api_version.unwrap_or_default(),
            os: info.operating_system.unwrap_or_default(),
            arch: info.architecture.unwrap_or_default(),
            kernel_version: info.kernel_version.unwrap_or_default(),
            memory_total: info.mem_total.unwrap_or(0) as u64,
            cpus: info.ncpu.unwrap_or(0) as u64,
            containers_total: info.containers.unwrap_or(0),
            containers_running: info.containers_running.unwrap_or(0),
            containers_paused: info.containers_paused.unwrap_or(0),
            containers_stopped: info.containers_stopped.unwrap_or(0),
            images: info.images.unwrap_or(0),
            storage_driver: info.driver.unwrap_or_default(),
        })
    }
}
