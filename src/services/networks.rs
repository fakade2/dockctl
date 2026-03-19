use std::sync::Arc;

use anyhow::Result;

use crate::runtime::{ContainerRuntime, NetworkSummary};

/// Business-logic layer for network operations.
pub struct NetworkService {
    runtime: Arc<Box<dyn ContainerRuntime>>,
}

impl NetworkService {
    pub fn new(runtime: Arc<Box<dyn ContainerRuntime>>) -> Self {
        Self { runtime }
    }

    /// List all networks sorted by name.
    pub async fn list(&self) -> Result<Vec<NetworkSummary>> {
        let mut networks = self.runtime.list_networks().await?;
        networks.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        Ok(networks)
    }

    /// Create a new network with the given driver (e.g. "bridge", "overlay").
    pub async fn create(&self, name: &str, driver: &str) -> Result<NetworkSummary> {
        self.runtime.create_network(name, driver).await
    }

    /// Remove a network by id.
    pub async fn remove(&self, id: &str) -> Result<()> {
        self.runtime.remove_network(id).await
    }

    /// List only user-defined networks (excludes built-in bridge, host, none).
    pub async fn list_user_defined(&self) -> Result<Vec<NetworkSummary>> {
        let networks = self.list().await?;
        let builtin = ["bridge", "host", "none"];
        Ok(networks
            .into_iter()
            .filter(|n| !builtin.contains(&n.name.as_str()))
            .collect())
    }

    /// Find networks that have at least one connected container.
    pub async fn list_active(&self) -> Result<Vec<NetworkSummary>> {
        let networks = self.runtime.list_networks().await?;
        Ok(networks
            .into_iter()
            .filter(|n| !n.containers.is_empty())
            .collect())
    }
}
