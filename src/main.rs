pub mod app;
pub mod config;
pub mod event;
pub mod runtime;
pub mod services;
pub mod tmux;
pub mod ui;

use std::io;
use std::sync::Arc;

use anyhow::{Context, Result};
use clap::Parser;
use crossterm::{
    event::{KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;
use tracing_subscriber::EnvFilter;

use crate::app::{Alert, AlertLevel, App, FocusZone, InputMode, Panel};
use crate::event::{AppEvent, EventHandler};
use crate::runtime::{detect_runtime, RuntimeType};
use crate::services::{
    ContainerService, ImageService, NetworkService, PodService, SystemService, VolumeService,
};

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

    // Main loop
    loop {
        // Draw
        terminal.draw(|frame| ui::draw(frame, &app, &config.layout))?;

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
                compute_alerts(&mut app);
            }
            AppEvent::Key(key) => {
                if app.input_mode == InputMode::Search {
                    handle_search_input(&mut app, key.code);
                    continue;
                }

                match (key.code, key.modifiers) {
                    // Quit
                    (KeyCode::Char('q'), _) | (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                        app.should_quit = true;
                    }

                    // Panel switching (1-6) — lazygit style
                    (KeyCode::Char('1'), _) => app.switch_to_panel(Panel::Status),
                    (KeyCode::Char('2'), _) => app.switch_to_panel(Panel::Containers),
                    (KeyCode::Char('3'), _) => app.switch_to_panel(Panel::Images),
                    (KeyCode::Char('4'), _) => app.switch_to_panel(Panel::Volumes),
                    (KeyCode::Char('5'), _) => app.switch_to_panel(Panel::Networks),
                    (KeyCode::Char('6'), _) if supports_pods => {
                        app.switch_to_panel(Panel::Pods);
                    }

                    // Tab cycles panels
                    (KeyCode::Tab, _) => app.next_panel(),
                    (KeyCode::BackTab, _) => app.previous_panel(),

                    // Focus switching between columns
                    (KeyCode::Left, _) => {
                        app.focus = FocusZone::LeftPanel;
                        app.preview_scroll = 0;
                    }
                    (KeyCode::Right, _) | (KeyCode::Char('0'), _) => {
                        app.focus = FocusZone::RightPreview;
                    }
                    (KeyCode::Esc, _) => {
                        app.focus = FocusZone::LeftPanel;
                        app.preview_scroll = 0;
                        app.show_detail = false;
                        app.clear_preview_data();
                    }

                    // Navigation within focused column
                    (KeyCode::Down | KeyCode::Char('j'), _) => match app.focus {
                        FocusZone::LeftPanel => app.next_item(),
                        FocusZone::RightPreview => {
                            app.preview_scroll = app.preview_scroll.saturating_add(1);
                        }
                    },
                    (KeyCode::Up | KeyCode::Char('k'), _) => match app.focus {
                        FocusZone::LeftPanel => app.previous_item(),
                        FocusZone::RightPreview => {
                            app.preview_scroll = app.preview_scroll.saturating_sub(1);
                        }
                    },

                    // Enter — load full preview data and focus right
                    (KeyCode::Enter, _) => match app.active_panel {
                        Panel::Containers => {
                            if let Some(id) = app.selected_container_id() {
                                if let Ok(detail) = container_svc.get_detail(&id).await {
                                    app.detail_container = Some(detail);
                                }
                                if let Ok(logs) =
                                    container_svc.get_logs(&id, Some(100)).await
                                {
                                    app.log_lines =
                                        logs.lines().map(String::from).collect();
                                }
                                if let Ok(stats) = container_svc.get_stats(&id).await {
                                    app.stats = Some(stats);
                                }
                                app.show_detail = true;
                                app.focus = FocusZone::RightPreview;
                                app.preview_scroll = 0;
                            }
                        }
                        _ => {
                            app.focus = FocusZone::RightPreview;
                            app.preview_scroll = 0;
                        }
                    },

                    // Container actions (only on Containers panel)
                    (KeyCode::Char('s'), _) if app.active_panel == Panel::Containers => {
                        if let Some(id) = app.selected_container_id() {
                            let _ = container_svc.stop(&id).await;
                        }
                    }
                    (KeyCode::Char('r'), _) if app.active_panel == Panel::Containers => {
                        if let Some(id) = app.selected_container_id() {
                            let _ = container_svc.restart(&id).await;
                        }
                    }
                    (KeyCode::Char('x'), _) if app.active_panel == Panel::Containers => {
                        if let Some(id) = app.selected_container_id() {
                            let _ = container_svc.remove(&id, true).await;
                        }
                    }
                    (KeyCode::Char('l'), _) if app.active_panel == Panel::Containers => {
                        if let Some(id) = app.selected_container_id() {
                            if let Ok(logs) =
                                container_svc.get_logs(&id, Some(200)).await
                            {
                                app.log_lines =
                                    logs.lines().map(String::from).collect();
                            }
                        }
                    }
                    (KeyCode::Char('e'), _) if app.active_panel == Panel::Containers => {
                        if let Some(id) = app.selected_container_id() {
                            if tmux::is_tmux_available() {
                                let bin = match runtime.runtime_type() {
                                    RuntimeType::Docker => "docker",
                                    RuntimeType::Podman => "podman",
                                };
                                let tmux_int = tmux::TmuxIntegration::new(bin);
                                let _ = tmux_int.split_pane_exec(&id, "/bin/sh").await;
                            }
                        }
                    }

                    // Search
                    (KeyCode::Char('/'), _) => {
                        app.input_mode = InputMode::Search;
                        app.search_query.clear();
                    }

                    _ => {}
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
    pod_svc: &PodService,
    system_svc: &SystemService,
) {
    match app.active_panel {
        Panel::Status => {
            if let Ok(containers) = container_svc.list_all().await {
                app.containers = containers;
            }
            if let Ok(info) = system_svc.info().await {
                app.system_info = Some(info);
            }
        }
        Panel::Containers => {
            if let Ok(containers) = container_svc.list_all().await {
                app.containers = containers;
            }
            // Refresh stats and logs if detail is open
            if app.show_detail {
                if let Some(id) = app.selected_container_id() {
                    if let Ok(stats) = container_svc.get_stats(&id).await {
                        app.stats = Some(stats);
                    }
                    if let Ok(logs) = container_svc.get_logs(&id, Some(100)).await {
                        app.log_lines = logs.lines().map(String::from).collect();
                    }
                }
            }
        }
        Panel::Images => {
            if let Ok(images) = image_svc.list().await {
                app.images = images;
            }
        }
        Panel::Volumes => {
            if let Ok(volumes) = volume_svc.list().await {
                app.volumes = volumes;
            }
        }
        Panel::Networks => {
            if let Ok(networks) = network_svc.list().await {
                app.networks = networks;
            }
        }
        Panel::Pods => {
            if app.supports_pods {
                if let Ok(pods) = pod_svc.list().await {
                    app.pods = pods;
                }
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
