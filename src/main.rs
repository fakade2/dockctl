pub mod app;
pub mod config;
pub mod display;
pub mod event;
pub mod runtime;
pub mod services;
pub mod themes;
pub mod tmux;

use std::io;
use std::sync::Arc;

use anyhow::{Context, Result};
use clap::Parser;
use crossterm::{
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::prelude::*;
use tracing_subscriber::EnvFilter;

use crate::app::{Alert, AlertLevel, App, PanelId};
use crate::event::{AppEvent, EventHandler};
use crate::runtime::{RuntimeType, detect_runtime};
use crate::services::{
    ContainerService, ImageService, NetworkService, PodService, SystemService, VolumeService,
};
use crate::themes::ThemeAction;
use sysinfo::System;

use crate::display::preview;

#[derive(Parser, Debug)]
#[command(
    name = "dockctl",
    version,
    about = "Universal container runtime TUI — Docker & Podman"
)]
struct Cli {
    /// Container runtime to use (auto-detected if omitted)
    #[arg(short, long, value_enum)]
    runtime: Option<RuntimeType>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Logging
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_target(false)
        .init();

    let cli = Cli::parse();
    let config = config::load()?;

    // Detect and connect to container runtime
    let runtime = detect_runtime(cli.runtime)
        .await
        .context("Failed to connect to any container runtime. Is Docker or Podman running?")?;

    let supports_pods = runtime.supports_pods();
    let runtime = Arc::new(runtime);

    // Build services
    let container_svc = ContainerService::new(Arc::clone(&runtime));
    let image_svc = ImageService::new(Arc::clone(&runtime));
    let volume_svc = VolumeService::new(Arc::clone(&runtime));
    let network_svc = NetworkService::new(Arc::clone(&runtime));
    let pod_svc = PodService::new(Arc::clone(&runtime));
    let system_svc = SystemService::new(Arc::clone(&runtime));

    // Initialize app state
    let mut app = App::new(supports_pods);

    // Initialize sysinfo for host metrics
    let mut sys = System::new_all();
    sys.refresh_all();
    refresh_host_stats(&mut app, &mut sys);

    // Initial data load
    load_data(
        &mut app,
        &container_svc,
        &image_svc,
        &volume_svc,
        &network_svc,
        &pod_svc,
        &system_svc,
    )
    .await;
    compute_alerts(&mut app);

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Event handler
    let mut events = EventHandler::new(config.general.refresh_interval_ms);

    // Create theme from config
    let theme = themes::create_theme(&config.general.theme);

