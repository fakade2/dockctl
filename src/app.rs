use crate::runtime::{
    ContainerDetail, ContainerStats, ContainerSummary, ImageSummary, NetworkSummary, PodSummary,
    SystemInfo, VolumeSummary,
};

// ---------------------------------------------------------------------------
// View — each screen the user can navigate to
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    Dashboard,
    Containers,
    Images,
    Volumes,
    Networks,
    Compose,
    Builds,
    Pods,
}

impl View {
    /// All views in display order (includes Pods).
    pub const ALL: &'static [View] = &[
        View::Dashboard,
        View::Containers,
        View::Images,
        View::Volumes,
        View::Networks,
        View::Compose,
        View::Builds,
        View::Pods,
    ];

    /// Human-readable label for the view.
    pub fn label(self) -> &'static str {
        match self {
            View::Dashboard => "Dashboard",
            View::Containers => "Containers",
            View::Images => "Images",
            View::Volumes => "Volumes",
            View::Networks => "Networks",
            View::Compose => "Compose",
            View::Builds => "Builds",
            View::Pods => "Pods",
        }
    }

    /// Keyboard shortcut number for this view ("1" through "8").
    pub fn shortcut(self) -> &'static str {
        match self {
            View::Dashboard => "1",
            View::Containers => "2",
            View::Images => "3",
            View::Volumes => "4",
            View::Networks => "5",
            View::Compose => "6",
            View::Builds => "7",
            View::Pods => "8",
        }
    }

    /// Ordered list of views available for the current runtime.
    /// `Pods` is only included when the runtime supports it (Podman).
    fn ordered(include_pods: bool) -> Vec<View> {
        let mut views = vec![
            View::Dashboard,
            View::Containers,
            View::Images,
            View::Volumes,
            View::Networks,
            View::Compose,
            View::Builds,
        ];
        if include_pods {
            views.push(View::Pods);
        }
        views
    }
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
    pub current_view: View,
    pub previous_view: Option<View>,
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
            current_view: View::Dashboard,
            previous_view: None,
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

            alerts: Vec::new(),
            stale_images_count: 0,
            unused_volumes_count: 0,
            dangling_images_count: 0,

            supports_pods,
            should_quit: false,
        }
    }

    // ------------------------------------------------------------------
    // View cycling
    // ------------------------------------------------------------------

    /// Move to the next view in order, wrapping around.
    pub fn next_view(&mut self) {
        let views = View::ordered(self.supports_pods);
        let current_pos = views
            .iter()
            .position(|v| *v == self.current_view)
            .unwrap_or(0);
        let next_pos = (current_pos + 1) % views.len();
        self.switch_to_view(views[next_pos]);
    }

    /// Move to the previous view in order, wrapping around.
    pub fn previous_view(&mut self) {
        let views = View::ordered(self.supports_pods);
        let current_pos = views
            .iter()
            .position(|v| *v == self.current_view)
            .unwrap_or(0);
        let prev_pos = if current_pos == 0 {
            views.len() - 1
        } else {
            current_pos - 1
        };
        self.switch_to_view(views[prev_pos]);
    }

    /// Switch to a specific view, saving the current one as `previous_view`.
    fn switch_to_view(&mut self, view: View) {
        self.previous_view = Some(self.current_view);
        self.current_view = view;
        self.reset_selection();
        self.show_detail = false;
    }

    // ------------------------------------------------------------------
    // List selection
    // ------------------------------------------------------------------

    /// Move selection down one item in the current view's list, wrapping.
    pub fn next_item(&mut self) {
        let len = self.current_list_len();
        if len > 0 {
            self.selected_index = (self.selected_index + 1) % len;
        }
    }

    /// Move selection up one item in the current view's list, wrapping.
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

    /// Length of the list currently visible in the active view.
    pub fn current_list_len(&self) -> usize {
        match self.current_view {
            View::Dashboard => self.containers.len(),
            View::Containers => self.containers.len(),
            View::Images => self.images.len(),
            View::Volumes => self.volumes.len(),
            View::Networks => self.networks.len(),
            View::Compose => 0, // compose projects not tracked as a simple vec yet
            View::Builds => 0,  // build queue not tracked yet
            View::Pods => self.pods.len(),
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

    #[test]
    fn new_app_starts_on_dashboard() {
        let app = App::new(false);
        assert_eq!(app.current_view, View::Dashboard);
        assert_eq!(app.input_mode, InputMode::Normal);
        assert!(!app.should_quit);
    }

    #[test]
    fn view_cycling_skips_pods_when_unsupported() {
        let mut app = App::new(false);
        for _ in 0..10 {
            app.next_view();
            assert_ne!(app.current_view, View::Pods);
        }
    }

    #[test]
    fn view_cycling_includes_pods_when_supported() {
        let mut app = App::new(true);
        let mut seen_pods = false;
        for _ in 0..10 {
            app.next_view();
            if app.current_view == View::Pods {
                seen_pods = true;
            }
        }
        assert!(seen_pods);
    }

    #[test]
    fn previous_view_wraps_around() {
        let mut app = App::new(false);
        assert_eq!(app.current_view, View::Dashboard);
        app.previous_view(); // should wrap to last view (Builds)
        assert_eq!(app.current_view, View::Builds);
    }

    #[test]
    fn item_navigation_wraps() {
        let mut app = App::new(false);
        app.current_view = View::Containers;
        // empty list — no crash
        app.next_item();
        app.previous_item();
        assert_eq!(app.selected_index, 0);
    }

    #[test]
    fn selected_container_id_returns_none_when_empty() {
        let app = App::new(false);
        assert!(app.selected_container_id().is_none());
    }

    #[test]
    fn view_shortcut_returns_correct_numbers() {
        assert_eq!(View::Dashboard.shortcut(), "1");
        assert_eq!(View::Containers.shortcut(), "2");
        assert_eq!(View::Pods.shortcut(), "8");
    }

    #[test]
    fn all_views_have_unique_shortcuts() {
        let shortcuts: Vec<&str> = View::ALL.iter().map(|v| v.shortcut()).collect();
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
    fn switch_view_resets_selection_and_detail() {
        let mut app = App::new(false);
        app.selected_index = 5;
        app.show_detail = true;
        app.next_view();
        assert_eq!(app.selected_index, 0);
        assert!(!app.show_detail);
    }

    #[test]
    fn item_navigation_with_data() {
        let mut app = App::new(false);
        app.current_view = View::Containers;
        app.containers = vec![
            ContainerSummary {
                id: "a".into(),
                name: "a".into(),
                names: vec!["a".into()],
                image: "img".into(),
                image_id: "sha256:aaa".into(),
                command: "".into(),
                state: "running".into(),
                status: "Up".into(),
                created: 0,
                ports: vec![],
                labels: Default::default(),
            },
            ContainerSummary {
                id: "b".into(),
                name: "b".into(),
                names: vec!["b".into()],
                image: "img".into(),
                image_id: "sha256:bbb".into(),
                command: "".into(),
                state: "running".into(),
                status: "Up".into(),
                created: 0,
                ports: vec![],
                labels: Default::default(),
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
            image: "img".into(),
            image_id: "sha256:aaa".into(),
            command: "".into(),
            state: "running".into(),
            status: "Up".into(),
            created: 0,
            ports: vec![],
            labels: Default::default(),
        }];
        app.images = vec![ImageSummary {
            id: "sha256:def".into(),
            repo_tags: vec!["nginx:latest".into()],
            repo_digests: vec![],
            size: 1000,
            created: 0,
            labels: Default::default(),
        }];

        app.current_view = View::Containers;
        assert_eq!(app.selected_container_id(), Some("abc123".into()));

        app.current_view = View::Images;
        assert_eq!(app.selected_image_id(), Some("sha256:def".into()));
    }
}
