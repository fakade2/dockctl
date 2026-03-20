use std::sync::Arc;

use anyhow::Result;

use crate::runtime::{ContainerRuntime, ImageSummary};

/// Business-logic layer for image operations.
pub struct ImageService {
    runtime: Arc<Box<dyn ContainerRuntime>>,
}

impl ImageService {
    pub fn new(runtime: Arc<Box<dyn ContainerRuntime>>) -> Self {
        Self { runtime }
    }

    /// List all images sorted by creation time (newest first).
    pub async fn list(&self) -> Result<Vec<ImageSummary>> {
        let mut images = self.runtime.list_images().await?;
        images.sort_by(|a, b| b.created.cmp(&a.created));
        Ok(images)
    }

    /// Search images whose repo tags match the query (case-insensitive).
    pub async fn search(&self, query: &str) -> Result<Vec<ImageSummary>> {
        let q = query.to_lowercase();
        let images = self.list().await?;
        Ok(images
            .into_iter()
            .filter(|img| {
                img.repo_tags
                    .iter()
                    .any(|tag| tag.to_lowercase().contains(&q))
            })
            .collect())
    }

    pub async fn pull(&self, reference: &str) -> Result<()> {
        self.runtime.pull_image(reference).await
    }

    pub async fn remove(&self, id: &str, force: bool) -> Result<()> {
        self.runtime.remove_image(id, force).await
    }

    /// Detailed metadata for a single image.
    pub async fn get_detail(&self, id: &str) -> Result<ImageSummary> {
        self.runtime.inspect_image(id).await
    }

    /// Total disk space consumed by all local images (in bytes).
    pub async fn total_disk_usage(&self) -> Result<u64> {
        let images = self.runtime.list_images().await?;
        Ok(images.iter().map(|img| img.size).sum())
    }

    /// List images that have no repo tag (dangling / `<none>:<none>`).
    pub async fn list_dangling(&self) -> Result<Vec<ImageSummary>> {
        let images = self.runtime.list_images().await?;
        Ok(images
            .into_iter()
            .filter(|img| {
                img.repo_tags.is_empty() || img.repo_tags.iter().all(|t| t == "<none>:<none>")
            })
            .collect())
    }

    /// Formats disk usage into a human-readable string (e.g. "3.2 GB").
    pub async fn disk_usage_display(&self) -> Result<String> {
        let bytes = self.total_disk_usage().await?;
        Ok(bytesize::ByteSize::b(bytes).to_string())
    }
}
