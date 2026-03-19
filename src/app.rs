use crate::runtime::{
    ContainerDetail, ContainerStats, ContainerSummary, ImageSummary, NetworkSummary, PodSummary,
    SystemInfo, VolumeSummary,
};

// ---------------------------------------------------------------------------
// Panel — each screen the user can navigate to
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Panel {
    Status,
    Containers,
    Images,
    Volumes,
    Networks,
    Pods,
}

impl Panel {
    /// All panels in display order (includes Pods).
    pub const ALL: &'static [Panel] = &[
        Panel::Status,
        Panel::Containers,
        Panel::Images,
        Panel::Volumes,
        Panel::Networks,
        Panel::Pods,
    ];

    /// Human-readable label for the panel.
    pub fn label(self) -> &'static str {
        match self {
            Panel::Status => "Status",
            Panel::Containers => "Containers",
            Panel::Images => "Images",
            Panel::Volumes => "Volumes",
            Panel::Networks => "Networks",
            Panel::Pods => "Pods",
        }
    }

    /// Keyboard shortcut number for this panel ("1" through "6").
    pub fn shortcut(self) -> &'static str {
        match self {
            Panel::Status => "1",
            Panel::Containers => "2",
            Panel::Images => "3",
            Panel::Volumes => "4",
            Panel::Networks => "5",
            Panel::Pods => "6",
        }
    }

    /// Ordered list of panels available for the current runtime.
    /// `Pods` is only included when the runtime supports it (Podman).
    pub fn ordered(include_pods: bool) -> Vec<Panel> {
        let mut panels = vec![
            Panel::Status,
            Panel::Containers,
            Panel::Images,
            Panel::Volumes,
            Panel::Networks,
        ];
        if include_pods {
            panels.push(Panel::Pods);
        }
        panels
    }
}

// ---------------------------------------------------------------------------
// FocusZone — which zone of the UI currently has keyboard focus
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusZone {
    LeftPanel,
    RightPreview,
}

// ---------------------------------------------------------------------------
// Alert — system alerts displayed in the sidebar
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct Alert {
    pub level: AlertLevel,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlertLevel {
    Warning,
    Critical,
}

// ---------------------------------------------------------------------------
// InputMode — what keystrokes mean in the current context
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    Search,
}

// ---------------------------------------------------------------------------
// App — central state machine
// ---------------------------------------------------------------------------

/// Central application state shared between event handling and rendering.
pub struct App {
    // Navigation
    pub active_panel: Panel,
    pub previous_panel: Option<Panel>,
    pub focus: FocusZone,
    pub input_mode: InputMode,
    pub search_query: String,

    // Domain data
    pub containers: Vec<ContainerSummary>,
    pub images: Vec<ImageSummary>,
    pub volumes: Vec<VolumeSummary>,
    pub networks: Vec<NetworkSummary>,
    pub pods: Vec<PodSummary>,
    pub system_info: Option<SystemInfo>,

    // Selection & detail
    pub selected_index: usize,
    pub show_detail: bool,
    pub detail_container: Option<ContainerDetail>,
    pub log_lines: Vec<String>,
    pub stats: Option<ContainerStats>,
    pub preview_scroll: u16,

    // Alerts & counts
    pub alerts: Vec<Alert>,
    pub stale_images_count: usize,
    pub unused_volumes_count: usize,
    pub dangling_images_count: usize,

    // Runtime capabilities
    pub supports_pods: bool,

    // Lifecycle
    pub should_quit: bool,
}

impl App {
    /// Create a new `App`. Pass `true` when the runtime is Podman (pod support).
    pub fn new(supports_pods: bool) -> Self {
        Self {
            active_panel: Panel::Status,
            previous_panel: None,
            focus: FocusZone::LeftPanel,
            input_mode: InputMode::Normal,
            search_query: String::new(),

            containers: Vec::new(),
            images: Vec::new(),
            volumes: Vec::new(),
            networks: Vec::new(),
            pods: Vec::new(),
            system_info: None,

            selected_index: 0,
            show_detail: false,
            detail_container: None,
            log_lines: Vec::new(),
            stats: None,
            preview_scroll: 0,

            alerts: Vec::new(),
            stale_images_count: 0,
            unused_volumes_count: 0,
            dangling_images_count: 0,

            supports_pods,
            should_quit: false,
        }
    }

