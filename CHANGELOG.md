# Changelog

All notable changes to mqtop will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

#### Publish Bookmarks - Quick One-Click Publishing Presets

A major new feature for power users who need to repeatedly publish test messages during debugging and development sessions.

**What are Bookmarks?**

Bookmarks are saved publish presets that store everything needed to quickly fire off an MQTT message:
- Topic
- Payload (supports multi-line content)
- QoS level (0, 1, or 2)
- Retain flag
- Optional category for organization

**Why Bookmarks?**

When debugging IoT systems, you often need to:
- Send the same command to a device repeatedly
- Trigger specific alerts or states
- Test edge cases with specific payloads
- Rapidly fire test messages while monitoring responses

Previously, you'd have to re-enter the topic and payload each time. Now you can save presets and publish with a single keypress.

**Key Features:**

1. **Quick Publish (`Enter`)** - Select a bookmark and press Enter to publish immediately. The bookmark manager stays open so you can rapid-fire multiple messages.

2. **Category Grouping** - Organize bookmarks into categories like "testing", "alerts", "commands", etc. Bookmarks are visually grouped by category in the manager.

3. **Save from Publish Dialog (`Ctrl+S`)** - While in the publish dialog, press Ctrl+S to save your current topic/payload/settings as a new bookmark.

4. **Full Edit Support** - Add, edit, and delete bookmarks with a familiar form interface. Tab between fields, use arrow keys to navigate.

5. **Persistence** - Bookmarks are automatically saved to `~/.config/mqtop/userdata.json` and survive application restarts.

**New Keybindings:**

| Key | Context | Action |
|-----|---------|--------|
| `B` | Normal mode | Open Bookmark Manager |
| `Enter` | Bookmark list | Quick publish selected bookmark |
| `a` | Bookmark list | Add new bookmark |
| `e` | Bookmark list | Edit selected bookmark |
| `d` | Bookmark list | Delete selected bookmark |
| `Ctrl+S` | Publish dialog | Save current settings as bookmark |
| `Tab/Shift+Tab` | Bookmark edit | Navigate between fields |
| `↑/↓` or `j/k` | Bookmark list | Navigate bookmarks |
| `Esc` | Bookmark manager | Close |

**UI Previews:**

Bookmark Manager (list view):
```
┌─────────────────── Bookmarks ───────────────────┐
│                                                 │
│  [testing]                                      │
│  ► Temp sensor alert    sensors/temp/alert      │
│    Device offline       devices/+/status        │
│                                                 │
│  [alerts]                                       │
│    Battery low          alerts/battery/low      │
│                                                 │
│  [uncategorized]                                │
│    Test message         test/topic              │
│                                                 │
│  Enter:Publish  e:Edit  a:Add  d:Delete  Esc   │
└─────────────────────────────────────────────────┘
```

Bookmark Edit Dialog:
```
┌─────────────── Edit Bookmark ───────────────────┐
│                                                 │
│  Name:     [Temp sensor alert____________]      │
│  Category: [testing______________________]      │
│  Topic:    [sensors/temp/alert___________]      │
│                                                 │
│  Payload:                                       │
│  ┌─────────────────────────────────────────┐   │
│  │ {"temp": 85, "alert": "high"}           │   │
│  └─────────────────────────────────────────┘   │
│                                                 │
│  QoS: [1]    Retain: [off]                     │
│                                                 │
│  Enter: Save | Esc: Cancel                     │
└─────────────────────────────────────────────────┘
```

**Example Workflows:**

*Workflow 1: Create bookmark from scratch*
1. Press `B` to open bookmark manager
2. Press `a` to add new bookmark
3. Fill in name, category, topic, payload, QoS, retain
4. Press `Enter` to save

*Workflow 2: Save from publish dialog*
1. Press `P` to open publish dialog
2. Enter your topic and payload
3. Set QoS and retain as needed
4. Press `Ctrl+S` to save as bookmark
5. Edit the name (auto-generated from topic) and add a category
6. Press `Enter` to save

*Workflow 3: Rapid-fire testing*
1. Press `B` to open bookmark manager
2. Use `j/k` or arrows to select a preset
3. Press `Enter` to publish
4. Observe the result in the messages panel
5. Select another preset and repeat
6. Press `Esc` when done

### Changed

- Updated help overlay (`?`) to include bookmark keybindings
- Footer hints now show `B:Bookmarks` in normal mode
- Publish dialog footer now shows `Ctrl+S:Save Bookmark`

### Technical Details

**New Files:**
- `src/ui/bookmarks.rs` - Bookmark manager UI (list view + edit dialog)

**Modified Files:**
- `src/persistence.rs` - Added `Bookmark` struct and `UserData` methods
- `src/app.rs` - Added `InputMode::BookmarkManager`, state management, handlers
- `src/ui/mod.rs` - Integrated bookmark manager rendering
- `src/ui/help.rs` - Added bookmark keybindings to help text
- `README.md` - Comprehensive documentation for publishing and bookmarks

**Data Structure:**
```rust
pub struct Bookmark {
    pub name: String,           // Display name
    pub topic: String,          // MQTT topic
    pub payload: String,        // Default payload
    pub qos: u8,                // 0, 1, 2
    pub retain: bool,
    pub category: Option<String>, // Optional grouping
}
```

---

## [0.2.3] - 2025-01-XX

### Fixed
- Fixed clippy warning in server deletion

### Changed
- Improved CI compatibility for Cargo.lock

## [0.2.2] - 2025-01-XX

### Added
- Multi-server configuration UI with safer navigation
- Server manager (`S` key) for managing multiple MQTT connections

### Fixed
- Code formatting improvements
- Config path on macOS now correctly uses `~/.config`

## [0.2.1] - 2025-01-XX

### Added
- MQTT message publishing (`P` key)
- Copy message to publish dialog (`Ctrl+P`)
- Publish dialog with topic, payload, QoS, and retain options

## [0.2.0] - 2024-XX-XX

### Added
- Metric tracking with sparklines
- Device health monitoring
- Latency tracking
- Schema change detection
- Starred topics with persistence
- Topic filtering with MQTT wildcards
- Fuzzy search
- Vim-style navigation
- JSON syntax highlighting
- Auto-reconnect with exponential backoff

## [0.1.0] - 2024-XX-XX

### Added
- Initial release
- Real-time MQTT streaming
- Hierarchical topic tree
- Message buffer
- Basic statistics
- TLS support
- Configuration file support

---

[Unreleased]: https://github.com/srcfl/mqtop/compare/v0.2.3...HEAD
[0.2.3]: https://github.com/srcfl/mqtop/compare/v0.2.2...v0.2.3
[0.2.2]: https://github.com/srcfl/mqtop/compare/v0.2.1...v0.2.2
[0.2.1]: https://github.com/srcfl/mqtop/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/srcfl/mqtop/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/srcfl/mqtop/releases/tag/v0.1.0