    // Main loop
    loop {
        // Draw via theme
        terminal.draw(|frame| theme.draw(frame, &app, &config))?;

        // Handle events
        match events.next().await? {
            AppEvent::Tick => {
                // Refresh data for the active panel on tick
                load_panel_data(
                    &mut app,
                    &container_svc,
                    &image_svc,
                    &volume_svc,
                    &network_svc,
                    &pod_svc,
                    &system_svc,
                )
                .await;
                refresh_host_stats(&mut app, &mut sys);
                compute_alerts(&mut app);
            }
            AppEvent::Key(key) => {
                // All key handling delegated to the active theme
                match theme.handle_key(key, &mut app, &config) {
                    ThemeAction::Consumed => {}
                    ThemeAction::NotHandled => {}
                    ThemeAction::Quit => {
                        app.should_quit = true;
                    }
                    ThemeAction::OpenPopup { title, content } => {
                        app.open_popup(title, content);
                    }
                    ThemeAction::FetchLogs { id, name, tail } => {
                        if let Ok(logs) = container_svc.get_logs(&id, Some(tail)).await {
                            let lines: Vec<String> = logs.lines().map(String::from).collect();
                            app.open_log_view(id, name, lines);
                        }
                    }
                    ThemeAction::FetchDetail { id } => {
                        let name = app
                            .containers
                            .iter()
                            .find(|c| c.id == id)
                            .map(|c| crate::display::container_display_name(&c.names))
                            .unwrap_or_default();
                        let lines = if let Ok(detail) = container_svc.get_detail(&id).await {
                            crate::display::preview::container_detail_lines(&detail)
                        } else {
                            vec!["Failed to fetch detail".into()]
                        };
                        app.open_detail_view(format!("Container: {}", name), lines);
                    }
                    ThemeAction::RefreshLogs { id, tail } => {
                        if let Ok(logs) = container_svc.get_logs(&id, Some(tail)).await {
                            app.log_view_lines = logs.lines().map(String::from).collect();
                            app.log_view_scroll = if app.log_view_follow {
                                app.log_view_lines.len().saturating_sub(1) as u16
                            } else {
                                0
                            };
                        }
                    }
                    ThemeAction::StopContainer { id } => {
                        if let Err(e) = container_svc.stop(&id).await {
                            tracing::warn!("Failed to stop container {}: {}", id, e);
                        }
                    }
                    ThemeAction::RestartContainer { id } => {
                        if let Err(e) = container_svc.restart(&id).await {
                            tracing::warn!("Failed to restart container {}: {}", id, e);
                        }
                    }
                    ThemeAction::RemoveContainer { id } => {
                        if let Err(e) = container_svc.remove(&id, true).await {
                            tracing::warn!("Failed to remove container {}: {}", id, e);
                        }
                    }
                    ThemeAction::ExecContainer { id } => {
                        if tmux::is_tmux_available() {
                            let bin = match runtime.runtime_type() {
                                RuntimeType::Docker => "docker",
                                RuntimeType::Podman => "podman",
                            };
                            let tmux_int = tmux::TmuxIntegration::new(bin);
                            if let Err(e) = tmux_int.split_pane_exec(&id, "/bin/sh").await {
                                tracing::warn!("Failed to exec: {}", e);
                            }
                        }
                    }
                    ThemeAction::ExplainItem { use_inline } => {
                        open_explain(&mut app, &container_svc, use_inline).await;
                    }
                }
            }
            AppEvent::Resize(_, _) => {
                // Terminal handles resize automatically
            }
        }

        if app.should_quit {
            break;
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}

/// Open the detail view for the currently selected item.
/// Uses inline detail view for operator theme, popup for dashboard theme.
async fn open_explain(app: &mut App, container_svc: &ContainerService, use_inline: bool) {
    let (title, lines) = match app.active_panel {
        PanelId::Alerts => {
            if let Some(alert) = app.alerts.get(app.selected_index()) {
                (
                    "Alert Detail".to_string(),
                    preview::alert_detail_lines(alert),
                )
            } else {
                return;
            }
        }
        PanelId::Resources => (
            "Resource Detail".to_string(),
            preview::resource_detail_lines(app, app.selected_index()),
        ),
        PanelId::Status => (
            "Status Detail".to_string(),
            preview::status_detail_lines(app, app.selected_index()),
        ),
        PanelId::Containers => {
            if let Some(id) = app.selected_container_id() {
                let detail_lines = if let Ok(detail) = container_svc.get_detail(&id).await {
                    preview::container_detail_lines(&detail)
                } else if let Some(c) = app.containers.get(app.selected_index()) {
                    preview::container_summary_lines(c)
                } else {
                    vec!["No container selected".to_string()]
                };
                let name = app
                    .containers
                    .get(app.selected_index())
                    .map(|c| crate::display::container_display_name(&c.names))
                    .unwrap_or_default();
                (format!("Container: {}", name), detail_lines)
            } else {
                return;
            }
        }
        PanelId::Images => {
            if let Some(img) = app.images.get(app.selected_index()) {
                let tag = img
                    .repo_tags
                    .first()
                    .cloned()
                    .unwrap_or_else(|| "<none>".into());
                (format!("Image: {}", tag), preview::image_detail_lines(img))
            } else {
                return;
            }
        }
        PanelId::Volumes => {
            if let Some(vol) = app.volumes.get(app.selected_index()) {
                (
                    format!("Volume: {}", vol.name),
                    preview::volume_detail_lines(vol),
                )
            } else {
                return;
            }
        }
        PanelId::Networks => {
            if let Some(net) = app.networks.get(app.selected_index()) {
                (
                    format!("Network: {}", net.name),
                    preview::network_detail_lines(net),
                )
            } else {
                return;
            }
        }
    };

    if use_inline {
        app.open_detail_view(title, lines);
    } else {
        app.open_popup(title, lines);
    }
}

/// Load all data for all panels (used at startup).
async fn load_data(
    app: &mut App,
    container_svc: &ContainerService,
    image_svc: &ImageService,
    volume_svc: &VolumeService,
    network_svc: &NetworkService,
    pod_svc: &PodService,
    system_svc: &SystemService,
) {
    if let Ok(containers) = container_svc.list_all().await {
        app.containers = containers;
    }
    if let Ok(images) = image_svc.list().await {
        app.images = images;
    }
    if let Ok(volumes) = volume_svc.list().await {
        app.volumes = volumes;
    }
    if let Ok(networks) = network_svc.list().await {
        app.networks = networks;
    }
    if app.supports_pods
        && let Ok(pods) = pod_svc.list().await
    {
        app.pods = pods;
    }
    if let Ok(info) = system_svc.info().await {
        app.system_info = Some(info);
    }
}

/// Refresh host-level CPU, memory, and swap stats from the OS.
fn refresh_host_stats(app: &mut App, sys: &mut System) {
    sys.refresh_cpu_all();
    sys.refresh_memory();

    app.host_cpu_percent = sys.global_cpu_usage() as f64;
    app.host_memory_used = sys.used_memory();
    app.host_memory_total = sys.total_memory();
    app.host_swap_used = sys.used_swap();
    app.host_swap_total = sys.total_swap();
}

/// Compute alerts based on current app state.
fn compute_alerts(app: &mut App) {
    app.alerts.clear();

    // Dangling images (<none>:<none>)
    app.dangling_images_count = app
        .images
        .iter()
        .filter(|i| i.repo_tags.is_empty() || i.repo_tags.iter().all(|t| t == "<none>:<none>"))
        .count();
    if app.dangling_images_count > 0 {
        app.alerts.push(Alert {
            level: AlertLevel::Warning,
            message: format!("{} dangling imgs", app.dangling_images_count),
        });
    }

    // Stale images — images not used by any container
    let used_images: std::collections::HashSet<&str> =
        app.containers.iter().map(|c| c.image.as_str()).collect();
    app.stale_images_count = app
        .images
        .iter()
        .filter(|i| !i.repo_tags.iter().any(|t| used_images.contains(t.as_str())))
        .count();
    if app.stale_images_count > 5 {
        app.alerts.push(Alert {
            level: AlertLevel::Warning,
            message: format!("{} unused images", app.stale_images_count),
        });
    }

    // Unused volumes — volumes not mounted by any container
    // (We can't easily check this without inspecting each container,
    // so we use a heuristic: if volume count > container count * 2)
    if app.volumes.len() > app.containers.len() * 2 && app.volumes.len() > 10 {
        app.unused_volumes_count = app.volumes.len() - app.containers.len();
        app.alerts.push(Alert {
            level: AlertLevel::Warning,
            message: format!("~{} unused volumes", app.unused_volumes_count),
        });
    }

    // System resource thresholds
    if let Some(info) = &app.system_info {
        let container_count = info.containers_running + info.containers_stopped;
        if info.containers_stopped > container_count / 2 && info.containers_stopped > 5 {
            app.alerts.push(Alert {
                level: AlertLevel::Warning,
                message: format!("{} stopped containers", info.containers_stopped),
            });
        }
    }
}

/// Load data only for the currently active panel (more efficient on tick).
async fn load_panel_data(
    app: &mut App,
    container_svc: &ContainerService,
    image_svc: &ImageService,
    volume_svc: &VolumeService,
    network_svc: &NetworkService,
    _pod_svc: &PodService,
    system_svc: &SystemService,
) {
    match app.active_panel {
        PanelId::Alerts | PanelId::Resources | PanelId::Status => {
            // Refresh system info and counts
            if let Ok(containers) = container_svc.list_all().await {
                app.containers = containers;
            }
            if let Ok(info) = system_svc.info().await {
                app.system_info = Some(info);
            }
        }
        PanelId::Containers => {
            if let Ok(containers) = container_svc.list_all().await {
                app.containers = containers;
            }
            // Fetch stats for running containers (limit to avoid overloading)
            let running: Vec<String> = app
                .containers
                .iter()
                .filter(|c| c.state == "running")
                .take(20) // cap to avoid too many API calls per tick
                .map(|c| c.id.clone())
                .collect();
            for id in running {
                if let Ok(stats) = container_svc.get_stats(&id).await {
                    app.container_stats
                        .insert(id, (stats.cpu_percent, stats.memory_usage));
                }
            }
        }
        PanelId::Images => {
            if let Ok(images) = image_svc.list().await {
                app.images = images;
            }
        }
        PanelId::Volumes => {
            if let Ok(volumes) = volume_svc.list().await {
                app.volumes = volumes;
            }
        }
        PanelId::Networks => {
            if let Ok(networks) = network_svc.list().await {
                app.networks = networks;
            }
        }
    }

    // Refresh logs if log view is active and follow mode is on
    if app.log_view_active
        && app.log_view_follow
        && let Ok(logs) = container_svc
            .get_logs(&app.log_view_container_id, Some(500))
            .await
    {
        app.log_view_lines = logs.lines().map(String::from).collect();
        // Auto-scroll to bottom in follow mode
        app.log_view_scroll = app.log_view_lines.len().saturating_sub(1) as u16;
    }
}