    // ------------------------------------------------------------------
    // Panel switching
    // ------------------------------------------------------------------

    /// Switch to a specific panel, saving the current one as `previous_panel`.
    pub fn switch_to_panel(&mut self, panel: Panel) {
        self.previous_panel = Some(self.active_panel);
        self.active_panel = panel;
        self.reset_selection();
        self.clear_preview_data();
    }

    /// Move to the next panel in order, wrapping around.
    pub fn next_panel(&mut self) {
        let panels = Panel::ordered(self.supports_pods);
        let current_pos = panels
            .iter()
            .position(|p| *p == self.active_panel)
            .unwrap_or(0);
        let next_pos = (current_pos + 1) % panels.len();
        self.switch_to_panel(panels[next_pos]);
    }

    /// Move to the previous panel in order, wrapping around.
    pub fn previous_panel(&mut self) {
        let panels = Panel::ordered(self.supports_pods);
        let current_pos = panels
            .iter()
            .position(|p| *p == self.active_panel)
            .unwrap_or(0);
        let prev_pos = if current_pos == 0 {
            panels.len() - 1
        } else {
            current_pos - 1
        };
        self.switch_to_panel(panels[prev_pos]);
    }

    /// Clear all preview/detail state.
    pub fn clear_preview_data(&mut self) {
        self.detail_container = None;
        self.log_lines.clear();
        self.stats = None;
        self.preview_scroll = 0;
    }

    // ------------------------------------------------------------------
    // List selection
    // ------------------------------------------------------------------

    /// Move selection down one item in the current panel's list, wrapping.
    pub fn next_item(&mut self) {
        let len = self.current_list_len();
        if len > 0 {
            self.selected_index = (self.selected_index + 1) % len;
        }
    }

    /// Move selection up one item in the current panel's list, wrapping.
    pub fn previous_item(&mut self) {
        let len = self.current_list_len();
        if len > 0 {
            self.selected_index = if self.selected_index == 0 {
                len - 1
            } else {
                self.selected_index - 1
            };
        }
    }

    /// Reset selected index to the top of the list.
    pub fn reset_selection(&mut self) {
        self.selected_index = 0;
    }

    /// Length of the list currently visible in the active panel.
    pub fn current_list_len(&self) -> usize {
        match self.active_panel {
            Panel::Status => 0,
            Panel::Containers => self.containers.len(),
            Panel::Images => self.images.len(),
            Panel::Volumes => self.volumes.len(),
            Panel::Networks => self.networks.len(),
            Panel::Pods => self.pods.len(),
        }
    }

    // ------------------------------------------------------------------
    // Selected-item getters
    // ------------------------------------------------------------------

    /// Return the container ID at the current selection, if any.
    pub fn selected_container_id(&self) -> Option<String> {
        self.containers
            .get(self.selected_index)
            .map(|c| c.id.clone())
    }

    /// Return the image ID at the current selection, if any.
    pub fn selected_image_id(&self) -> Option<String> {
        self.images.get(self.selected_index).map(|i| i.id.clone())
    }

    /// Return the volume name at the current selection, if any.
    pub fn selected_volume_name(&self) -> Option<String> {
        self.volumes
            .get(self.selected_index)
            .map(|v| v.name.clone())
    }

    /// Return the network ID at the current selection, if any.
    pub fn selected_network_id(&self) -> Option<String> {
        self.networks
            .get(self.selected_index)
            .map(|n| n.id.clone())
    }

