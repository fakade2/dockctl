use crate::runtime::{
    ContainerSummary, ImageSummary, NetworkSummary, PodSummary, SystemInfo, VolumeSummary,
};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// PanelId — identifies each of the 7 panels in the lazygit-style layout
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PanelId {
    Alerts,     // 1 — left column
    Resources,  // 2 — left column
    Status,     // 3 — left column
    Containers, // 4 — right column
    Images,     // 5 — right column
    Volumes,    // 6 — right column
    Networks,   // 7 — right column
}

impl PanelId {
    /// All panels in order (1–7).
    pub const ALL: &'static [PanelId] = &[
        PanelId::Alerts,
        PanelId::Resources,
        PanelId::Status,
        PanelId::Containers,
        PanelId::Images,
        PanelId::Volumes,
        PanelId::Networks,
    ];

    /// Left-column panels (always visible, stacked).
    pub const LEFT: &'static [PanelId] = &[PanelId::Alerts, PanelId::Resources, PanelId::Status];

    /// Right-column panels (accordion — one expanded at a time).
    pub const RIGHT: &'static [PanelId] = &[
        PanelId::Containers,
        PanelId::Images,
        PanelId::Volumes,
        PanelId::Networks,
    ];

    /// Numeric shortcut (1–7).
    pub fn number(self) -> u8 {
        match self {
            PanelId::Alerts => 1,
            PanelId::Resources => 2,
            PanelId::Status => 3,
            PanelId::Containers => 4,
            PanelId::Images => 5,
            PanelId::Volumes => 6,
            PanelId::Networks => 7,
        }
    }

    /// Resolve a 1-based number back to a `PanelId`.
    pub fn from_number(n: u8) -> Option<Self> {
        match n {
            1 => Some(PanelId::Alerts),
            2 => Some(PanelId::Resources),
            3 => Some(PanelId::Status),
            4 => Some(PanelId::Containers),
            5 => Some(PanelId::Images),
            6 => Some(PanelId::Volumes),
            7 => Some(PanelId::Networks),
            _ => None,
        }
    }

    /// Human-readable label for the panel.
    pub fn label(self) -> &'static str {
        match self {
            PanelId::Alerts => "Alerts",
            PanelId::Resources => "Resources",
            PanelId::Status => "Status",
            PanelId::Containers => "Containers",
            PanelId::Images => "Images",
            PanelId::Volumes => "Volumes",
            PanelId::Networks => "Networks",
        }
    }

    /// Whether this panel belongs to the left column.
    pub fn is_left(self) -> bool {
        matches!(self, PanelId::Alerts | PanelId::Resources | PanelId::Status)
    }

    /// Whether this panel belongs to the right column.
    pub fn is_right(self) -> bool {
        !self.is_left()
    }
}

// ---------------------------------------------------------------------------
// Column — which side of the two-column layout
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Column {
    Left,
    Right,
}

