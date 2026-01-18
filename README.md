# mqtop

A high-performance MQTT explorer TUI built in Rust by Sourceful Energy. Like htop for your MQTT broker - designed to handle high-throughput IoT telemetry streams.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│ mqtop │ ● Connected │ Topics: 436 │ Msgs: 12.5k (219/s) │ 5m │
├─────────────────┬───────────────────────────────┬───────────────────────────┤
│ Topics          │ Messages                      │ Stats                     │
│                 │                               │                           │
│ ▼ sensors       │ sensors/device-001/temp       │ Connection                │
│   ▼ device-001  │                               │   Status: Connected       │
│     ▶ temp      │ {                             │   Host: mqtt:8883         │
│   ★ device-002  │   "timestamp": 1703...        │                           │
│ ▶ metrics       │   "value": 23.5,              │ Messages                  │
│ ▶ status        │   "unit": "celsius"           │   Total: 12.5k            │
│                 │ }                             │   Rate: 219.2/s           │
│                 │                               │                           │
│                 │                               │ Device Health             │
│                 │                               │   ● 12 healthy  ● 2 warn  │
│                 │                               │                           │
│                 │                               │ Tracked Metrics           │
│                 │                               │   value: 23.5             │
│                 │                               │   ▃▄▅▆▅▄▃▄▅▆▇▆▅▄▃▄▅▆▇█   │
└─────────────────┴───────────────────────────────┴───────────────────────────┘
 q:Quit /:Search f:Filter s:Star y:Copy m:Track ?:Help
```

## Features

- **Real-time MQTT streaming** - Subscribe to any topic pattern, handles 1000+ msg/sec
- **Hierarchical topic tree** - Collapsible tree view with configurable color coding
- **Device health monitoring** - Auto-tracks device activity and health status
- **Metric tracking with sparklines** - Track numeric fields over time with live graphs
- **MQTT wildcard filters** - Filter topics using `+` and `#` patterns
- **Latency monitoring** - Track message delays and jitter
- **Starred topics** - Bookmark important topics with persistence
- **Clipboard support** - Copy topics and payloads
- **JSON syntax highlighting** - Pretty-printed payload inspection
- **Vim-style navigation** - `hjkl`, arrows, and more
- **Resilient connection** - Auto-reconnect with exponential backoff

## Installation

### Quick Install (Recommended)

**macOS / Linux:**
```bash
curl -fsSL https://raw.githubusercontent.com/srcfl/mqtop/master/install.sh | bash
```

**Homebrew (macOS/Linux):**
```bash
brew tap srcfl/tap
brew install mqtop
```

### Pre-built binaries

