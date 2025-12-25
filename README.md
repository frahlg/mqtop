# Sourceful Energy DataFeeder

A high-performance MQTT explorer and debug tool built in Rust for the Sourceful Energy platform. Designed to handle high-throughput IoT telemetry streams with a responsive terminal UI.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│ Sourceful DataFeeder │ ● Connected │ Topics: 436 │ Msgs: 12.5k (219/s) │ 5m │
├─────────────────┬───────────────────────────────┬───────────────────────────┤
│ Topics          │ Messages                      │ Stats                     │
│                 │                               │                           │
│ ▼ telemetry     │ telemetry/zap-00.../meter/... │ Connection                │
│   ▼ zap-0000d8..│                               │   Status: Connected       │
│     ▶ meter     │ {                             │   Host: mqtt:8883         │
│   ★ zap-0000e2..│   "timestamp": 1703...        │                           │
│ ▶ sites         │   "L1_W": 1523.5,             │ Messages                  │
│ ▶ ems           │   "L2_W": 892.1,              │   Total: 12.5k            │
│                 │   "total_import_Wh": 48291    │   Rate: 219.2/s           │
│                 │ }                             │                           │
│                 │                               │ Device Health             │
│                 │                               │   ● 12 healthy  ● 2 warn  │
│                 │                               │                           │
│                 │                               │ Tracked Metrics           │
│                 │                               │   L1_W: 1523              │
│                 │                               │   ▃▄▅▆▅▄▃▄▅▆▇▆▅▄▃▄▅▆▇█   │
└─────────────────┴───────────────────────────────┴───────────────────────────┘
 q:Quit /:Search f:Filter s:Star y:Copy m:Track ?:Help
```

## Features

- **Real-time MQTT streaming** - Subscribe to any topic pattern, handles 1000+ msg/sec
- **Hierarchical topic tree** - Collapsible tree view with Sourceful entity color coding
- **Device health monitoring** - Auto-tracks devices from telemetry topics
- **Metric tracking with sparklines** - Track numeric fields over time with live graphs
- **MQTT wildcard filters** - Filter topics using `+` and `#` patterns
- **Latency monitoring** - Track message delays and jitter
- **Starred topics** - Bookmark important topics with persistence
- **Clipboard support** - Copy topics and payloads
- **JSON syntax highlighting** - Pretty-printed payload inspection
- **Vim-style navigation** - `hjkl`, arrows, and more
- **Resilient connection** - Auto-reconnect with exponential backoff

## Installation

### Build from source

Requires Rust 1.70+:

```bash
git clone https://github.com/sourceful-energy/datafeeder.git
cd datafeeder
cargo build --release
```

Binary will be at `target/release/datafeeder` (3MB).

### Pre-built binaries

Download from [GitHub Releases](https://github.com/srcfl/datafeeder/releases):

| Platform | Download |
|----------|----------|
| macOS (Apple Silicon) | `datafeeder-macos-arm64` |
| macOS (Intel) | `datafeeder-macos-x64` |
| Linux (x64) | `datafeeder-linux-x64` |
| Linux (ARM64/Raspberry Pi) | `datafeeder-linux-arm64` |
| Windows | `datafeeder-windows-x64.exe` |

```bash
# macOS/Linux: Make executable after download
chmod +x datafeeder-*
./datafeeder-macos-arm64 --help
```

## Quick Start

### 1. Create a config file

```bash
cp config.toml.example config.toml
# Edit with your MQTT credentials
```

### 2. Or use command-line arguments

```bash
# With token in environment
export MQTT_TOKEN="your-jwt-token"
./datafeeder --host mqtt.sourceful.energy --port 8883 --tls --client-id mydevice

# Subscribe to specific topics
./datafeeder -c config.toml --topic "telemetry/#"
```

### 3. Run it

```bash
./datafeeder
```

## Configuration

Create `config.toml` in your working directory:

```toml
[mqtt]
host = "mqtt.sourceful.energy"
port = 8883
use_tls = true
client_id = "datafeeder-explorer"
token = "your-jwt-token"           # Or set MQTT_TOKEN env var
subscribe_topic = "#"              # Subscribe to all topics

[ui]
message_buffer_size = 100          # Messages kept per topic
stats_window_secs = 10             # Window for rate calculations
tick_rate_ms = 100                 # UI refresh rate
```

### Environment Variables

| Variable | Description |
|----------|-------------|
| `MQTT_TOKEN` | JWT token for authentication (overrides config) |

### CLI Options

```
Usage: datafeeder [OPTIONS]

Options:
  -c, --config <FILE>      Config file path [default: config.toml]
      --host <HOST>        MQTT broker host (overrides config)
      --port <PORT>        MQTT broker port (overrides config)
      --client-id <ID>     Client ID (overrides config)
  -u, --username <USER>    Username (defaults to client_id)
  -t, --topic <TOPIC>      Subscribe topic (overrides config)
      --tls                Enable TLS
  -d, --debug              Enable debug logging to datafeeder.log
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
| `←` `→` or `h` `l` | Collapse/expand or move to parent |
| `Enter` | Toggle expand/collapse |
| `PgUp` `PgDn` | Page up/down |
| `g` `G` | Go to top/bottom |

### Search & Filter

| Key | Action |
|-----|--------|
| `/` | Open fuzzy search |
| `f` | Set topic filter (MQTT wildcards) |
| `F` | Clear topic filter |
| `*` | Toggle starred-only view |
| `Esc` | Cancel/close overlay |

**Filter examples:**
- `telemetry/#` - All telemetry topics
- `telemetry/+/meter` - Any device's meter data
- `sites/+/devices/#` - All devices under any site

### Topic Management

| Key | Action |
|-----|--------|
| `s` | Star/unstar current topic |
| `y` | Copy topic path to clipboard |
| `Y` | Copy payload to clipboard |

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

Displays all discovered topics in a hierarchical tree. Topics are color-coded:

- **Red** - Wallets
- **Cyan** - Sites
- **Green** - Devices
- **Magenta** - Telemetry
- **Blue** - EMS (Energy Management System)
- **Gray** - IDs and UUIDs

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
- Topic counts by category (Sourceful entities)
- Device health summary
- Tracked metrics with sparklines

## Sourceful Data Model

DataFeeder understands the Sourceful Energy hierarchy:

```
Wallet → Site → Device → DER (Distributed Energy Resource)
```

**Topic patterns:**
- `telemetry/{device_id}/meter/{type}/json` - Device telemetry
- `sites/{site_id}/...` - Site data
- `ems/...` - Energy management system

## Data Persistence

User preferences are saved to `~/.config/datafeeder/userdata.json`:
- Starred topics
- Tracked metrics

## Troubleshooting

### Connection issues

Enable debug logging:
```bash
./datafeeder --debug
tail -f datafeeder.log
```

### Large payloads

DataFeeder supports payloads up to 1MB. If you see truncated messages, check the source.

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

## License

MIT License - Sourceful Energy 2024

## Contributing

1. Fork the repository
2. Create a feature branch
3. Submit a pull request

For bugs and feature requests, open an issue on GitHub.
