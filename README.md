# dockctl

> Universal container runtime TUI — Docker & Podman, built in Rust.

[Screenshot placeholder]

## What is dockctl?

dockctl is a fast, terminal-native container management tool that replaces Docker Desktop. It supports Docker and Podman through a unified runtime abstraction and provides a rich TUI experience with multiple themes.

## Features

- **Two themes**: Dashboard (lazygit-style multi-panel) and Operator (K9s-style full-width tables)
- **Vim-style command mode**: `:containers`, `:images`, `:volumes`, `:networks`, `:status`, `:q`
- **Real-time stats**: CPU, memory, network I/O per container
- **System monitoring**: Host CPU, RAM, Swap + Docker aggregate stats
- **Smart alerts**: Dangling images, unused volumes, stopped containers
- **Configurable columns**: Add, remove, reorder columns per resource via TOML
- **Wide mode**: Toggle extra columns with Ctrl+W
- **Sortable tables**: Shift+O/N/S/A to sort by column
- **Filter/search**: `/` for regex filtering across any resource
- **Autocomplete**: Ghost text suggestions in command mode (Tab to accept)
- **Inline detail views**: `d` or Enter to drill into any resource
- **Container actions**: Stop, restart, remove, logs, exec — all from the keyboard
- **Tmux integration**: Exec into containers in tmux split panes
- **Docker + Podman**: Unified runtime abstraction, auto-detected

## Installation

### From binary (recommended)

Download the latest release for your platform:

```bash
# macOS (Apple Silicon)
curl -L https://github.com/fakade2/dockctl/releases/latest/download/dockctl-darwin-arm64 -o dockctl
chmod +x dockctl && sudo mv dockctl /usr/local/bin/

# macOS (Intel)
curl -L https://github.com/fakade2/dockctl/releases/latest/download/dockctl-darwin-amd64 -o dockctl
chmod +x dockctl && sudo mv dockctl /usr/local/bin/

# Linux (x86_64)
curl -L https://github.com/fakade2/dockctl/releases/latest/download/dockctl-linux-amd64 -o dockctl
chmod +x dockctl && sudo mv dockctl /usr/local/bin/
```

### From source

```bash
git clone https://github.com/fakade2/dockctl.git
cd dockctl
cargo build --release
./target/release/dockctl
```

## Usage

```bash
dockctl              # auto-detects Docker or Podman
dockctl --runtime docker   # force Docker
dockctl --runtime podman   # force Podman
```

## Themes

### Operator (K9s-style) — default

Full-width tables with vim-style command mode, inline detail views, and system stats header.

```
┌──────────────────────────────────────────────────────────────┐
│ dockctl    Docker › Containers(9)    v29.2.1 · 9↑ 2↓ · 16GB │
│ CPU: 12%  RAM: 68%  Swap: 4%                       8 cores  │
├──────────────────────────────────────────────────────────────┤
│ :containers                                                  │
├──────────────────────────────────────────────────────────────┤
│ NAME        STATE    PORTS       CPU    MEM     ID    STATUS │
│ ● nginx     running  8080:80    2.1%   45MB    a1b2  Up 5d  │
│ ● postgres  running  5432:5432  1.8%   256MB   f6e5  Up 5d  │
├──────────────────────────────────────────────────────────────┤
│ : cmd  / filter  d describe  s stop  ? help    rows:9  v0.0.1│
└──────────────────────────────────────────────────────────────┘
```

### Dashboard (lazygit-style)

Multi-panel layout with alerts, resources, status on the left and accordion containers/images/volumes/networks on the right.

Switch themes in `config.toml`:
```toml
[general]
theme = "operator"    # or "dashboard"
```

## Keybindings (Operator theme)

### Navigation
| Key | Action |
|-----|--------|
| `j` / `↓` | Move down |
| `k` / `↑` | Move up |
| `h` / `←` | Scroll columns left |
| `l` / `→` | Scroll columns right |
| `Esc` | Command mode / back |

### Commands
| Key | Action |
|-----|--------|
| `:` | Enter command mode |
| `:containers` | Switch to containers |
| `:images` | Switch to images |
| `:volumes` | Switch to volumes |
| `:networks` | Switch to networks |
| `:status` | System dashboard |
| `:q` | Quit |
| `/` | Filter (regex) |
| `Tab` | Accept autocomplete |

### Views
| Key | Action |
|-----|--------|
| `d` | Describe (inline detail) |
| `e` / `Enter` | Explain (inline detail) |
| `?` | Keybindings help |
| `Ctrl+A` | Resource aliases |

### Sort & Display
| Key | Action |
|-----|--------|
| `Shift+O` | Cycle sort column |
| `Shift+N` | Sort by name |
| `Shift+S` | Sort by state |
| `Shift+A` | Sort by age |
| `Ctrl+W` | Toggle wide mode |

### Container Actions
| Key | Action |
|-----|--------|
| `s` | Stop container |
| `r` | Restart container |
| `x` | Remove container |
| `l` | View logs |
| `E` | Exec into container (tmux) |

## Configuration

Config file: `./config.toml` or `~/.config/dockctl/config.toml`

```toml
[general]
default_runtime = "auto"    # auto | docker | podman
refresh_interval_ms = 1000
theme = "operator"          # operator | dashboard

[layout]
left_panel_percent = 22     # dashboard theme: left panel width

[columns]
containers = ["Name", "State", "Ports", "CPU", "Memory", "ID", "Status"]
images = ["Repository", "Tag", "ID", "Size", "Created"]
volumes = ["Name", "Driver", "Mountpoint", "Created"]
networks = ["Name", "ID", "Driver", "Scope", "Containers"]
```

## Architecture

```
┌──────────────────────────────────────────────┐
│              Theme Layer                      │
│  ┌──────────┐ ┌──────────┐                  │
│  │ operator │ │dashboard │  (swappable)     │
│  │ (k9s)   │ │(lazygit) │                  │
│  └──────────┘ └──────────┘                  │
├──────────────────────────────────────────────┤
│           App State + Event Bus              │
├──────────────────────────────────────────────┤
│           Service Layer                      │
├──────────────────────────────────────────────┤
│        Runtime Abstraction (trait)            │
│   ┌────────┐  ┌────────┐                    │
│   │ Docker │  │ Podman │                    │
│   └────────┘  └────────┘                    │
└──────────────────────────────────────────────┘
```

## Requirements

- Docker or Podman running
- Terminal with 256-color support
- macOS, Linux (Windows WSL2 untested)

## License

MIT

## Links

- Website: [dockctl.dev](https://dockctl.dev)
- Repository: [github.com/fakade2/dockctl](https://github.com/fakade2/dockctl)
