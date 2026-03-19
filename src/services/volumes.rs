use std::sync::Arc;

use anyhow::Result;

use crate::runtime::{ContainerRuntime, VolumeSummary};

/// Business-logic layer for volume operations.
pub struct VolumeService {
    runtime: Arc<Box<dyn ContainerRuntime>>,
}

impl VolumeService {
    pub fn new(runtime: Arc<Box<dyn ContainerRuntime>>) -> Self {
        Self { runtime }
    }

    /// List all volumes sorted alphabetically by name.
    pub async fn list(&self) -> Result<Vec<VolumeSummary>> {
        let mut volumes = self.runtime.list_volumes().await?;
        volumes.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        Ok(volumes)
    }

    /// Create a new named volume.
    pub async fn create(&self, name: &str) -> Result<VolumeSummary> {
        self.runtime.create_volume(name).await
    }

    /// Remove a volume by name.
    pub async fn remove(&self, name: &str, force: bool) -> Result<()> {
        self.runtime.remove_volume(name, force).await
    }

    /// Search volumes whose name contains the query (case-insensitive).
    pub async fn search(&self, query: &str) -> Result<Vec<VolumeSummary>> {
        let q = query.to_lowercase();
        let volumes = self.list().await?;
        Ok(volumes
            .into_iter()
            .filter(|v| v.name.to_lowercase().contains(&q))
            .collect())
    }

    /// Total number of volumes.
    pub async fn count(&self) -> Result<usize> {
        let volumes = self.runtime.list_volumes().await?;
        Ok(volumes.len())
    }
}
