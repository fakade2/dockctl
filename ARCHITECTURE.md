# dockctl — Architecture

> Universal container runtime TUI — Docker & Podman, built in Rust.
> https://dockctl.dev

## Overview

dockctl is a fast, terminal-native container management tool that replaces Docker Desktop entirely in the terminal. It supports both Docker and Podman through a unified runtime abstraction, works standalone in any terminal, and unlocks enhanced features when tmux is detected.

## Tech Stack

| Layer | Crate | Purpose |
|-------|-------|---------|
| TUI Framework | `ratatui` + `crossterm` | Cross-platform terminal rendering |
| Container API | `bollard` | Async Docker/Podman API via Unix socket |
| Async Runtime | `tokio` | Event loop, streaming, concurrency |
| CLI Args | `clap` | Startup flags, configuration |
| Serialization | `serde` / `serde_json` / `toml` | API payloads + config |
| Errors | `anyhow` / `thiserror` | Error handling |

## Architecture Layers

```
┌─────────────────────────────────────────────────────┐
│                     TUI Layer                        │
│  ┌───────┬──────────┬────────┬────────┬───────────┐ │
│  │Sidebar│ Main View│ Detail │ Logs   │ Stats     │ │
│  │(tabs) │ (table)  │ Panel  │ Panel  │ Panel     │ │
│  └───────┴──────────┴────────┴────────┴───────────┘ │
├─────────────────────────────────────────────────────┤
│                App State (shared)                    │
│   containers, images, volumes, networks, pods, events│
├─────────────────────────────────────────────────────┤
│            Container Runtime (trait)                  │
│  ┌──────────────┐  ┌──────────────────────────────┐ │
│  │    Docker     │  │          Podman              │ │
│  │   (bollard)   │  │   (bollard, compat API)      │ │
│  └──────────────┘  └──────────────────────────────┘ │
├─────────────────────────────────────────────────────┤
│               Connection Layer                       │
│  Local:  /var/run/docker.sock                        │
│          /run/user/{uid}/podman/podman.sock           │
└─────────────────────────────────────────────────────┘
```

## Multi-Runtime Support

dockctl supports Docker and Podman through a unified `ContainerRuntime` trait. Since Podman implements a Docker-compatible API, the `bollard` crate works with both — pointed at different sockets.

### Runtime Detection

On startup, dockctl auto-detects available runtimes:
1. Check for Docker socket at `/var/run/docker.sock`
2. Check for Podman socket at `/run/user/{uid}/podman/podman.sock`
3. Check `docker` / `podman` in PATH
4. User can override via `--runtime docker|podman` flag or config file

### Docker vs Podman Differences

| Feature | Docker | Podman |
|---------|--------|--------|
| Socket path | `/var/run/docker.sock` | `/run/user/{uid}/podman/podman.sock` |
| Daemon | Always running (dockerd) | Daemonless (socket on-demand) |
| Rootless | Requires config | Default |
| Compose | `docker compose` | `podman-compose` / `podman compose` |
| Pods | N/A | Native pod support (K8s-style) |
| K8s YAML | N/A | Generate/deploy K8s manifests from pods |
| Checkpointing | N/A | CRIU checkpoint/restore |
| API | Docker Engine API | Docker-compatible API |

## Views

### Core Views (v1)

1. **Dashboard** — Running containers overview, system resource usage, engine info, disk usage summary
2. **Containers** — List, start/stop/restart/remove, logs (live stream), exec shell, inspect, per-container resource stats
3. **Images** — List, pull, push, tag, remove, build history
4. **Volumes** — List, create, remove, inspect
5. **Networks** — List, create, remove, inspect, connected containers, port mapping visualization
6. **Compose** — Detect compose projects, up/down/restart stacks, per-service logs
7. **Builds** — Build images from Dockerfile, stream build output, layer caching info
8. **Pods** — *(Podman only)* Create, manage, inspect pods; generate K8s YAML from pods

## Key Features

- **Real-time stats**: CPU, memory, network I/O, disk I/O per container (sparkline charts)
- **Log search/filter**: Regex search across live log streams
- **Container grouping**: Group by Compose project or pod
- **Quick actions**: Keyboard-driven — `s` stop, `r` restart, `x` remove, `l` logs, `e` exec
- **Notifications**: Container crash/restart/OOM events shown inline
- **Event stream**: Real-time updates from container daemon

## Tmux Integration

dockctl works fully standalone. When tmux is detected, additional capabilities are unlocked:

| Action | Without Tmux | With Tmux |
|--------|-------------|-----------|
| View logs | Inline panel | Dedicated tmux pane |
| Exec shell | Suspend TUI, attach | New tmux split pane |
| Multi-container stats | Tabbed view | Tmux windows per container |
| Session restore | N/A | Full layout restoration |

