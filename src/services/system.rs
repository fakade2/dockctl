use std::sync::Arc;

use anyhow::Result;

use crate::runtime::{ContainerRuntime, SystemInfo};

/// A human-readable disk-usage summary.
#[derive(Debug, Clone)]
pub struct DiskUsageSummary {
    pub images_count: usize,
    pub images_total_bytes: u64,
    pub images_display: String,
    pub volumes_count: usize,
    pub containers_count: usize,
}

/// Business-logic layer for system / engine information.
pub struct SystemService {
    runtime: Arc<Box<dyn ContainerRuntime>>,
}

impl SystemService {
    pub fn new(runtime: Arc<Box<dyn ContainerRuntime>>) -> Self {
        Self { runtime }
    }

    /// Retrieve high-level engine info.
    pub async fn info(&self) -> Result<SystemInfo> {
        self.runtime.system_info().await
    }

    /// Aggregate disk usage across images, volumes, and containers.
    pub async fn disk_usage_summary(&self) -> Result<DiskUsageSummary> {
        // Fetch all resources concurrently.
        let (images, volumes, containers) = tokio::try_join!(
            self.runtime.list_images(),
            self.runtime.list_volumes(),
            self.runtime.list_containers(true),
        )?;

        let images_total_bytes: u64 = images.iter().map(|i| i.size).sum();

        Ok(DiskUsageSummary {
            images_count: images.len(),
            images_total_bytes,
            images_display: bytesize::ByteSize::b(images_total_bytes).to_string(),
            volumes_count: volumes.len(),
            containers_count: containers.len(),
        })
    }

    /// Quick health check: can we reach the runtime?
    pub async fn is_healthy(&self) -> bool {
        self.runtime.system_info().await.is_ok()
    }

    /// Format total memory from SystemInfo into a human-readable string.
    pub async fn memory_display(&self) -> Result<String> {
        let info = self.runtime.system_info().await?;
        Ok(bytesize::ByteSize::b(info.memory_total).to_string())
    }
}