    /// Return the pod ID at the current selection, if any.
    pub fn selected_pod_id(&self) -> Option<String> {
        self.pods.get(self.selected_index).map(|p| p.id.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn new_app_starts_on_status_panel() {
        let app = App::new(false);
        assert_eq!(app.active_panel, Panel::Status);
        assert_eq!(app.focus, FocusZone::LeftPanel);
        assert_eq!(app.input_mode, InputMode::Normal);
        assert!(!app.should_quit);
    }

    #[test]
    fn panel_cycling_skips_pods_when_unsupported() {
        let mut app = App::new(false);
        for _ in 0..10 {
            app.next_panel();
            assert_ne!(app.active_panel, Panel::Pods);
        }
    }

    #[test]
    fn panel_cycling_includes_pods_when_supported() {
        let mut app = App::new(true);
        let mut seen_pods = false;
        for _ in 0..10 {
            app.next_panel();
            if app.active_panel == Panel::Pods {
                seen_pods = true;
            }
        }
        assert!(seen_pods);
    }

    #[test]
    fn previous_panel_wraps_around() {
        let mut app = App::new(false);
        assert_eq!(app.active_panel, Panel::Status);
        app.previous_panel(); // should wrap to last panel (Networks, since no pods)
        assert_eq!(app.active_panel, Panel::Networks);
    }

    #[test]
    fn switch_panel_resets_selection_and_preview() {
        let mut app = App::new(false);
        app.selected_index = 5;
        app.detail_container = Some(ContainerDetail {
            id: "test".into(),
            name: "test".into(),
            image: "img".into(),
            created: "now".into(),
            state: crate::runtime::ContainerState {
                status: "running".into(),
                running: true,
                paused: false,
                restarting: false,
                oom_killed: false,
                dead: false,
                pid: 1,
                exit_code: 0,
                started_at: "".into(),
                finished_at: "".into(),
            },
            path: "".into(),
            args: vec![],
            restart_count: 0,
            driver: "".into(),
            platform: "".into(),
            log_path: "".into(),
            labels: HashMap::new(),
            env: vec![],
        });
        app.log_lines = vec!["line1".into()];
        app.stats = Some(ContainerStats {
            container_id: "test".into(),
            name: "test".into(),
            cpu_percent: 0.0,
            memory_usage: 0,
            memory_limit: 0,
            memory_percent: 0.0,
            network_rx: 0,
            network_tx: 0,
            block_read: 0,
            block_write: 0,
            pids: 0,
        });
        app.preview_scroll = 10;

        app.switch_to_panel(Panel::Images);

        assert_eq!(app.selected_index, 0);
        assert!(app.detail_container.is_none());
        assert!(app.log_lines.is_empty());
        assert!(app.stats.is_none());
        assert_eq!(app.preview_scroll, 0);
    }

    #[test]
    fn focus_toggles_between_zones() {
        let mut app = App::new(false);
        assert_eq!(app.focus, FocusZone::LeftPanel);
        app.focus = FocusZone::RightPreview;
        assert_eq!(app.focus, FocusZone::RightPreview);
        app.focus = FocusZone::LeftPanel;
        assert_eq!(app.focus, FocusZone::LeftPanel);
    }

    #[test]
    fn item_navigation_wraps() {
        let mut app = App::new(false);
        app.active_panel = Panel::Containers;
        // empty list -- no crash
        app.next_item();
        app.previous_item();
        assert_eq!(app.selected_index, 0);
    }

    #[test]
    fn item_navigation_with_data() {
        let mut app = App::new(false);
        app.active_panel = Panel::Containers;
        app.containers = vec![
            ContainerSummary {
                id: "a".into(),
                name: "a".into(),
                names: vec!["a".into()],
                image: "img".into(),
                image_id: "sha256:aaa".into(),
                command: "/bin/sh".into(),
                state: "running".into(),
                status: "Up 2 hours".into(),
                created: 1700000000,
                ports: vec![],
                labels: HashMap::from([("env".to_string(), "dev".to_string())]),
            },
            ContainerSummary {
                id: "b".into(),
                name: "b".into(),
                names: vec!["b".into()],
                image: "img".into(),
                image_id: "sha256:bbb".into(),
                command: "/bin/bash".into(),
                state: "exited".into(),
                status: "Exited (0)".into(),
                created: 1700000100,
                ports: vec![],
                labels: HashMap::from([("env".to_string(), "prod".to_string())]),
            },
        ];
        assert_eq!(app.selected_index, 0);
        app.next_item();
        assert_eq!(app.selected_index, 1);
        app.next_item(); // wraps
        assert_eq!(app.selected_index, 0);
        app.previous_item(); // wraps back
        assert_eq!(app.selected_index, 1);
    }

    #[test]
    fn selected_getters_return_correct_ids() {
        let mut app = App::new(false);
        app.containers = vec![ContainerSummary {
            id: "abc123".into(),
            name: "test".into(),
            names: vec!["test".into()],
            image: "nginx:latest".into(),
            image_id: "sha256:aaa".into(),
            command: "nginx -g daemon off;".into(),
            state: "running".into(),
            status: "Up 5 minutes".into(),
            created: 1700000000,
            ports: vec![],
            labels: HashMap::from([("maintainer".to_string(), "nginx".to_string())]),
        }];
        app.images = vec![ImageSummary {
            id: "sha256:def".into(),
            repo_tags: vec!["nginx:latest".into()],
            repo_digests: vec!["nginx@sha256:abc123".into()],
            size: 187000000,
            created: 1699999000,
            labels: HashMap::from([("version".to_string(), "1.25".to_string())]),
        }];

        app.active_panel = Panel::Containers;
        assert_eq!(app.selected_container_id(), Some("abc123".into()));

        app.active_panel = Panel::Images;
        assert_eq!(app.selected_image_id(), Some("sha256:def".into()));
    }

    #[test]
    fn all_panels_have_unique_shortcuts() {
        let shortcuts: Vec<&str> = Panel::ALL.iter().map(|p| p.shortcut()).collect();
        let unique: std::collections::HashSet<&&str> = shortcuts.iter().collect();
        assert_eq!(shortcuts.len(), unique.len());
    }

    #[test]
    fn new_app_has_empty_alerts() {
        let app = App::new(false);
        assert!(app.alerts.is_empty());
        assert_eq!(app.stale_images_count, 0);
        assert_eq!(app.unused_volumes_count, 0);
        assert_eq!(app.dangling_images_count, 0);
    }

    #[test]
    fn clear_preview_data_resets_everything() {
        let mut app = App::new(false);
        app.detail_container = Some(ContainerDetail {
            id: "test".into(),
            name: "test".into(),
            image: "img".into(),
            created: "now".into(),
            state: crate::runtime::ContainerState {
                status: "running".into(),
                running: true,
                paused: false,
                restarting: false,
                oom_killed: false,
                dead: false,
                pid: 1,
                exit_code: 0,
                started_at: "".into(),
                finished_at: "".into(),
            },
            path: "".into(),
            args: vec![],
            restart_count: 0,
            driver: "".into(),
            platform: "".into(),
            log_path: "".into(),
            labels: HashMap::new(),
            env: vec![],
        });
        app.log_lines = vec!["log line".into()];
        app.stats = Some(ContainerStats {
            container_id: "test".into(),
            name: "test".into(),
            cpu_percent: 50.0,
            memory_usage: 1024,
            memory_limit: 4096,
            memory_percent: 25.0,
            network_rx: 100,
            network_tx: 200,
            block_read: 50,
            block_write: 75,
            pids: 5,
        });
        app.preview_scroll = 42;

        app.clear_preview_data();

        assert!(app.detail_container.is_none());
        assert!(app.log_lines.is_empty());
        assert!(app.stats.is_none());
        assert_eq!(app.preview_scroll, 0);
    }
}
