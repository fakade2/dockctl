# dockctl — Architecture

> Universal container runtime TUI — Docker & Podman, built in Rust.
> https://dockctl.dev

## Overview

dockctl is a theme-based container management platform. The core engine provides service hooks for container operations, and themes consume those hooks to build completely different UX paradigms.

## Architecture Layers

```
┌─────────────────────────────────────────────────────────────┐
│                      Theme Layer                             │
│                                                              │
│  Owns: ALL key handling, ALL view state, ALL rendering       │
│  Consumes: service hooks via ThemeAction                     │
│                                                              │
│  ┌─────────────────┐    ┌─────────────────────────────────┐ │
│  │   Dashboard      │    │          Operator               │ │
│  │   (lazygit)      │    │          (K9s)                  │ │
│  │                  │    │                                  │ │
│  │  - Panel layout  │    │  - Full-width tables            │ │
│  │  - Number keys   │    │  - Vim command mode (:cmd)      │ │
│  │  - Popup detail  │    │  - Inline detail/log views      │ │
│  │  - Accordion     │    │  - Autocomplete                 │ │
│  └─────────────────┘    │  - Sort/filter/wide mode        │ │
│                          │  - Status dot indicators         │ │
│                          └─────────────────────────────────┘ │
├─────────────────────────────────────────────────────────────┤
│                    Engine (main.rs)                           │
│                                                              │
│  Owns: event loop, tick refresh, async action execution      │
│  Does NOT own: key handling, view state, rendering           │
│                                                              │
│  1. Polls events (crossterm + tick timer)                    │
│  2. Dispatches to theme.handle_key()                         │
│  3. Executes ThemeAction (async container ops)               │
│  4. Refreshes data on tick (calls service hooks)             │
│  5. Calls theme.draw() to render                             │
├─────────────────────────────────────────────────────────────┤
│                    App State (app.rs)                         │
│                                                              │
│  Core data (shared across all themes):                       │
│  - containers, images, volumes, networks, pods               │
│  - system_info, container_stats                              │
│  - alerts, host CPU/RAM/Swap                                 │
│                                                              │
│  Theme-specific state (owned by themes, stored here          │
│  for simplicity):                                            │
│  - operator_resource, operator_command_mode, etc.            │
│  - log_view_*, detail_view_*, popup_*                        │
├─────────────────────────────────────────────────────────────┤
│                   Service Layer                              │
│                                                              │
│  Provides hooks — stateless business logic:                  │
│  - ContainerService: list, start, stop, restart, remove,     │
│    get_detail, get_logs, get_stats                           │
│  - ImageService: list, pull, remove                          │
│  - VolumeService: list, create, remove                       │
│  - NetworkService: list, create, remove                      │
│  - SystemService: info, disk_usage                           │
│  - ComposeService: detect_projects, up, down                 │
│  - PodService: list, create, start, stop, remove             │
├─────────────────────────────────────────────────────────────┤
│                Container Runtime (trait)                      │
│                                                              │
│  ContainerRuntime trait — unified API for Docker & Podman    │
│  ┌──────────────┐  ┌──────────────────────────────────────┐ │
│  │    Docker     │  │           Podman                     │ │
│  │   (bollard)   │  │   (bollard + CLI for pods)           │ │
│  └──────────────┘  └──────────────────────────────────────┘ │
│                                                              │
│  Local:  /var/run/docker.sock                                │
│          /run/user/{uid}/podman/podman.sock                   │
└─────────────────────────────────────────────────────────────┘
```

## Core Principle: Engine = Hooks, Theme = Everything Else

### What the Engine Does
- Provides **service hooks** (async functions to fetch/mutate container data)
- Runs the **event loop** (poll crossterm events, tick timer)
- Dispatches `theme.handle_key()` — does NOT interpret keys itself
- Executes **ThemeAction** requests (async container operations)
- Refreshes data on tick (calls service hooks, updates App state)
- Calls `theme.draw()` to render each frame

### What the Engine Does NOT Do
- Handle keys (no `if key == 'l' { open logs }` in main.rs)
- Own view state (no scroll positions, view modes, UI state)
- Decide rendering (no layout decisions)

### What Themes Do
- **ALL key handling** — navigation, commands, scroll, filter, sort
- **ALL view state** — scroll positions, follow mode, command input, filter text
- **ALL rendering** — layout, colors, widgets, popups vs inline views
- **Request data** from engine via `ThemeAction` enum
- **Request actions** via `ThemeAction` (stop, restart, remove, exec, fetch logs)

### ThemeAction — How Themes Talk to the Engine

```rust
pub enum ThemeAction {
    /// Key was handled by the theme. Engine does nothing.
    Consumed,

    /// Theme wants to quit the application.
    Quit,

    /// Theme wants to open a popup (dashboard theme uses this).
    OpenPopup { title: String, content: Vec<String> },

    /// Theme wants async data or action — engine fulfills it.
    /// (Future: FetchLogs, FetchDetail, StopContainer, etc.)
    NotHandled,  // Falls through to engine's global action handler
}
```

### Theme Trait

```rust
pub trait Theme: Send + Sync {
    /// Display name of the theme.
    fn name(&self) -> &str;

    /// Render the full UI frame.
    fn draw(&self, frame: &mut Frame, app: &App, config: &DockctlConfig);

    /// Handle a key event. Returns ThemeAction.
    fn handle_key(&self, key: KeyEvent, app: &mut App, config: &DockctlConfig) -> ThemeAction;
}
```

