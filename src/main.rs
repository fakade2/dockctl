pub mod app;
pub mod config;
pub mod event;
pub mod runtime;
pub mod services;
pub mod themes;
pub mod tmux;
pub mod ui;

use std::io;
use std::sync::Arc;

use anyhow::{Context, Result};
use clap::Parser;
use crossterm::{
    event::{KeyCode, KeyEvent},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;
use tracing_subscriber::EnvFilter;

use crate::app::{Alert, AlertLevel, App, InputMode, PanelId};
use crate::event::{AppEvent, EventHandler};
use crate::runtime::{detect_runtime, ContainerRuntime, RuntimeType};
use crate::services::{
    ContainerService, ImageService, NetworkService, PodService, SystemService, VolumeService,
};
use crate::themes::ThemeAction;
use sysinfo::System;

use crate::ui::preview;

#[derive(Parser, Debug)]
#[command(name = "dockctl", version, about = "Universal container runtime TUI — Docker & Podman")]
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
                // ── Popup mode — same for all themes ────────────────────
                if app.show_popup {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => {
                            app.close_popup();
                        }
                        KeyCode::Char('j') | KeyCode::Down => {
                            app.popup_scroll = app.popup_scroll.saturating_add(1);
                        }
                        KeyCode::Char('k') | KeyCode::Up => {
                            app.popup_scroll = app.popup_scroll.saturating_sub(1);
                        }
                        _ => {} // ignore everything else in popup
                    }
                    continue;
                }

                // ── Search mode — same for all themes ───────────────────
                if app.input_mode == InputMode::Search {
                    handle_search_input(&mut app, key.code);
                    continue;
                }

                // ── Theme-specific key handling ─────────────────────────
                match theme.handle_key(key, &mut app, &config) {
                    ThemeAction::Consumed => {}
                    ThemeAction::Quit => {
                        app.should_quit = true;
                    }
                    ThemeAction::OpenPopup { title, content } => {
                        app.open_popup(title, content);
                    }
                    ThemeAction::NotHandled => {
                        // Global actions (async operations)
                        handle_global_key(key, &mut app, &container_svc, &runtime).await;
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

/// Handle global keys that are not theme-specific (async operations).
/// Called when the theme returns `ThemeAction::NotHandled`.
async fn handle_global_key(
    key: KeyEvent,
    app: &mut App,
    container_svc: &ContainerService,
    runtime: &Arc<Box<dyn ContainerRuntime>>,
) {
    let is_container_context =
        app.active_panel == PanelId::Containers || app.operator_resource == "containers";

    match key.code {
        // Container actions (active when viewing containers in any theme)
        KeyCode::Char('s') if is_container_context => {
            if let Some(id) = app.selected_container_id() {
                if let Err(e) = container_svc.stop(&id).await {
                    tracing::warn!("Failed to stop container {}: {}", id, e);
                }
            }
        }
        KeyCode::Char('r') if is_container_context => {
            if let Some(id) = app.selected_container_id() {
                if let Err(e) = container_svc.restart(&id).await {
                    tracing::warn!("Failed to restart container {}: {}", id, e);
                }
            }
        }
        KeyCode::Char('x') if is_container_context => {
            if let Some(id) = app.selected_container_id() {
                if let Err(e) = container_svc.remove(&id, true).await {
                    tracing::warn!("Failed to remove container {}: {}", id, e);
                }
            }
        }
        KeyCode::Char('l') if is_container_context => {
            // Reserved for logs
        }
        KeyCode::Char('E') if is_container_context => {
            if let Some(id) = app.selected_container_id() {
                if tmux::is_tmux_available() {
                    let bin = match runtime.runtime_type() {
                        RuntimeType::Docker => "docker",
                        RuntimeType::Podman => "podman",
                    };
                    let tmux_int = tmux::TmuxIntegration::new(bin);
                    if let Err(e) = tmux_int.split_pane_exec(&id, "/bin/sh").await {
                        tracing::warn!("Failed to exec into container {}: {}", id, e);
                    }
                }
            }
        }

        // Search
        KeyCode::Char('/') => {
            app.input_mode = InputMode::Search;
            app.search_query.clear();
        }

        // Explain popup
        KeyCode::Char('e') | KeyCode::Enter => {
            open_explain_popup(app, container_svc).await;
        }

        _ => {}
    }
}

/// Open the explain/detail popup for the currently selected item in the active panel.
async fn open_explain_popup(app: &mut App, container_svc: &ContainerService) {
    match app.active_panel {
        PanelId::Alerts => {
            if let Some(alert) = app.alerts.get(app.selected_index()) {
                app.open_popup(
                    "Alert Detail".to_string(),
                    preview::alert_detail_lines(alert),
                );
            }
        }
        PanelId::Resources => {
            app.open_popup(
                "Resource Detail".to_string(),
                preview::resource_detail_lines(&app, app.selected_index()),
            );
        }
        PanelId::Status => {
            app.open_popup(
                "Status Detail".to_string(),
                preview::status_detail_lines(&app, app.selected_index()),
            );
        }
        PanelId::Containers => {
            if let Some(id) = app.selected_container_id() {
                let lines = if let Ok(detail) = container_svc.get_detail(&id).await {
                    preview::container_detail_lines(&detail)
                } else if let Some(c) = app.containers.get(app.selected_index()) {
                    preview::container_summary_lines(c)
                } else {
                    vec!["No container selected".to_string()]
                };
                app.open_popup("Container Detail".to_string(), lines);
            }
        }
        PanelId::Images => {
            if let Some(img) = app.images.get(app.selected_index()) {
                app.open_popup(
                    "Image Detail".to_string(),
                    preview::image_detail_lines(img),
                );
            }
        }
        PanelId::Volumes => {
            if let Some(vol) = app.volumes.get(app.selected_index()) {
                app.open_popup(
                    "Volume Detail".to_string(),
                    preview::volume_detail_lines(vol),
                );
            }
        }
        PanelId::Networks => {
            if let Some(net) = app.networks.get(app.selected_index()) {
                app.open_popup(
                    "Network Detail".to_string(),
                    preview::network_detail_lines(net),
                );
            }
        }
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
    if app.supports_pods {
        if let Ok(pods) = pod_svc.list().await {
            app.pods = pods;
        }
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
    let used_images: std::collections::HashSet<&str> = app
        .containers
        .iter()
        .map(|c| c.image.as_str())
        .collect();
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
}

fn handle_search_input(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Esc => {
            app.input_mode = InputMode::Normal;
            app.search_query.clear();
        }
        KeyCode::Enter => {
            app.input_mode = InputMode::Normal;
        }
        KeyCode::Backspace => {
            app.search_query.pop();
        }
        KeyCode::Char(c) => {
            app.search_query.push(c);
        }
        _ => {}
    }
}
