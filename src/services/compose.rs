use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result};
use tokio::process::Command;

use crate::runtime::{ContainerRuntime, RuntimeType};

/// Known compose file names, checked in priority order.
const COMPOSE_FILENAMES: &[&str] = &[
    "docker-compose.yml",
    "docker-compose.yaml",
    "compose.yml",
    "compose.yaml",
];

/// Represents a detected Compose project on disk.
#[derive(Debug, Clone)]
pub struct ComposeProject {
    /// Absolute path to the compose file.
    pub file: PathBuf,
    /// Directory containing the compose file (used as project context).
    pub directory: PathBuf,
    /// Inferred project name (directory name).
    pub name: String,
}

/// Business-logic layer for docker-compose / podman-compose operations.
pub struct ComposeService {
    runtime: Arc<Box<dyn ContainerRuntime>>,
}

impl ComposeService {
    pub fn new(runtime: Arc<Box<dyn ContainerRuntime>>) -> Self {
        Self { runtime }
    }

    /// Scan a root directory (non-recursively) for compose files.
    /// Returns a list of detected projects.
    pub fn detect_projects(root: &Path) -> Result<Vec<ComposeProject>> {
        let mut projects = Vec::new();

        let entries = std::fs::read_dir(root)
            .with_context(|| format!("cannot read directory {}", root.display()))?;

        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            // Check each candidate file inside this subdirectory.
            for &filename in COMPOSE_FILENAMES {
                let candidate = path.join(filename);
                if candidate.is_file() {
                    let name = path
                        .file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_default();
                    projects.push(ComposeProject {
                        file: candidate,
                        directory: path.clone(),
                        name,
                    });
                    break; // one project per directory
                }
            }
        }

        // Also check root itself.
        for &filename in COMPOSE_FILENAMES {
            let candidate = root.join(filename);
            if candidate.is_file() {
                let name = root
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "project".to_string());
                projects.push(ComposeProject {
                    file: candidate,
                    directory: root.to_path_buf(),
                    name,
                });
                break;
            }
        }

        projects.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(projects)
    }

    /// Bring a compose project up (detached).
    pub async fn up(&self, project: &ComposeProject) -> Result<String> {
        self.run_compose(project, &["up", "-d"]).await
    }

    /// Tear a compose project down (remove containers and default network).
    pub async fn down(&self, project: &ComposeProject) -> Result<String> {
        self.run_compose(project, &["down"]).await
    }

    /// Restart all services in a compose project.
    pub async fn restart(&self, project: &ComposeProject) -> Result<String> {
        self.run_compose(project, &["restart"]).await
    }

    /// Pull latest images for all services in a compose project.
    pub async fn pull(&self, project: &ComposeProject) -> Result<String> {
        self.run_compose(project, &["pull"]).await
    }

    /// View combined logs for a compose project.
    pub async fn logs(&self, project: &ComposeProject, tail: usize) -> Result<String> {
        let tail_str = tail.to_string();
        self.run_compose(project, &["logs", "--tail", &tail_str])
            .await
    }

    // -----------------------------------------------------------------------
    // Internal helpers
    // -----------------------------------------------------------------------

    /// Build and execute the appropriate compose command based on the active runtime.
    async fn run_compose(&self, project: &ComposeProject, args: &[&str]) -> Result<String> {
        let (program, base_args) = match self.runtime.runtime_type() {
            RuntimeType::Docker => ("docker", vec!["compose"]),
            RuntimeType::Podman => ("podman-compose", vec![]),
        };

        let mut cmd = Command::new(program);
        cmd.args(&base_args);
        cmd.arg("-f").arg(&project.file);
        cmd.args(args);
        cmd.current_dir(&project.directory);

        let output = cmd
            .output()
            .await
            .with_context(|| format!("failed to execute {} compose", program))?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).into_owned())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("{} compose failed: {}", program, stderr)
        }
    }
}