// ---------------------------------------------------------------------------
// Alert — system alerts displayed in the Alerts panel
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
    // Panel navigation
    /// Which panel currently has keyboard focus (1–7).
    pub active_panel: PanelId,
    /// Which right-column panel is expanded (accordion). Default: Containers.
    pub expanded_right: PanelId,
    pub input_mode: InputMode,
    pub search_query: String,

    // Popup state
    /// Whether the explain/detail popup is open.
    pub show_popup: bool,
    /// Lines to display inside the popup.
    pub popup_content: Vec<String>,
    /// Title bar text for the popup.
    pub popup_title: String,
    /// Vertical scroll offset inside the popup.
    pub popup_scroll: u16,

    // Domain data
    pub containers: Vec<ContainerSummary>,
    pub images: Vec<ImageSummary>,
    pub volumes: Vec<VolumeSummary>,
    pub networks: Vec<NetworkSummary>,
    pub pods: Vec<PodSummary>,
    pub system_info: Option<SystemInfo>,

    // Per-container stats cache: container_id → (cpu_percent, memory_usage_bytes)
    pub container_stats: HashMap<String, (f64, u64)>,

    // Per-panel selection index (each panel tracks its own cursor independently).
    pub selection: HashMap<PanelId, usize>,

    // Per-panel horizontal column offset (for left/right scrolling in tables).
    pub column_offset: HashMap<PanelId, usize>,

    // Host system stats (real OS-level metrics from sysinfo)
    pub host_cpu_percent: f64,
    pub host_memory_used: u64,
    pub host_memory_total: u64,
    pub host_swap_used: u64,
    pub host_swap_total: u64,

    // Alerts & counts
    pub alerts: Vec<Alert>,
    pub stale_images_count: usize,
    pub unused_volumes_count: usize,
    pub dangling_images_count: usize,

    // Runtime capabilities
    pub supports_pods: bool,

    // Operator theme state
    pub operator_resource: String,
    pub operator_command_mode: bool,
    pub operator_command_input: String,
    pub operator_filter: String,
    pub operator_filter_active: bool,

    // View history (for Esc-back navigation)
    pub view_history: Vec<String>, // stack of previous operator_resource values

    // Sort state
    pub sort_column: Option<String>, // which column is sorted
    pub sort_ascending: bool,        // sort direction

    // Wide mode (show extra columns)
    pub wide_mode: bool,

    // Inline detail view (K9s-style — replaces table, not a popup)
    pub detail_view_active: bool,
    pub detail_view_title: String,
    pub detail_view_content: Vec<String>,
    pub detail_view_scroll: u16,

    // Status dot indicators cache: container_id -> health status
    // true = healthy (green dot), false = unhealthy (red dot), None = unknown
    pub container_health: HashMap<String, Option<bool>>,

    // Lifecycle
    pub should_quit: bool,
}

impl App {
    /// Create a new `App`. Pass `true` when the runtime is Podman (pod support).
    pub fn new(supports_pods: bool) -> Self {
        let mut selection = HashMap::new();
        let mut column_offset = HashMap::new();
        for &panel in PanelId::ALL {
            selection.insert(panel, 0);
            column_offset.insert(panel, 0);
        }

        Self {
            active_panel: PanelId::Containers,
            expanded_right: PanelId::Containers,
            input_mode: InputMode::Normal,
            search_query: String::new(),

            show_popup: false,
            popup_content: Vec::new(),
            popup_title: String::new(),
            popup_scroll: 0,

            containers: Vec::new(),
            images: Vec::new(),
            volumes: Vec::new(),
            networks: Vec::new(),
            pods: Vec::new(),
            system_info: None,

            container_stats: HashMap::new(),

            selection,
            column_offset,

            host_cpu_percent: 0.0,
            host_memory_used: 0,
            host_memory_total: 0,
            host_swap_used: 0,
            host_swap_total: 0,

            alerts: Vec::new(),
            stale_images_count: 0,
            unused_volumes_count: 0,
            dangling_images_count: 0,

            supports_pods,

            operator_resource: "containers".to_string(),
            operator_command_mode: false,
            operator_command_input: String::new(),
            operator_filter: String::new(),
            operator_filter_active: false,

            view_history: Vec::new(),
            sort_column: None,
            sort_ascending: true,
            wide_mode: false,
            container_health: HashMap::new(),

            detail_view_active: false,
            detail_view_title: String::new(),
            detail_view_content: Vec::new(),
            detail_view_scroll: 0,

            should_quit: false,
        }
    }

    // ------------------------------------------------------------------
    // View history (Esc-back navigation)
    // ------------------------------------------------------------------

    /// Push current resource to history and switch to new one (for Esc-back).
    pub fn push_view(&mut self, new_resource: &str) {
        self.view_history.push(self.operator_resource.clone());
        self.operator_resource = new_resource.to_string();
    }

    /// Pop back to previous view. Returns true if there was a history entry.
    pub fn pop_view(&mut self) -> bool {
        if let Some(prev) = self.view_history.pop() {
            self.operator_resource = prev;
            true
        } else {
            false
        }
    }

    // ------------------------------------------------------------------
    // Sort
    // ------------------------------------------------------------------

