# mqtop

*"In the beginning there was the Message, and the Message was with the Broker, and the Message was... well, probably JSON."*

A high-performance MQTT explorer TUI built in Rust. Like htop for your MQTT broker, except it won't judge you for subscribing to `#` in production.*

**It handles 1000+ messages per second without breaking a sweat, much like Death handles souls - efficiently and without unnecessary drama.*

```
┌─────────────────────────────────────────────────────────────────────────────┐
│ mqtop │ ● Connected │ Topics: 436 │ Msgs: 12.5k (219/s) │ 5m              │
├─────────────────┬───────────────────────────────────────┬───────────────────┤
│ Topics          │ Messages                              │ Stats             │
│                 │                                       │                   │
│ ▼ sensors       │ sensors/device-001/temp               │ Connection        │
│   ▼ device-001  │                                       │   Status: ●       │
│     ▶ temp      │ {                                     │   Host: mqtt:8883 │
│   ★ device-002  │   "timestamp": 1703...                │                   │
│ ▶ metrics       │   "value": 23.5,                      │ Messages          │
│ ▶ status        │   "unit": "celsius"                   │   Total: 12.5k    │
│                 │ }                                     │   Rate: 219.2/s   │
│                 │                                       │                   │
│                 │                                       │ Device Health     │
│                 │                                       │   ● 12 healthy    │
│                 │                                       │   ● 2 warning     │
└─────────────────┴───────────────────────────────────────┴───────────────────┘
 q:Quit /:Search f:Filter s:Star y:Copy m:Track ?:Help
```

<sub>* Though we make no guarantees about what the Librarian would say if you tried subscribing to `ook/#`.</sub>

---

## Features

The universe, as has been observed, operates on certain immutable rules. So does mqtop:

- **Real-time MQTT streaming** - Messages arrive faster than you can read them, much like footnotes in a Discworld novel
- **Hierarchical topic tree** - Collapsible, expandable, and infinitely more organized than L-space
- **Device health monitoring** - Knows when your devices are healthy, warning, or have shuffled off this mortal coil
- **Metric tracking with sparklines** - Little graphs that go up and down, creating the illusion of understanding
- **MQTT wildcard filters** - `+` and `#` patterns, because sometimes you need to catch everything
- **Latency monitoring** - Track message delays with the precision of a well-oiled mechanism
- **Starred topics** - Bookmark the important ones, forget the rest
- **Publish bookmarks** - Save your favorite messages for rapid-fire testing
- **MQTT publishing** - Send messages directly, no external tools required
- **Clipboard support** - Copy topics and payloads to share the joy
- **JSON syntax highlighting** - Pretty colors for pretty data
- **Vim-style navigation** - `hjkl` for those who have Seen The Light
- **Resilient connection** - Auto-reconnect with exponential backoff, because hope springs eternal

---

## Installation

### The Easy Way

**macOS / Linux:**
```bash
curl -fsSL https://raw.githubusercontent.com/frahlg/mqtop/master/install.sh | bash
```

**Homebrew (macOS/Linux):**
```bash
brew tap frahlg/tap
brew install mqtop
```

### Pre-built Binaries

For those who prefer their software pre-compiled, download from [GitHub Releases](https://github.com/frahlg/mqtop/releases):

| Platform | Binary | Notes |
|----------|--------|-------|
| macOS (Apple Silicon) | `mqtop-macos-arm64` | For the M1/M2/M3 enlightened |
| macOS (Intel) | `mqtop-macos-x64` | For the x86 nostalgic |
| Linux (x64) | `mqtop-linux-x64` | The penguin approves |
| Linux (ARM64) | `mqtop-linux-arm64` | Raspberry Pi and friends |
| Windows | `mqtop-windows-x64.exe` | It works here too |

```bash
# Make it executable (macOS/Linux)
chmod +x mqtop-*

# macOS: Remove the quarantine attribute
xattr -cr ./mqtop-*

# Run it
./mqtop --help
```

### Build From Source

For those who prefer to compile their own reality:

```bash
git clone https://github.com/frahlg/mqtop.git
cd mqtop
cargo build --release
```

Binary materializes at `target/release/mqtop` (~3MB).

---

## Quick Start

1. **Run mqtop:**
   ```bash
   mqtop
   ```

2. **The Server Manager opens automatically.** Press `a` to add a new server.

3. **Fill in your broker details:**
   - Name: `my-broker`
   - Host: `mqtt.example.com`
   - Port: `8883` (or `1883` for non-TLS)
   - Enable TLS if needed
   - Set your client ID, username, password, or token

4. **Press `Enter` to save**, then select your server and press `Enter` again to connect.

5. **Watch the messages flow.** Navigate with arrow keys or `hjkl`, expand topics with `Enter` or `→`.

That's it. No config files required (though you can use them if you're that kind of person).

