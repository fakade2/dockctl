use std::sync::Arc;

use anyhow::Result;

use crate::runtime::{ContainerDetail, ContainerRuntime, ContainerStats, ContainerSummary};

/// Business-logic layer for container operations.
pub struct ContainerService {
    runtime: Arc<Box<dyn ContainerRuntime>>,
}

impl ContainerService {
    pub fn new(runtime: Arc<Box<dyn ContainerRuntime>>) -> Self {
        Self { runtime }
    }

    /// List every container (running + stopped), sorted by creation time descending.
    pub async fn list_all(&self) -> Result<Vec<ContainerSummary>> {
        let mut containers = self.runtime.list_containers(true).await?;
        containers.sort_by(|a, b| b.created.cmp(&a.created));
        Ok(containers)
    }

    /// List only running containers, sorted by name.
    pub async fn list_running(&self) -> Result<Vec<ContainerSummary>> {
        let mut containers = self.runtime.list_containers(false).await?;
        containers.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        Ok(containers)
    }

    /// Filter containers whose name or image matches the given query (case-insensitive).
    pub async fn search(&self, query: &str) -> Result<Vec<ContainerSummary>> {
        let q = query.to_lowercase();
        let containers = self.list_all().await?;
        Ok(containers
            .into_iter()
            .filter(|c| c.name.to_lowercase().contains(&q) || c.image.to_lowercase().contains(&q))
            .collect())
    }

    /// Full inspection details for a single container.
    pub async fn get_detail(&self, id: &str) -> Result<ContainerDetail> {
        self.runtime.inspect_container(id).await
    }

    pub async fn start(&self, id: &str) -> Result<()> {
        self.runtime.start_container(id).await
    }

    pub async fn stop(&self, id: &str) -> Result<()> {
        self.runtime.stop_container(id).await
    }

    pub async fn restart(&self, id: &str) -> Result<()> {
        self.runtime.restart_container(id).await
    }

    /// Remove a container. Stopped containers are removed normally; running
    /// containers require `force = true`.
    pub async fn remove(&self, id: &str, force: bool) -> Result<()> {
        self.runtime.remove_container(id, force).await
    }

    /// Fetch the last `tail` lines of a container's logs.
    /// Defaults to 200 lines when `tail` is `None`.
    pub async fn get_logs(&self, id: &str, tail: Option<usize>) -> Result<String> {
        self.runtime
            .container_logs(id, Some(tail.unwrap_or(200)))
            .await
    }

    /// Point-in-time resource stats for a container.
    pub async fn get_stats(&self, id: &str) -> Result<ContainerStats> {
        self.runtime.container_stats(id).await
    }

    /// Collect stats for every running container in one shot.
    pub async fn get_all_stats(&self) -> Result<Vec<ContainerStats>> {
        let running = self.runtime.list_containers(false).await?;
        let mut stats = Vec::with_capacity(running.len());
        for c in &running {
            match self.runtime.container_stats(&c.id).await {
                Ok(s) => stats.push(s),
                // Skip containers that disappeared between list and stats.
                Err(_) => continue,
            }
        }
        Ok(stats)
    }

    /// Group containers by their Compose project label, if any.
    /// Containers without a project label are grouped under `"(standalone)"`.
    pub async fn group_by_compose_project(
        &self,
    ) -> Result<std::collections::HashMap<String, Vec<ContainerSummary>>> {
        let all = self.list_all().await?;
        let mut groups: std::collections::HashMap<String, Vec<ContainerSummary>> =
            std::collections::HashMap::new();

        for container in all {
            let detail = self.runtime.inspect_container(&container.id).await;
            let project = detail
                .ok()
                .and_then(|d| d.labels.get("com.docker.compose.project").cloned())
                .unwrap_or_else(|| "(standalone)".to_string());
            groups.entry(project).or_default().push(container);
        }
        Ok(groups)
    }
}