    /// Toggle sort on a column. If already sorted by this column, reverse direction.
    pub fn toggle_sort(&mut self, column: &str) {
        if self.sort_column.as_deref() == Some(column) {
            self.sort_ascending = !self.sort_ascending;
        } else {
            self.sort_column = Some(column.to_string());
            self.sort_ascending = true;
        }
    }

    // ------------------------------------------------------------------
    // Wide mode
    // ------------------------------------------------------------------

    /// Toggle wide mode.
    pub fn toggle_wide_mode(&mut self) {
        self.wide_mode = !self.wide_mode;
    }

    /// Open an inline detail view (replaces the table, K9s-style).
    pub fn open_detail_view(&mut self, title: String, content: Vec<String>) {
        self.detail_view_active = true;
        self.detail_view_title = title;
        self.detail_view_content = content;
        self.detail_view_scroll = 0;
    }

    /// Close the inline detail view and return to the table.
    pub fn close_detail_view(&mut self) {
        self.detail_view_active = false;
        self.detail_view_content.clear();
        self.detail_view_title.clear();
        self.detail_view_scroll = 0;
    }

    // ------------------------------------------------------------------
    // Panel switching
    // ------------------------------------------------------------------

    /// Focus a panel by its `PanelId`. If the panel is in the right column
    /// it also becomes the expanded (accordion) panel.
    pub fn focus_panel(&mut self, panel: PanelId) {
        self.active_panel = panel;
        if panel.is_right() {
            self.expanded_right = panel;
        }
    }

    /// Which column currently has focus.
    pub fn active_column(&self) -> Column {
        if self.active_panel.is_left() {
            Column::Left
        } else {
            Column::Right
        }
    }

    /// Move to the next panel in order (1 -> 2 -> ... -> 7 -> 1), wrapping.
    pub fn next_panel(&mut self) {
        let all = PanelId::ALL;
        let current_pos = all
            .iter()
            .position(|&p| p == self.active_panel)
            .unwrap_or(0);
        let next_pos = (current_pos + 1) % all.len();
        self.focus_panel(all[next_pos]);
    }

    /// Move to the previous panel in order (1 <- 2 <- ... <- 7 <- 1), wrapping.
    pub fn previous_panel(&mut self) {
        let all = PanelId::ALL;
        let current_pos = all
            .iter()
            .position(|&p| p == self.active_panel)
            .unwrap_or(0);
        let prev_pos = if current_pos == 0 {
            all.len() - 1
        } else {
            current_pos - 1
        };
        self.focus_panel(all[prev_pos]);
    }

    // ------------------------------------------------------------------
    // List selection (per-panel)
    // ------------------------------------------------------------------

    /// Move the cursor down one item in the active panel's list, wrapping.
    pub fn next_item(&mut self) {
        let len = self.current_list_len();
        if len > 0 {
            let sel = self.selection.entry(self.active_panel).or_insert(0);
            *sel = (*sel + 1) % len;
        }
    }

    /// Move the cursor up one item in the active panel's list, wrapping.
    pub fn previous_item(&mut self) {
        let len = self.current_list_len();
        if len > 0 {
            let sel = self.selection.entry(self.active_panel).or_insert(0);
            *sel = if *sel == 0 { len - 1 } else { *sel - 1 };
        }
    }

    /// Get the selected index for the currently active panel.
    pub fn selected_index(&self) -> usize {
        *self.selection.get(&self.active_panel).unwrap_or(&0)
    }

    /// Get the selected index for a specific panel.
    pub fn panel_selection(&self, panel: PanelId) -> usize {
        *self.selection.get(&panel).unwrap_or(&0)
    }

    /// Get the horizontal column offset for a panel.
    pub fn panel_column_offset(&self, panel: PanelId) -> usize {
        *self.column_offset.get(&panel).unwrap_or(&0)
    }

    /// Scroll columns right for the active panel (capped by max_columns).
    pub fn scroll_columns_right(&mut self, max_columns: usize) {
        let offset = self.column_offset.entry(self.active_panel).or_insert(0);
        if max_columns > 0 && *offset < max_columns - 1 {
            *offset += 1;
        }
    }