---

## Usage Guide

### Navigation

| Key | What It Does |
|-----|--------------|
| `Tab` | Cycle panels (Topics → Messages → Stats) |
| `1` `2` `3` | Jump directly to panel |
| `↑` `↓` or `j` `k` | Move up/down |
| `←` `→` or `h` `l` | Collapse/expand or dive deeper |
| `Enter` | Toggle expand/collapse |
| `g` / `G` | Top / Bottom |
| `PgUp` `PgDn` | Page navigation |

### Search & Filter

| Key | What It Does |
|-----|--------------|
| `/` | Fuzzy search |
| `f` | Set topic filter (MQTT wildcards) |
| `F` | Clear filter |
| `*` | Show only starred topics |

**Filter examples:**
- `sensors/#` - All sensor topics
- `sensors/+/temperature` - Temperature from any device
- `ook/#` - The Librarian's private topics*

<sub>* Not recommended unless you want to be hit with a very large dictionary.</sub>

### Server Manager

| Key | What It Does |
|-----|--------------|
| `S` | Open server manager |
| `Enter` | Activate selected server |
| `e` | Edit server configuration |
| `a` | Add new server |
| `d` | Delete server |
| `Esc` | Close |

### Publishing

| Key | What It Does |
|-----|--------------|
| `P` | Open publish dialog |
| `Ctrl+P` | Copy current message to publish |
| `B` | Open bookmark manager |
| `Ctrl+S` | Save publish as bookmark |

### General

| Key | What It Does |
|-----|--------------|
| `s` | Star/unstar topic |
| `y` | Copy topic to clipboard |
| `Y` | Copy payload to clipboard |
| `m` | Track metric from message |
| `p` | Cycle payload mode (Auto → Raw → Hex → JSON) |
| `c` | Clear statistics |
| `?` | Help overlay |
| `q` | Quit |

---

## Configuration (Optional)

While mqtop works fine without config files, you can create one at `~/.config/mqtop/config.toml` for persistent settings:

```toml
[mqtt]
active_server = "production"

[[mqtt.servers]]
name = "production"
host = "mqtt.example.com"
port = 8883
use_tls = true
client_id = "mqtop-prod"
subscribe_topic = "#"
keep_alive_secs = 30

[ui]
message_buffer_size = 100    # Messages per topic
stats_window_secs = 10       # Rate calculation window
tick_rate_ms = 100           # UI refresh rate

# Topic highlighting
[[ui.topic_colors]]
pattern = "sensors"
color = "cyan"

[[ui.topic_colors]]
pattern = "alerts"
color = "red"
```

Servers added via the UI are automatically saved to the config file.

---

## Data Persistence

mqtop stores data in `~/.config/mqtop/`:

- `config.toml` - Configuration and servers
- `backups/` - Rolling config backups (last 5)
- `userdata.json` - Starred topics, metrics, bookmarks

---

## Troubleshooting

### Connection Issues

```bash
mqtop --debug
tail -f mqtop.log
```

The log file knows all.

### High CPU Usage

```toml
[ui]
tick_rate_ms = 200  # Slower refresh
```

---

## Development

```bash
cargo test              # Run tests
cargo run -- --debug    # Debug mode
cargo build --release   # Production build
```

---

## Author

Created by **Fredrik Ahlgren** ([@frahlg](https://github.com/frahlg) on GitHub, [@frahlg](https://x.com/frahlg) on X)

Co-authored with [Claude Code](https://claude.ai/code).

---

## License

MIT License - See [LICENSE](LICENSE) file.

---

## Contributing

1. Fork it
2. Create a feature branch
3. Make your changes
4. Submit a pull request

For bugs and features, open an issue.

---

*GNU Terry Pratchett*