Download from [GitHub Releases](https://github.com/srcfl/mqtop/releases):

| Platform | Download |
|----------|----------|
| macOS (Apple Silicon) | `mqtop-macos-arm64` |
| macOS (Intel) | `mqtop-macos-x64` |
| Linux (x64) | `mqtop-linux-x64` |
| Linux (ARM64/Raspberry Pi) | `mqtop-linux-arm64` |
| Windows | `mqtop-windows-x64.exe` |

```bash
# macOS/Linux: Make executable after download
chmod +x mqtop-*

# macOS: Remove quarantine (if you get a security warning)
xattr -cr ./mqtop-*

./mqtop-macos-arm64 --help
```

### Build from source

Requires Rust 1.70+:

```bash
git clone https://github.com/srcfl/mqtop.git
cd mqtop
cargo build --release
```

Binary will be at `target/release/mqtop` (3MB).

## Quick Start

### 1. Create a config file

mqtop looks for configuration in this order:
1. Path specified with `-c/--config`
2. `./config.toml` in current directory
3. `~/.config/mqtop/config.toml` (recommended for global install)

```bash
# Quick interactive setup
./mqtop --setup

# Or for global config (recommended)
mkdir -p ~/.config/mqtop
cp config.toml.example ~/.config/mqtop/config.toml
# Edit with your MQTT credentials

# Or for project-specific config
cp config.toml.example config.toml
```

### 2. Or use command-line arguments

```bash
# With token passed via CLI
./mqtop --host mqtt.example.com --port 8883 --tls --client-id mydevice --token "your-jwt-token"

# Subscribe to specific topics
./mqtop -c config.toml --topic "sensors/#"
```

### 3. Run it

```bash
./mqtop
```

## Configuration

Create `config.toml` in `~/.config/mqtop/` or your working directory:

```toml
[mqtt]
active_server = "default"

[[mqtt.servers]]
name = "default"
host = "mqtt.example.com"
port = 8883
use_tls = true
client_id = "mqtop-explorer"
token = "your-jwt-token"           # Stored in config
subscribe_topic = "#"              # Subscribe to all topics
keep_alive_secs = 30

[ui]
message_buffer_size = 100          # Messages kept per topic
stats_window_secs = 10             # Window for rate calculations
tick_rate_ms = 100                 # UI refresh rate

# Optional: Custom topic highlighting
[[ui.topic_colors]]
pattern = "sensors"
color = "cyan"

[[ui.topic_colors]]
pattern = "alerts"
color = "red"

# Optional: Count topics by category in Stats panel
[[ui.topic_categories]]
label = "Sensors"
pattern = "sensors"
color = "cyan"

[[ui.topic_categories]]
label = "Alerts"
pattern = "alerts"
color = "red"
```

### CLI Options

```
Usage: mqtop [OPTIONS]

Options:
  -c, --config <FILE>      Config file path [default: ~/.config/mqtop/config.toml]
      --host <HOST>        MQTT broker host (overrides config)
      --port <PORT>        MQTT broker port (overrides config)
      --client-id <ID>     Client ID (overrides config)
  -u, --username <USER>    Username (defaults to client_id)
      --token <TOKEN>      Token (overrides config)
  -t, --topic <TOPIC>      Subscribe topic (overrides config)
      --tls                Enable TLS
      --setup              Run interactive config wizard
      --list-backups       List config backups
      --rollback <INDEX>   Restore config from backup (1 = newest)
  -d, --debug              Enable debug logging to mqtop.log
  -h, --help               Print help
  -V, --version            Print version
```

## Usage Guide

### Navigation

| Key | Action |
|-----|--------|
| `Tab` | Switch between panels (Topics → Messages → Stats) |
| `1` `2` `3` | Jump to panel directly |
| `↑` `↓` or `j` `k` | Move up/down |
| `←` `→` or `h` `l` | Collapse/expand or move into child |
| `H` `L` | Collapse/expand full branch |
| `Enter` | Toggle expand/collapse |
| `PgUp` `PgDn` | Page up/down |
| `g` `G` | Go to top/bottom |

On smaller terminals, mqtop automatically switches to 2-panel or 1-panel layouts.

### Search & Filter

| Key | Action |
|-----|--------|
| `/` | Open fuzzy search |
| `PgUp` `PgDn` | Scroll search results |
| `f` | Set topic filter (MQTT wildcards) |
| `F` | Clear topic filter |
| `*` | Toggle starred-only view |
| `Esc` | Cancel/close overlay |
| `S` | Open server manager |

**Filter examples:**
- `sensors/#` - All sensor topics
- `sensors/+/temperature` - Temperature from any device
- `building/+/floor/#` - All topics under any floor

### Topic Management

| Key | Action |
|-----|--------|
| `s` | Star/unstar current topic |
| `y` | Copy topic path to clipboard |
| `Y` | Copy payload to clipboard |

### Server Manager

| Key | Action |
|-----|--------|
| `S` | Open server manager |
| `Enter` | Edit server |
| `a` | Add server |
| `d` | Delete server |
| `Space` | Activate server |
| `w` | Save config |
| `Esc` | Close manager |

### Server Edit

| Key | Action |
|-----|--------|
| `Tab` `Shift+Tab` | Next/previous field |
| `←` `→` `Home` `End` | Move cursor in field |
| `Backspace` `Del` | Delete characters |
| `Space` | Toggle TLS |
| `Enter` | Save changes |
| `Esc` | Cancel edit |

### Metrics & Display

| Key | Action |
|-----|--------|
| `m` | Track metric from current message (opens field selector) |
| `p` | Cycle payload mode (Auto → Raw → Hex → JSON) |
| `c` | Clear statistics |

### General

| Key | Action |
|-----|--------|
| `?` | Toggle help overlay |
| `q` or `Ctrl+C` | Quit |

## Panels

### Topic Tree (Left)

Displays all discovered topics in a hierarchical tree. Topic colors are configurable via `[[ui.topic_colors]]` in your config file. UUIDs and IDs are automatically shown in gray.

A `★` indicator shows starred topics.

### Messages (Center)

Shows messages for the selected topic with:
- Timestamp
- JSON syntax highlighting (keys, strings, numbers, booleans)
- Multiple display modes (Auto, Raw, Hex, JSON)

### Stats (Right)

Real-time statistics including:
- Connection status
- Message count and rate
- Data throughput
- **Latency metrics** - Message delays, inter-arrival times, jitter
- **Topic categories** - Configurable counters via `[[ui.topic_categories]]`
- Device health summary
- Tracked metrics with sparklines

## Data Persistence

mqtop stores data in `~/.config/mqtop/`:
- `config.toml` - Configuration file (optional, can also use `./config.toml`)
- `backups/` - Rolling config backups (last 5)
- `userdata.json` - Starred topics and tracked metrics (auto-saved)

## Troubleshooting

### Connection issues

Enable debug logging:
```bash
./mqtop --debug
tail -f mqtop.log
```

### Large payloads

mqtop supports payloads up to 1MB. If you see truncated messages, check the source.

### High CPU usage

Reduce tick rate in config:
```toml
[ui]
tick_rate_ms = 200  # Default is 100
```

## Development

```bash
# Run tests
cargo test

# Run with debug output
cargo run -- --debug

# Build release
cargo build --release
```

## Author

Created by **Fredrik Ahlgren** ([@frahlg](https://github.com/frahlg) on GitHub, [@frahlg](https://x.com/frahlg) on X)
CEO of [Sourceful Labs AB](https://sourceful.energy) (Sourceful Energy)

Co-authored with [Claude Code](https://claude.com/claude-code).

## License

MIT License - Sourceful Energy 2024

## Contributing

1. Fork the repository
2. Create a feature branch
3. Submit a pull request

For bugs and feature requests, open an issue on GitHub.