    /// Scroll columns left for the active panel.
    pub fn scroll_columns_left(&mut self) {
        let offset = self.column_offset.entry(self.active_panel).or_insert(0);
        *offset = offset.saturating_sub(1);
    }

    /// Length of the list in the currently active panel.
    pub fn current_list_len(&self) -> usize {
        self.panel_list_len(self.active_panel)
    }

    /// Length of the list for a given panel.
    pub fn panel_list_len(&self, panel: PanelId) -> usize {
        match panel {
            PanelId::Alerts => self.alerts.len(),
            PanelId::Resources => 4, // containers, images, volumes, networks
            PanelId::Status => 6,    // System: CPU, RAM, Swap + Docker: CPU, RAM, Disk
            PanelId::Containers => self.containers.len(),
            PanelId::Images => self.images.len(),
            PanelId::Volumes => self.volumes.len(),
            PanelId::Networks => self.networks.len(),
        }
    }

    // ------------------------------------------------------------------
    // Selected-item getters
    // ------------------------------------------------------------------

    /// Return the container ID at the Containers panel's selection, if any.
    pub fn selected_container_id(&self) -> Option<String> {
        let idx = self.panel_selection(PanelId::Containers);
        self.containers.get(idx).map(|c| c.id.clone())
    }

    /// Return the image ID at the Images panel's selection, if any.
    pub fn selected_image_id(&self) -> Option<String> {
        let idx = self.panel_selection(PanelId::Images);
        self.images.get(idx).map(|i| i.id.clone())
    }

    /// Return the volume name at the Volumes panel's selection, if any.
    pub fn selected_volume_name(&self) -> Option<String> {
        let idx = self.panel_selection(PanelId::Volumes);
        self.volumes.get(idx).map(|v| v.name.clone())
    }

    /// Return the network ID at the Networks panel's selection, if any.
    pub fn selected_network_id(&self) -> Option<String> {
        let idx = self.panel_selection(PanelId::Networks);
        self.networks.get(idx).map(|n| n.id.clone())
    }

    // ------------------------------------------------------------------
    // Popup
    // ------------------------------------------------------------------

    /// Open the detail/explain popup with the given title and content lines.
    pub fn open_popup(&mut self, title: String, content: Vec<String>) {
        self.show_popup = true;
        self.popup_title = title;
        self.popup_content = content;
        self.popup_scroll = 0;
    }