## Tech Stack

| Layer | Crate | Purpose |
|-------|-------|---------|
| TUI Framework | `ratatui` + `crossterm` | Cross-platform terminal rendering |
| Container API | `bollard` | Async Docker/Podman API via Unix socket |
| System Stats | `sysinfo` | Real host CPU/RAM/Swap metrics |
| Async Runtime | `tokio` | Event loop, streaming, concurrency |
| CLI Args | `clap` | Startup flags, configuration |
| Serialization | `serde` / `serde_json` / `toml` | API payloads + config |
| Errors | `anyhow` / `thiserror` | Error handling |

## Themes

### Operator (K9s-style) — default
- Full-width resource tables with configurable columns
- Vim-style `:command` mode with autocomplete
- Inline detail views and log viewer (not popups)
- Status dot indicators, sortable columns, wide mode
- Pulses dashboard (`:status`)

### Dashboard (lazygit-style)
- Multi-panel layout: alerts, resources, status (left) + accordion (right)
- Number keys 1-7 to focus panels
- Popup-based detail views

### Creating a New Theme
1. Create `src/themes/mytheme/mod.rs`
2. Implement the `Theme` trait (`name`, `draw`, `handle_key`)
3. Register in `src/themes/mod.rs` → `create_theme()`
4. Use service hooks via `ThemeAction` for async operations
5. Store view state in App or theme-local state

## Configuration

Config loaded from: `./config.toml` → `~/.config/dockctl/config.toml` → OS config dir

```toml
[general]
default_runtime = "auto"          # auto | docker | podman
refresh_interval_ms = 1000
theme = "operator"                # operator | dashboard

[layout]
left_panel_percent = 22           # dashboard theme: left panel width

[columns]
containers = ["Name", "State", "Ports", "CPU", "Memory", "ID", "Status"]
containers_wide = ["Name", "Image", "State", "Ports", "CPU", "Memory", "ID", "Command", "Status", "Created"]
images = ["Repository", "Tag", "ID", "Size", "Created"]
volumes = ["Name", "Driver", "Mountpoint", "Created"]
networks = ["Name", "ID", "Driver", "Scope", "Containers"]
```

## Project Structure

```
dockctl/
├── Cargo.toml
├── config.toml                    # Local config (project-level)
├── src/
│   ├── main.rs                    # Engine: event loop, action executor
│   ├── app.rs                     # App state (core data + theme state)
│   ├── config.rs                  # Config parsing (TOML)
│   ├── event.rs                   # Event handler (crossterm + tick)
│   ├── themes/
│   │   ├── mod.rs                 # Theme trait + create_theme()
│   │   ├── dashboard/mod.rs       # Dashboard theme (lazygit-style)
│   │   └── operator/mod.rs        # Operator theme (K9s-style)
│   ├── ui/                        # Shared rendering utilities
│   │   ├── mod.rs                 # Shared helpers + dashboard renderer
│   │   ├── alerts.rs              # Alerts panel (dashboard)
│   │   ├── resources.rs           # Resources panel (dashboard)
│   │   ├── status.rs              # Status panel (dashboard)
│   │   ├── containers.rs          # Container table (dashboard)
│   │   ├── images.rs              # Image table (dashboard)
│   │   ├── volumes.rs             # Volume table (dashboard)
│   │   ├── networks.rs            # Network table (dashboard)
│   │   ├── preview.rs             # Detail content generators
│   │   ├── logs.rs                # Log viewer widget
│   │   └── stats.rs               # Stats widget
│   ├── runtime/
│   │   ├── mod.rs                 # ContainerRuntime trait + types
│   │   ├── docker.rs              # Docker backend (bollard)
│   │   ├── podman.rs              # Podman backend (bollard + CLI)
│   │   └── detect.rs              # Auto-detect runtimes
│   ├── services/
│   │   ├── mod.rs                 # Service exports
│   │   ├── containers.rs          # Container hooks
│   │   ├── images.rs              # Image hooks
│   │   ├── volumes.rs             # Volume hooks
│   │   ├── networks.rs            # Network hooks
│   │   ├── compose.rs             # Compose hooks
│   │   ├── pods.rs                # Pod hooks
│   │   └── system.rs              # System hooks
│   └── tmux/
│       ├── mod.rs                 # Tmux detection
│       ├── panes.rs               # Pane management
│       └── sessions.rs            # Session save/restore
```

## Design Principles

1. **Engine = Hooks** — Core provides service hooks, themes consume them
2. **Theme = Everything Else** — ALL key handling, view state, rendering in themes
3. **Speed** — Rust for performance, async for non-blocking I/O
4. **Universal** — Docker and Podman through unified runtime trait
5. **Cross-terminal** — Works in any terminal via crossterm
6. **Keyboard-first** — Every action accessible via keyboard
7. **Configurable** — Themes, columns, keybindings, layout via TOML
8. **Extensible** — New themes implement one trait

## Future

- Multi-host support (SSH/TCP)
- Image security scanning (trivy/grype)
- Registry browser
- Container file browser
- Prune manager
- Resource history / trends
- Plugin system (WASM/script)
- More themes (community-contributed)