### Tmux Features

- Detect tmux session on startup
- `exec` opens a real shell in a tmux split pane (proper TTY)
- Log tailing in a dedicated tmux pane
- Multi-container monitoring across tmux windows
- Session persistence — save/restore Docker workspace layouts

## Configuration

dockctl uses a TOML config file at `~/.config/dockctl/config.toml`:

```toml
[general]
default_runtime = "auto"          # auto | docker | podman
refresh_interval_ms = 1000
theme = "default"

[keybindings]
stop = "s"
restart = "r"
remove = "x"
logs = "l"
exec = "e"
search = "/"
```

## Project Structure

```
dockctl/
├── Cargo.toml
├── src/
│   ├── main.rs                # Entry point, arg parsing
│   ├── app.rs                 # App state machine
│   ├── config.rs              # Config file parsing (TOML)
│   ├── event.rs               # Input + container event handling
│   ├── ui/
│   │   ├── mod.rs
│   │   ├── dashboard.rs       # Dashboard view
│   │   ├── containers.rs      # Container list + detail
│   │   ├── images.rs          # Image management
│   │   ├── volumes.rs         # Volume management
│   │   ├── networks.rs        # Network management
│   │   ├── compose.rs         # Compose project view
│   │   ├── builds.rs          # Build view
│   │   ├── pods.rs            # Pods view (Podman only)
│   │   ├── logs.rs            # Log viewer widget
│   │   └── stats.rs           # Stats/charts widget
│   ├── runtime/
│   │   ├── mod.rs             # ContainerRuntime trait
│   │   ├── docker.rs          # Docker backend (bollard → docker.sock)
│   │   ├── podman.rs          # Podman backend (bollard → podman.sock)
│   │   └── detect.rs          # Auto-detect available runtimes
│   ├── services/
│   │   ├── mod.rs
│   │   ├── containers.rs      # Container operations
│   │   ├── images.rs          # Image operations
│   │   ├── volumes.rs         # Volume operations
│   │   ├── networks.rs        # Network operations
│   │   ├── compose.rs         # Compose operations
│   │   ├── pods.rs            # Pod operations (Podman)
│   │   └── system.rs          # System/engine info
│   └── tmux/
│       ├── mod.rs             # Tmux detection + integration
│       ├── panes.rs           # Pane management
│       └── sessions.rs        # Session save/restore
```

## Design Principles

1. **Speed** — Rust for performance, async for non-blocking I/O
2. **Universal** — Works with Docker and Podman through unified abstraction
3. **Cross-terminal** — Works in any terminal via crossterm backend
4. **Keyboard-first** — Every action accessible via keyboard shortcuts
5. **Progressive enhancement** — Works standalone, better with tmux
6. **Minimal footprint** — Single binary, no runtime dependencies beyond the container engine
7. **Configurable** — Keybindings, themes, host profiles, all via TOML config

---

## Future Improvements

Features planned for post-v1 releases:

### Multi-Host Support
- **SSH tunneling** — `dockctl --host ssh://user@server` (tunnels socket over SSH)
- **TCP** — `dockctl --host tcp://server:2375`
- **Host profiles** — Save named connections in config: `dockctl --profile production`
- **Host switcher** — Switch between hosts from within the TUI via keybinding

### Image Security Scanning
- **Vulnerability scanning** — Integrate with trivy or grype to scan images
- **Scan results view** — Severity breakdown (critical/high/medium/low) per image
- **Scan on pull** — Optionally auto-scan images when pulled

### Registry Browser
- **Search** — Search Docker Hub, GHCR, or private registries
- **Browse tags** — View available tags, architectures, and sizes
- **Pull from browser** — Pull images directly from the registry view

### Container File Browser
- Browse files inside a container without exec
- View/download individual files
- Tmux-enhanced: opens in a split pane

### Prune Manager
- **Preview mode** — See exactly what will be deleted and how much space is reclaimed
- **Selective prune** — Choose which categories to clean (images, containers, volumes, build cache)
- **Scheduled prune** — Configure auto-prune rules in config

### Resource History
- Historical CPU/memory charts, not just real-time snapshots
- Per-container usage trends over time
- Export stats to CSV/JSON

### Container Diff
- Show filesystem changes since container creation
- Layer-by-layer diff view

### Plugin System
- **Script plugins** — External scripts (any language) invoked via stdin/stdout JSON protocol
- **WASM plugins** — Sandboxed WebAssembly plugins for custom views or actions
- **Plugin directory** — `~/.config/dockctl/plugins/`
- **Use cases** — Custom dashboards, alerting integrations, CI/CD triggers, cost monitoring