    /// Close the popup and reset its state.
    pub fn close_popup(&mut self) {
        self.show_popup = false;
        self.popup_content.clear();
        self.popup_title.clear();
        self.popup_scroll = 0;
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -- Helpers for building test data ------------------------------------

    fn make_container(id: &str) -> ContainerSummary {
        ContainerSummary {
            id: id.into(),
            name: id.into(),
            names: vec![id.into()],
            image: "img".into(),
            image_id: format!("sha256:{id}"),
            command: "/bin/sh".into(),
            state: "running".into(),
            status: "Up 1 hour".into(),
            created: 1700000000,
            ports: vec![],
            labels: HashMap::new(),
        }
    }

    fn make_image(id: &str) -> ImageSummary {
        ImageSummary {
            id: id.into(),
            repo_tags: vec![format!("{id}:latest")],
            repo_digests: vec![],
            size: 100_000,
            created: 1700000000,
            labels: HashMap::new(),
        }
    }

    fn make_volume(name: &str) -> VolumeSummary {
        VolumeSummary {
            name: name.into(),
            driver: "local".into(),
            mountpoint: format!("/var/lib/docker/volumes/{name}/_data"),
            labels: HashMap::new(),
            scope: "local".into(),
            created_at: "2024-01-01T00:00:00Z".into(),
        }
    }

    fn make_network(id: &str) -> NetworkSummary {
        NetworkSummary {
            id: id.into(),
            name: id.into(),
            driver: "bridge".into(),
            scope: "local".into(),
            internal: false,
            ipv6_enabled: false,
            labels: HashMap::new(),
            containers: HashMap::new(),
        }
    }

    // -- Tests ------------------------------------------------------------

    #[test]
    fn new_app_defaults() {
        let app = App::new(false);
        assert_eq!(app.active_panel, PanelId::Containers);
        assert_eq!(app.expanded_right, PanelId::Containers);
        assert!(!app.show_popup);
        assert_eq!(app.input_mode, InputMode::Normal);
        assert!(!app.should_quit);
        assert!(app.popup_content.is_empty());
        assert!(app.popup_title.is_empty());
        assert_eq!(app.popup_scroll, 0);
    }

    #[test]
    fn focus_panel_expands_right() {
        let mut app = App::new(false);
        assert_eq!(app.expanded_right, PanelId::Containers);

        app.focus_panel(PanelId::Images);
        assert_eq!(app.active_panel, PanelId::Images);
        assert_eq!(app.expanded_right, PanelId::Images);

        app.focus_panel(PanelId::Volumes);
        assert_eq!(app.active_panel, PanelId::Volumes);
        assert_eq!(app.expanded_right, PanelId::Volumes);

        app.focus_panel(PanelId::Networks);
        assert_eq!(app.active_panel, PanelId::Networks);
        assert_eq!(app.expanded_right, PanelId::Networks);
    }

    #[test]
    fn focus_panel_left_doesnt_change_expanded() {
        let mut app = App::new(false);
        assert_eq!(app.expanded_right, PanelId::Containers);

        // Focus a right panel first to change expanded_right
        app.focus_panel(PanelId::Images);
        assert_eq!(app.expanded_right, PanelId::Images);

        // Now focus left panels — expanded_right should stay on Images
        app.focus_panel(PanelId::Alerts);
        assert_eq!(app.active_panel, PanelId::Alerts);
        assert_eq!(app.expanded_right, PanelId::Images);

        app.focus_panel(PanelId::Resources);
        assert_eq!(app.active_panel, PanelId::Resources);
        assert_eq!(app.expanded_right, PanelId::Images);

        app.focus_panel(PanelId::Status);
        assert_eq!(app.active_panel, PanelId::Status);
        assert_eq!(app.expanded_right, PanelId::Images);
    }

    #[test]
    fn panel_cycling_wraps() {
        let mut app = App::new(false);

        // Start on Containers (panel 4). Cycle forward through all 7 and verify wrap.
        app.focus_panel(PanelId::Networks); // panel 7
        assert_eq!(app.active_panel, PanelId::Networks);

        app.next_panel(); // should wrap to panel 1 (Alerts)
        assert_eq!(app.active_panel, PanelId::Alerts);

        // Cycle backward from Alerts -> Networks
        app.previous_panel();
        assert_eq!(app.active_panel, PanelId::Networks);

        // Full forward cycle: visit every panel
        let mut visited = Vec::new();
        app.focus_panel(PanelId::Alerts);
        visited.push(app.active_panel);
        for _ in 0..7 {
            app.next_panel();
            visited.push(app.active_panel);
        }
        // After 7 steps from Alerts we should be back at Alerts
        assert_eq!(visited.first(), visited.last());
        // All 7 panels should appear in the first 7 entries
        let unique: std::collections::HashSet<PanelId> = visited[..7].iter().copied().collect();
        assert_eq!(unique.len(), 7);
    }

    #[test]
    fn item_navigation_per_panel() {
        let mut app = App::new(false);
        app.containers = vec![
            make_container("a"),
            make_container("b"),
            make_container("c"),
        ];
        app.images = vec![make_image("x"), make_image("y")];

        // Navigate in Containers panel
        app.focus_panel(PanelId::Containers);
        assert_eq!(app.selected_index(), 0);
        app.next_item();
        assert_eq!(app.selected_index(), 1);
        app.next_item();
        assert_eq!(app.selected_index(), 2);

        // Switch to Images panel — independent selection
        app.focus_panel(PanelId::Images);
        assert_eq!(app.selected_index(), 0);
        app.next_item();
        assert_eq!(app.selected_index(), 1);

        // Switch back to Containers — selection preserved at 2
        app.focus_panel(PanelId::Containers);
        assert_eq!(app.selected_index(), 2);

        // Images still at 1
        assert_eq!(app.panel_selection(PanelId::Images), 1);
    }

    #[test]
    fn item_navigation_wraps() {
        let mut app = App::new(false);
        app.containers = vec![make_container("a"), make_container("b")];

        app.focus_panel(PanelId::Containers);
        assert_eq!(app.selected_index(), 0);
        app.next_item();
        assert_eq!(app.selected_index(), 1);
        app.next_item(); // wraps to 0
        assert_eq!(app.selected_index(), 0);
        app.previous_item(); // wraps to 1
        assert_eq!(app.selected_index(), 1);
    }

    #[test]
    fn item_navigation_empty_list() {
        let mut app = App::new(false);
        app.focus_panel(PanelId::Containers);
        // No containers — should not crash
        app.next_item();
        app.previous_item();
        assert_eq!(app.selected_index(), 0);
    }

    #[test]
    fn popup_open_close() {
        let mut app = App::new(false);

        assert!(!app.show_popup);
        assert!(app.popup_title.is_empty());
        assert!(app.popup_content.is_empty());

        app.open_popup("Test Popup".into(), vec!["line 1".into(), "line 2".into()]);
        assert!(app.show_popup);
        assert_eq!(app.popup_title, "Test Popup");
        assert_eq!(app.popup_content.len(), 2);
        assert_eq!(app.popup_scroll, 0);

        // Simulate scrolling
        app.popup_scroll = 5;

        app.close_popup();
        assert!(!app.show_popup);
        assert!(app.popup_title.is_empty());
        assert!(app.popup_content.is_empty());
        assert_eq!(app.popup_scroll, 0);
    }

    #[test]
    fn panel_id_number_roundtrip() {
        for &panel in PanelId::ALL {
            let n = panel.number();
            let roundtripped = PanelId::from_number(n);
            assert_eq!(roundtripped, Some(panel), "roundtrip failed for {panel:?}");
        }
    }

    #[test]
    fn all_panels_have_unique_numbers() {
        let numbers: Vec<u8> = PanelId::ALL.iter().map(|p| p.number()).collect();
        let unique: std::collections::HashSet<u8> = numbers.iter().copied().collect();
        assert_eq!(numbers.len(), unique.len());
    }

    #[test]
    fn left_right_classification() {
        for &panel in PanelId::LEFT {
            assert!(
                panel.is_left(),
                "{panel:?} should be left but is_left() returned false"
            );
            assert!(
                !panel.is_right(),
                "{panel:?} should be left but is_right() returned true"
            );
        }
        for &panel in PanelId::RIGHT {
            assert!(
                panel.is_right(),
                "{panel:?} should be right but is_right() returned false"
            );
            assert!(
                !panel.is_left(),
                "{panel:?} should be right but is_left() returned true"
            );
        }
        // Every panel is in exactly one column
        assert_eq!(
            PanelId::LEFT.len() + PanelId::RIGHT.len(),
            PanelId::ALL.len()
        );
    }

    #[test]
    fn from_number_out_of_range_returns_none() {
        assert_eq!(PanelId::from_number(0), None);
        assert_eq!(PanelId::from_number(8), None);
        assert_eq!(PanelId::from_number(255), None);
    }

    #[test]
    fn active_column_tracks_active_panel() {
        let mut app = App::new(false);

        app.focus_panel(PanelId::Alerts);
        assert_eq!(app.active_column(), Column::Left);

        app.focus_panel(PanelId::Resources);
        assert_eq!(app.active_column(), Column::Left);

        app.focus_panel(PanelId::Status);
        assert_eq!(app.active_column(), Column::Left);

        app.focus_panel(PanelId::Containers);
        assert_eq!(app.active_column(), Column::Right);

        app.focus_panel(PanelId::Images);
        assert_eq!(app.active_column(), Column::Right);

        app.focus_panel(PanelId::Volumes);
        assert_eq!(app.active_column(), Column::Right);

        app.focus_panel(PanelId::Networks);
        assert_eq!(app.active_column(), Column::Right);
    }

    #[test]
    fn selected_getters_return_correct_ids() {
        let mut app = App::new(false);
        app.containers = vec![make_container("c1"), make_container("c2")];
        app.images = vec![make_image("img1"), make_image("img2")];
        app.volumes = vec![make_volume("vol1"), make_volume("vol2")];
        app.networks = vec![make_network("net1"), make_network("net2")];

        // Default selection is 0 for all panels
        assert_eq!(app.selected_container_id(), Some("c1".into()));
        assert_eq!(app.selected_image_id(), Some("img1".into()));
        assert_eq!(app.selected_volume_name(), Some("vol1".into()));
        assert_eq!(app.selected_network_id(), Some("net1".into()));

        // Advance selection in Containers panel
        app.focus_panel(PanelId::Containers);
        app.next_item();
        assert_eq!(app.selected_container_id(), Some("c2".into()));
        // Other panels unaffected
        assert_eq!(app.selected_image_id(), Some("img1".into()));
    }

    #[test]
    fn selected_getters_return_none_on_empty() {
        let app = App::new(false);
        assert_eq!(app.selected_container_id(), None);
        assert_eq!(app.selected_image_id(), None);
        assert_eq!(app.selected_volume_name(), None);
        assert_eq!(app.selected_network_id(), None);
    }

    #[test]
    fn panel_list_len_for_fixed_panels() {
        let app = App::new(false);
        assert_eq!(app.panel_list_len(PanelId::Resources), 4);
        assert_eq!(app.panel_list_len(PanelId::Status), 6);
        assert_eq!(app.panel_list_len(PanelId::Alerts), 0); // no alerts initially
    }

    #[test]
    fn panel_list_len_reflects_data() {
        let mut app = App::new(false);
        app.containers = vec![make_container("a")];
        app.images = vec![make_image("x"), make_image("y")];
        app.alerts.push(Alert {
            level: AlertLevel::Warning,
            message: "dangling images".into(),
        });

        assert_eq!(app.panel_list_len(PanelId::Containers), 1);
        assert_eq!(app.panel_list_len(PanelId::Images), 2);
        assert_eq!(app.panel_list_len(PanelId::Volumes), 0);
        assert_eq!(app.panel_list_len(PanelId::Networks), 0);
        assert_eq!(app.panel_list_len(PanelId::Alerts), 1);
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
    fn all_labels_are_nonempty() {
        for &panel in PanelId::ALL {
            assert!(!panel.label().is_empty(), "{panel:?} has an empty label");
        }
    }

    #[test]
    fn view_history_push_pop() {
        let mut app = App::new(false);
        assert_eq!(app.operator_resource, "containers");

        // Push two views
        app.push_view("images");
        assert_eq!(app.operator_resource, "images");

        app.push_view("volumes");
        assert_eq!(app.operator_resource, "volumes");

        // Pop back — should restore "images"
        assert!(app.pop_view());
        assert_eq!(app.operator_resource, "images");

        // Pop back — should restore "containers"
        assert!(app.pop_view());
        assert_eq!(app.operator_resource, "containers");

        // Pop on empty history — returns false, resource unchanged
        assert!(!app.pop_view());
        assert_eq!(app.operator_resource, "containers");
    }

    #[test]
    fn toggle_sort_column() {
        let mut app = App::new(false);
        assert!(app.sort_column.is_none());
        assert!(app.sort_ascending);

        // First toggle on "Name" — sets column, ascending
        app.toggle_sort("Name");
        assert_eq!(app.sort_column.as_deref(), Some("Name"));
        assert!(app.sort_ascending);

        // Toggle same column — reverses direction
        app.toggle_sort("Name");
        assert_eq!(app.sort_column.as_deref(), Some("Name"));
        assert!(!app.sort_ascending);

        // Toggle different column — resets to ascending
        app.toggle_sort("State");
        assert_eq!(app.sort_column.as_deref(), Some("State"));
        assert!(app.sort_ascending);
    }

    #[test]
    fn toggle_wide_mode() {
        let mut app = App::new(false);
        assert!(!app.wide_mode);

        app.toggle_wide_mode();
        assert!(app.wide_mode);

        app.toggle_wide_mode();
        assert!(!app.wide_mode);
    }
}
