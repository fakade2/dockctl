use std::sync::Arc;

use anyhow::{Result, bail};

use crate::runtime::{ContainerRuntime, PodSummary};

/// Business-logic layer for pod operations (Podman only).
pub struct PodService {
    runtime: Arc<Box<dyn ContainerRuntime>>,
}

impl PodService {
    pub fn new(runtime: Arc<Box<dyn ContainerRuntime>>) -> Self {
        Self { runtime }
    }

    /// List all pods, sorted by creation time descending.
    /// Returns an error if the runtime does not support pods.
    pub async fn list(&self) -> Result<Vec<PodSummary>> {
        self.ensure_pods_supported()?;
        let mut pods = self.runtime.list_pods().await?;
        pods.sort_by(|a, b| b.created.cmp(&a.created));
        Ok(pods)
    }

    /// Create a new pod with the given name.
    pub async fn create(&self, name: &str) -> Result<PodSummary> {
        self.ensure_pods_supported()?;
        self.runtime.create_pod(name).await
    }

    pub async fn start(&self, id: &str) -> Result<()> {
        self.ensure_pods_supported()?;
        self.runtime.start_pod(id).await
    }

    pub async fn stop(&self, id: &str) -> Result<()> {
        self.ensure_pods_supported()?;
        self.runtime.stop_pod(id).await
    }

    /// Remove a pod. Use `force = true` to remove a running pod.
    pub async fn remove(&self, id: &str, force: bool) -> Result<()> {
        self.ensure_pods_supported()?;
        self.runtime.remove_pod(id, force).await
    }

    /// Search pods whose name contains the query (case-insensitive).
    pub async fn search(&self, query: &str) -> Result<Vec<PodSummary>> {
        let q = query.to_lowercase();
        let pods = self.list().await?;
        Ok(pods
            .into_iter()
            .filter(|p| p.name.to_lowercase().contains(&q))
            .collect())
    }

    /// Whether the underlying runtime has pod support.
    pub fn is_supported(&self) -> bool {
        self.runtime.supports_pods()
    }

    /// Guard that returns a clear error when pods are not supported.
    fn ensure_pods_supported(&self) -> Result<()> {
        if !self.runtime.supports_pods() {
            bail!(
                "pods are not supported by the current runtime ({:?})",
                self.runtime.runtime_type()
            );
        }
        Ok(())
    }
}
