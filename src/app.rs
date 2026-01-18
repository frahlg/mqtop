#![allow(dead_code)]

use std::collections::HashSet;
use std::path::PathBuf;

use anyhow::{anyhow, Context, Result};
use crossterm::event::{KeyCode, KeyModifiers};

use crate::config::{Config, MqttServerConfig, CONFIG_BACKUP_LIMIT};
use crate::mqtt::{ConnectionState, MqttEvent, MqttMessage};
use crate::persistence::UserData;
use crate::state::metric_tracker::topic_matches;
use crate::state::{
    get_numeric_fields, DeviceTracker, LatencyTracker, MessageBuffer, MetricTracker, SchemaTracker,
    Stats, TopicInfo, TopicTree,
};

/// Current UI panel focus
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Panel {
    TopicTree,
    Messages,
    Stats,
}

/// Input mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    Search,
    MetricSelect,
    Filter,
    ServerManager,
}

/// Filter mode for topic tree
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterMode {
    All,
    Starred,
}

/// Application state
pub struct App {
    /// Configuration
    pub config: Config,
    /// Config path
    pub config_path: PathBuf,
    /// User data (persisted)
    pub user_data: UserData,
    /// Topic tree
    pub topic_tree: TopicTree,
    /// Message buffer
    pub message_buffer: MessageBuffer,
    /// Statistics
    pub stats: Stats,
    /// Currently selected topic in tree
    pub selected_topic_index: usize,
    /// Currently selected message index
    pub selected_message_index: usize,
    /// Expanded topics in tree
    pub expanded_topics: HashSet<String>,
    /// Current panel focus
    pub focused_panel: Panel,
    /// Input mode
    pub input_mode: InputMode,
    /// Filter mode
    pub filter_mode: FilterMode,
    /// Search query
    pub search_query: String,
    /// Search results
    pub search_results: Vec<String>,
    /// Selected search result index
    pub search_result_index: usize,
    /// Search results scroll offset
    pub search_scroll: usize,
    /// Connection state
    pub connection_state: ConnectionState,
    /// Last error message
    pub last_error: Option<String>,
    /// Whether app should quit
    pub should_quit: bool,
    /// Scroll offset for topic tree
    pub tree_scroll: usize,
    /// Scroll offset for messages
    pub message_scroll: usize,
    /// Scroll offset for stats panel
    pub stats_scroll: usize,
    /// Currently selected topic (full path)
    pub selected_topic: Option<String>,
    /// Show help overlay
    pub show_help: bool,
    /// Payload display mode
    pub payload_mode: PayloadMode,
    /// Status message (temporary)
    pub status_message: Option<(String, std::time::Instant)>,
    /// Metric tracker
    pub metric_tracker: MetricTracker,
    /// Device health tracker
    pub device_tracker: DeviceTracker,
    /// Latency tracker
    pub latency_tracker: LatencyTracker,
    /// Schema change tracker
    pub schema_tracker: SchemaTracker,
    /// Available numeric fields for metric selection
    pub available_fields: Vec<(String, f64)>,
    /// Selected field index in metric selection mode
    pub metric_select_index: usize,
    /// Topic filter pattern (MQTT wildcard syntax)
    pub topic_filter: Option<String>,
    /// Filter input buffer
    pub filter_input: String,
    /// Pending server switch selection
    pub pending_server_switch: Option<usize>,
    /// Server manager selection index
    pub server_manager_index: usize,
    /// Server edit buffer
    pub server_edit: ServerEditState,
}

#[derive(Debug, Clone)]
pub struct ServerEditState {
    pub active: bool,
    pub is_new: bool,
    pub index: usize,
    pub field: ServerField,
    pub cursor: usize,
    pub name: String,
    pub host: String,
    pub port: String,
    pub use_tls: bool,
    pub client_id: String,
    pub username: String,
    pub token: String,
    pub subscribe_topic: String,
    pub keep_alive_secs: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServerField {
    Name,
    Host,
    Port,
    UseTls,
    ClientId,
    Username,
    Token,
    SubscribeTopic,
    KeepAlive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PayloadMode {
    Auto, // Auto-detect JSON vs raw
    Raw,  // Raw string
    Hex,  // Hex dump
    Json, // Force JSON pretty-print
}

impl Default for ServerEditState {
    fn default() -> Self {
        Self {
            active: false,
            is_new: false,
            index: 0,
            field: ServerField::Name,
            cursor: 0,
            name: String::new(),
            host: String::new(),
            port: String::new(),
            use_tls: false,
            client_id: String::new(),
            username: String::new(),
            token: String::new(),
            subscribe_topic: String::new(),
            keep_alive_secs: String::new(),
        }
    }
}

impl ServerField {
    pub const ALL: [ServerField; 9] = [
        ServerField::Name,
        ServerField::Host,
        ServerField::Port,
        ServerField::UseTls,
        ServerField::ClientId,
        ServerField::Username,
        ServerField::Token,
        ServerField::SubscribeTopic,
        ServerField::KeepAlive,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            ServerField::Name => "Name",
            ServerField::Host => "Host",
            ServerField::Port => "Port",
            ServerField::UseTls => "TLS",
            ServerField::ClientId => "Client ID",
            ServerField::Username => "Username",
            ServerField::Token => "Token",
            ServerField::SubscribeTopic => "Subscribe",
            ServerField::KeepAlive => "Keep Alive",
        }
    }
}

impl App {
    pub fn new(config: Config, config_path: PathBuf) -> Self {
        let message_buffer_size = config.ui.message_buffer_size;
        let stats_window = config.ui.stats_window_secs;
        let user_data = UserData::load();

        Self {
            config,
            config_path,
            user_data,
            topic_tree: TopicTree::new(),
            message_buffer: MessageBuffer::new(message_buffer_size),
            stats: Stats::new(stats_window),
            selected_topic_index: 0,
            selected_message_index: 0,
            expanded_topics: HashSet::new(),
            focused_panel: Panel::TopicTree,
            input_mode: InputMode::Normal,
            filter_mode: FilterMode::All,
            search_query: String::new(),
            search_results: Vec::new(),
            search_result_index: 0,
            search_scroll: 0,
            connection_state: ConnectionState::Disconnected,
            last_error: None,
            should_quit: false,
            tree_scroll: 0,
            message_scroll: 0,
            stats_scroll: 0,
            selected_topic: None,
            show_help: false,
            payload_mode: PayloadMode::Auto,
            status_message: None,
            metric_tracker: MetricTracker::new(100), // Keep last 100 data points
            device_tracker: DeviceTracker::new(),
            latency_tracker: LatencyTracker::new(100),
            schema_tracker: SchemaTracker::new(),
            available_fields: Vec::new(),
            metric_select_index: 0,
            topic_filter: None,
            filter_input: String::new(),
            pending_server_switch: None,
            server_manager_index: 0,
            server_edit: ServerEditState::default(),
        }
    }

    /// Save user data to disk
    pub fn save_user_data(&self) {
        if let Err(e) = self.user_data.save() {
            tracing::error!("Failed to save user data: {:?}", e);
        }
    }

    /// Set a temporary status message
    pub fn set_status(&mut self, msg: &str) {
        self.status_message = Some((msg.to_string(), std::time::Instant::now()));
    }

    /// Get status message if not expired (3 seconds)
    pub fn get_status(&self) -> Option<&str> {
        self.status_message.as_ref().and_then(|(msg, time)| {
            if time.elapsed().as_secs() < 3 {
                Some(msg.as_str())
            } else {
                None
            }
        })
    }

    /// Toggle star for currently selected topic
    pub fn toggle_star(&mut self) {
        if let Some(topic) = &self.selected_topic.clone() {
            let starred = self.user_data.toggle_star(topic);
            self.set_status(if starred {
                "★ Starred"
            } else {
                "☆ Unstarred"
            });
            self.save_user_data();
        }
    }

    /// Check if a topic is starred
    pub fn is_starred(&self, topic: &str) -> bool {
        self.user_data.is_starred(topic)
    }

    /// Toggle filter mode
    pub fn toggle_filter_mode(&mut self) {
        self.filter_mode = match self.filter_mode {
            FilterMode::All => FilterMode::Starred,
            FilterMode::Starred => FilterMode::All,
        };
        self.selected_topic_index = 0;
        self.set_status(match self.filter_mode {
            FilterMode::All => "Showing all topics",
            FilterMode::Starred => "Showing starred only",
        });
    }

    /// Process an MQTT event
    pub fn handle_mqtt_event(&mut self, event: MqttEvent) {
        match event {
            MqttEvent::Message(msg) => {
                self.stats.record_message(msg.payload_size());
                self.topic_tree.insert(&msg.topic, msg.payload_size());
                // Process for metric tracking
                self.metric_tracker
                    .process_message(&msg.topic, &msg.payload);
                // Process for device health tracking
                self.device_tracker
                    .process_message(&msg.topic, msg.payload_size());
                // Process for latency tracking
                self.latency_tracker.record_message(&msg.payload);
                // Process for schema tracking (silent - no notifications)
                let _ = self
                    .schema_tracker
                    .process_message(&msg.topic, &msg.payload);
                self.message_buffer.push(msg);
            }
            MqttEvent::StateChange(state) => {
                self.connection_state = state;
                if state == ConnectionState::Connected {
                    self.last_error = None;
                }
            }
            MqttEvent::Error(err) => {
                self.last_error = Some(err);
            }
        }
    }

    /// Handle keyboard input
    pub fn handle_key(&mut self, code: KeyCode, modifiers: KeyModifiers) {
        match self.input_mode {
            InputMode::Search => self.handle_search_input(code, modifiers),
            InputMode::Normal => self.handle_normal_input(code, modifiers),
            InputMode::MetricSelect => self.handle_metric_select_input(code, modifiers),
            InputMode::Filter => self.handle_filter_input(code, modifiers),
            InputMode::ServerManager => self.handle_server_manager_input(code, modifiers),
        }
    }

    fn handle_metric_select_input(&mut self, code: KeyCode, _modifiers: KeyModifiers) {
        match code {
            KeyCode::Esc => {
                self.input_mode = InputMode::Normal;
                self.available_fields.clear();
            }
            KeyCode::Enter => {
                if let Some((field, _)) = self.available_fields.get(self.metric_select_index) {
                    if let Some(topic) = &self.selected_topic {
                        // Create a wildcard pattern to match similar topics
                        // e.g., telemetry/device123/meter/zap/json -> telemetry/+/meter/+/json
                        let pattern = create_wildcard_pattern(topic);
                        let label = format!("{} ({})", field, short_topic(topic));
                        self.metric_tracker
                            .track(label.clone(), pattern, field.clone());
                        self.set_status(&format!("Tracking: {}", field));
                    }
                }
                self.input_mode = InputMode::Normal;
                self.available_fields.clear();
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if !self.available_fields.is_empty() {
                    self.metric_select_index =
                        (self.metric_select_index + 1) % self.available_fields.len();
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if !self.available_fields.is_empty() {
                    self.metric_select_index = self
                        .metric_select_index
                        .checked_sub(1)
                        .unwrap_or(self.available_fields.len() - 1);
                }
            }
            _ => {}
        }
    }

    /// Enter metric selection mode
    pub fn enter_metric_select(&mut self) {
        // Get the current message's JSON fields
        let messages = self.get_current_messages();
        if let Some(msg) = messages.first() {
            if let Ok(json) = serde_json::from_slice::<serde_json::Value>(&msg.payload) {
                self.available_fields = get_numeric_fields(&json);
                if !self.available_fields.is_empty() {
                    self.input_mode = InputMode::MetricSelect;
                    self.metric_select_index = 0;
                } else {
                    self.set_status("No numeric fields found");
                }
            } else {
                self.set_status("Payload is not JSON");
            }
        } else {
            self.set_status("No message selected");
        }
    }

    /// Remove a tracked metric
    pub fn remove_metric(&mut self, label: &str) {
        self.metric_tracker.untrack(label);
        self.set_status(&format!("Stopped tracking: {}", label));
    }

    /// Copy current topic to clipboard
    pub fn copy_topic(&mut self) {
        if let Some(topic) = &self.selected_topic {
            match arboard::Clipboard::new() {
                Ok(mut clipboard) => {
                    if clipboard.set_text(topic.clone()).is_ok() {
                        self.set_status("Topic copied to clipboard");
                    } else {
                        self.set_status("Failed to copy topic");
                    }
                }
                Err(_) => self.set_status("Clipboard unavailable"),
            }
        } else {
            self.set_status("No topic selected");
        }
    }

    /// Copy current payload to clipboard
    pub fn copy_payload(&mut self) {
        let messages = self.get_current_messages();
        if let Some(msg) = messages.first() {
            let payload = self.format_payload(msg);
            match arboard::Clipboard::new() {
                Ok(mut clipboard) => {
                    if clipboard.set_text(payload).is_ok() {
                        self.set_status("Payload copied to clipboard");
                    } else {
                        self.set_status("Failed to copy payload");
                    }
                }
                Err(_) => self.set_status("Clipboard unavailable"),
            }
        } else {
            self.set_status("No message to copy");
        }
    }

    fn handle_filter_input(&mut self, code: KeyCode, _modifiers: KeyModifiers) {
        match code {
            KeyCode::Esc => {
                self.input_mode = InputMode::Normal;
                self.filter_input.clear();
            }
            KeyCode::Enter => {
                if self.filter_input.is_empty() {
                    self.topic_filter = None;
                    self.set_status("Filter cleared");
                } else {
                    self.topic_filter = Some(self.filter_input.clone());
                    self.set_status(&format!("Filter: {}", self.filter_input));
                }
                self.input_mode = InputMode::Normal;
                self.filter_input.clear();
                self.selected_topic_index = 0;
            }
            KeyCode::Backspace => {
                self.filter_input.pop();
            }
            KeyCode::Char(c) => {
                self.filter_input.push(c);
            }
            _ => {}
        }
    }

    /// Clear the topic filter
    pub fn clear_filter(&mut self) {
        self.topic_filter = None;
        self.filter_input.clear();
        self.set_status("Filter cleared");
        self.selected_topic_index = 0;
    }

    pub fn open_server_manager(&mut self) {
        self.input_mode = InputMode::ServerManager;
        self.server_manager_index = self.config.mqtt.active_index().unwrap_or_default();
        self.server_edit.active = false;
        self.set_status("Server manager");
    }

    fn handle_search_input(&mut self, code: KeyCode, _modifiers: KeyModifiers) {
        match code {
            KeyCode::Esc => {
                self.input_mode = InputMode::Normal;
                self.search_query.clear();
                self.search_results.clear();
                self.search_scroll = 0;
            }
            KeyCode::Enter => {
                if !self.search_results.is_empty() {
                    if let Some(topic) = self.search_results.get(self.search_result_index).cloned()
                    {
                        self.selected_topic = Some(topic.clone());
                        self.expand_to_topic(&topic);
                    }
                }
                self.input_mode = InputMode::Normal;
                self.search_query.clear();
                self.search_results.clear();
                self.search_scroll = 0;
            }
            KeyCode::Backspace => {
                self.search_query.pop();
                self.update_search_results();
            }
            KeyCode::Char(c) => {
                self.search_query.push(c);
                self.update_search_results();
            }
            KeyCode::Down => {
                if self.search_result_index + 1 < self.search_results.len() {
                    self.search_result_index += 1;
                    self.ensure_search_visible();
                }
            }
            KeyCode::Up => {
                if self.search_result_index > 0 {
                    self.search_result_index -= 1;
                    self.ensure_search_visible();
                }
            }
            KeyCode::PageDown => {
                if !self.search_results.is_empty() {
                    let step = 5usize;
                    self.search_result_index =
                        (self.search_result_index + step).min(self.search_results.len() - 1);
                    self.ensure_search_visible();
                }
            }
            KeyCode::PageUp => {
                let step = 5usize;
                self.search_result_index = self.search_result_index.saturating_sub(step);
                self.ensure_search_visible();
            }
            KeyCode::Home => {
                if !self.search_results.is_empty() {
                    self.search_result_index = 0;
                    self.ensure_search_visible();
                }
            }
            KeyCode::End => {
                if !self.search_results.is_empty() {
                    self.search_result_index = self.search_results.len() - 1;
                    self.ensure_search_visible();
                }
            }
            _ => {}
        }
    }

    fn handle_normal_input(&mut self, code: KeyCode, modifiers: KeyModifiers) {
        // Global shortcuts
        if modifiers.contains(KeyModifiers::CONTROL) {
            match code {
                KeyCode::Char('c') | KeyCode::Char('q') => {
                    self.should_quit = true;
                    return;
                }
                _ => {}
            }
        }

        match code {
            // Quit
            KeyCode::Char('q') => self.should_quit = true,

            // Help
            KeyCode::Char('?') => self.show_help = !self.show_help,

            // Search
            KeyCode::Char('/') => {
                self.input_mode = InputMode::Search;
                self.search_query.clear();
                self.search_results.clear();
                self.search_result_index = 0;
                self.search_scroll = 0;
            }

            // Panel navigation
            KeyCode::Tab => self.next_panel(),
            KeyCode::BackTab => self.prev_panel(),
            KeyCode::Char('1') => self.focused_panel = Panel::TopicTree,
            KeyCode::Char('2') => self.focused_panel = Panel::Messages,
            KeyCode::Char('3') => self.focused_panel = Panel::Stats,

            // Payload mode toggle
            KeyCode::Char('p') => self.cycle_payload_mode(),

            // Clear stats
            KeyCode::Char('c') => self.stats.reset(),

            // Star current topic
            KeyCode::Char('s') => self.toggle_star(),

            // Toggle starred filter
            KeyCode::Char('*') => self.toggle_filter_mode(),

            // Track metric from current message
            KeyCode::Char('m') => self.enter_metric_select(),

            // Copy to clipboard
            KeyCode::Char('y') => self.copy_topic(),
            KeyCode::Char('Y') => self.copy_payload(),

            // Topic filter
            KeyCode::Char('f') => {
                self.input_mode = InputMode::Filter;
                self.filter_input = self.topic_filter.clone().unwrap_or_default();
            }
            KeyCode::Char('F') => self.clear_filter(),

            // Navigation (vim-style + arrows)
            KeyCode::Down | KeyCode::Char('j') => self.move_down(),
            KeyCode::Up | KeyCode::Char('k') => self.move_up(),
            KeyCode::Left | KeyCode::Char('h') => self.collapse_or_left(),
            KeyCode::Right | KeyCode::Char('l') => self.expand_or_right(),
            KeyCode::Char('L') => self.expand_branch(),
            KeyCode::Char('H') => self.collapse_branch(),

            // Expand/collapse
            KeyCode::Enter => self.toggle_expand(),

            // Page navigation
            KeyCode::PageDown => self.page_down(),
            KeyCode::PageUp => self.page_up(),
            KeyCode::Home | KeyCode::Char('g') => self.goto_top(),
            KeyCode::End | KeyCode::Char('G') => self.goto_bottom(),
            KeyCode::Char('S') => self.open_server_manager(),

            // Escape closes help
            KeyCode::Esc => {
                if self.show_help {
                    self.show_help = false;
                }
            }

            _ => {}
        }
    }

    fn next_panel(&mut self) {
        self.focused_panel = match self.focused_panel {
            Panel::TopicTree => Panel::Messages,
            Panel::Messages => Panel::Stats,
            Panel::Stats => Panel::TopicTree,
        };
    }

    fn prev_panel(&mut self) {
        self.focused_panel = match self.focused_panel {
            Panel::TopicTree => Panel::Stats,
            Panel::Messages => Panel::TopicTree,
            Panel::Stats => Panel::Messages,
        };
    }

    fn cycle_payload_mode(&mut self) {
        self.payload_mode = match self.payload_mode {
            PayloadMode::Auto => PayloadMode::Raw,
            PayloadMode::Raw => PayloadMode::Hex,
            PayloadMode::Hex => PayloadMode::Json,
            PayloadMode::Json => PayloadMode::Auto,
        };
    }

    fn move_down(&mut self) {
        match self.focused_panel {
            Panel::TopicTree => {
                let visible = self.get_visible_topics();
                if !visible.is_empty() && self.selected_topic_index < visible.len() - 1 {
                    self.selected_topic_index += 1;
                    self.update_selected_topic();
                }
            }
            Panel::Messages => {
                let count = self.get_current_messages().len();
                if count > 0 && self.selected_message_index < count - 1 {
                    self.selected_message_index += 1;
                }
            }
            Panel::Stats => {
                self.stats_scroll = self.stats_scroll.saturating_add(1);
            }
        }
    }

    fn move_up(&mut self) {
        match self.focused_panel {
            Panel::TopicTree => {
                if self.selected_topic_index > 0 {
                    self.selected_topic_index -= 1;
                    self.update_selected_topic();
                }
            }
            Panel::Messages => {
                if self.selected_message_index > 0 {
                    self.selected_message_index -= 1;
                }
            }
            Panel::Stats => {
                self.stats_scroll = self.stats_scroll.saturating_sub(1);
            }
        }
    }

    fn expand_or_right(&mut self) {
        if self.focused_panel == Panel::TopicTree {
            let visible = self.get_visible_topics();
            if let Some(topic) = visible.get(self.selected_topic_index) {
                if topic.has_children && !topic.is_expanded {
                    self.expanded_topics.insert(topic.full_path.clone());
                } else if topic.has_children && topic.is_expanded {
                    let target_depth = topic.depth + 1;
                    for (idx, entry) in visible
                        .iter()
                        .enumerate()
                        .skip(self.selected_topic_index + 1)
                    {
                        if entry.depth == target_depth {
                            self.selected_topic_index = idx;
                            self.update_selected_topic();
                            break;
                        }
                        if entry.depth <= topic.depth {
                            break;
                        }
                    }
                }
            }
        }
    }

    fn collapse_or_left(&mut self) {
        if self.focused_panel == Panel::TopicTree {
            let visible = self.get_visible_topics();
            if let Some(topic) = visible.get(self.selected_topic_index) {
                if topic.is_expanded {
                    self.expanded_topics.remove(&topic.full_path);
                } else if topic.depth > 0 {
                    let parent_path = topic.full_path.rsplit_once('/').map(|(p, _)| p.to_string());
                    if let Some(parent) = parent_path {
                        for (i, t) in visible.iter().enumerate() {
                            if t.full_path == parent {
                                self.selected_topic_index = i;
                                break;
                            }
                        }
                    }
                }
            }
        }
    }

    fn expand_branch(&mut self) {
        if self.focused_panel != Panel::TopicTree {
            return;
        }
        let visible = self.get_visible_topics();
        if let Some(topic) = visible.get(self.selected_topic_index) {
            let paths = self.topic_tree.expandable_paths_from(&topic.full_path);
            for path in paths {
                self.expanded_topics.insert(path);
            }
        }
    }

    fn collapse_branch(&mut self) {
        if self.focused_panel != Panel::TopicTree {
            return;
        }
        let visible = self.get_visible_topics();
        if let Some(topic) = visible.get(self.selected_topic_index) {
            let paths = self.topic_tree.expandable_paths_from(&topic.full_path);
            for path in paths {
                self.expanded_topics.remove(&path);
            }
        }
    }

    fn toggle_expand(&mut self) {
        if self.focused_panel == Panel::TopicTree {
            let visible = self.get_visible_topics();
            if let Some(topic) = visible.get(self.selected_topic_index) {
                if topic.has_children {
                    if topic.is_expanded {
                        self.expanded_topics.remove(&topic.full_path);
                    } else {
                        self.expanded_topics.insert(topic.full_path.clone());
                    }
                }
                self.selected_topic = Some(topic.full_path.clone());
                self.selected_message_index = 0;
            }
        }
    }

    fn page_down(&mut self) {
        for _ in 0..10 {
            self.move_down();
        }
    }

    fn page_up(&mut self) {
        for _ in 0..10 {
            self.move_up();
        }
    }

    fn goto_top(&mut self) {
        match self.focused_panel {
            Panel::TopicTree => {
                self.selected_topic_index = 0;
                self.update_selected_topic();
            }
            Panel::Messages => {
                self.selected_message_index = 0;
            }
            Panel::Stats => {
                self.stats_scroll = 0;
            }
        }
    }

    fn goto_bottom(&mut self) {
        match self.focused_panel {
            Panel::TopicTree => {
                let visible = self.get_visible_topics();
                if !visible.is_empty() {
                    self.selected_topic_index = visible.len() - 1;
                    self.update_selected_topic();
                }
            }
            Panel::Messages => {
                let count = self.get_current_messages().len();
                if count > 0 {
                    self.selected_message_index = count - 1;
                }
            }
            Panel::Stats => {
                self.stats_scroll = 100; // Scroll to approximate bottom
            }
        }
    }

    fn update_selected_topic(&mut self) {
        let visible = self.get_visible_topics();
        if let Some(topic) = visible.get(self.selected_topic_index) {
            self.selected_topic = Some(topic.full_path.clone());
            self.selected_message_index = 0;
        }
    }

    fn update_search_results(&mut self) {
        if self.search_query.is_empty() {
            self.search_results.clear();
            self.search_result_index = 0;
            self.search_scroll = 0;
        } else {
            self.search_results = self.topic_tree.search(&self.search_query);
            self.search_result_index = 0;
            self.search_scroll = 0;
        }
    }

    pub fn ensure_search_visible(&mut self) {
        self.ensure_search_visible_with_window(12);
    }

    pub fn ensure_search_visible_with_window(&mut self, window: usize) {
        if window == 0 || self.search_results.is_empty() {
            self.search_scroll = 0;
            return;
        }
        let max_scroll = self.search_results.len().saturating_sub(1);
        if self.search_result_index < self.search_scroll {
            self.search_scroll = self.search_result_index;
        } else if self.search_result_index >= self.search_scroll + window {
            self.search_scroll = self.search_result_index + 1 - window;
        }
        if self.search_scroll > max_scroll {
            self.search_scroll = max_scroll;
        }
    }

    fn expand_to_topic(&mut self, topic: &str) {
        // Expand all parent topics
        let parts: Vec<&str> = topic.split('/').collect();
        let mut path = String::new();

        for (i, part) in parts.iter().enumerate() {
            if i > 0 {
                path.push('/');
            }
            path.push_str(part);

            if i < parts.len() - 1 {
                self.expanded_topics.insert(path.clone());
            }
        }

        // Update selected index
        let visible = self.get_visible_topics();
        for (i, t) in visible.iter().enumerate() {
            if t.full_path == topic {
                self.selected_topic_index = i;
                break;
            }
        }
    }

    /// Get visible topics for rendering
    pub fn get_visible_topics(&self) -> Vec<TopicInfo> {
        let topics = self.topic_tree.get_visible_topics(&self.expanded_topics);

        // Apply starred filter
        let topics = match self.filter_mode {
            FilterMode::All => topics,
            FilterMode::Starred => topics
                .into_iter()
                .filter(|t| self.user_data.is_starred(&t.full_path))
                .collect(),
        };

        // Apply topic pattern filter
        if let Some(pattern) = &self.topic_filter {
            topics
                .into_iter()
                .filter(|t| topic_matches(pattern, &t.full_path))
                .collect()
        } else {
            topics
        }
    }

    pub fn active_server(&self) -> Option<&MqttServerConfig> {
        self.config.mqtt.active_server()
    }

    pub fn reset_for_server_switch(&mut self, server_index: usize) -> Result<()> {
        let server = self
            .config
            .mqtt
            .servers
            .get(server_index)
            .context("Server index out of range")?
            .name
            .clone();

        self.config.mqtt.active_server = server.clone();
        self.save_config()?;

        self.topic_tree.clear();
        self.message_buffer.clear();
        self.stats.reset();
        self.metric_tracker = MetricTracker::new(100);
        self.device_tracker = DeviceTracker::new();
        self.latency_tracker = LatencyTracker::new(100);
        self.schema_tracker = SchemaTracker::new();
        self.selected_topic_index = 0;
        self.selected_message_index = 0;
        self.selected_topic = None;
        self.expanded_topics.clear();
        self.stats_scroll = 0;
        self.message_scroll = 0;
        self.tree_scroll = 0;

        self.set_status(&format!("Switched to {}", server));
        Ok(())
    }

    pub fn handle_server_manager_input(&mut self, code: KeyCode, _modifiers: KeyModifiers) {
        if self.server_edit.active {
            self.handle_server_edit_input(code);
            return;
        }

        match code {
            KeyCode::Esc => {
                self.input_mode = InputMode::Normal;
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.server_manager_index + 1 < self.config.mqtt.servers.len() {
                    self.server_manager_index += 1;
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.server_manager_index > 0 {
                    self.server_manager_index -= 1;
                }
            }
            KeyCode::Char('a') => {
                self.start_server_edit(None);
            }
            KeyCode::Char('e') | KeyCode::Enter => {
                if !self.config.mqtt.servers.is_empty() {
                    let index = self
                        .server_manager_index
                        .min(self.config.mqtt.servers.len() - 1);
                    self.start_server_edit(Some(index));
                }
            }
            KeyCode::Char('w') => {
                if let Err(err) = self.save_config() {
                    self.set_status(&format!("Save failed: {}", err));
                } else {
                    self.set_status("Config saved");
                }
            }
            KeyCode::Char('d') => {
                if self.config.mqtt.servers.len() <= 1 {
                    self.set_status("Cannot delete last server");
                } else if let Some(server) = self.config.mqtt.servers.get(self.server_manager_index)
                {
                    let name = server.name.clone();
                    let was_active = self.config.mqtt.active_server == name;
                    self.config.mqtt.servers.remove(self.server_manager_index);
                    if was_active {
                        self.config.mqtt.active_server = self
                            .config
                            .mqtt
                            .servers
                            .get(0)
                            .map(|s| s.name.clone())
                            .unwrap_or_default();
                    }
                    self.server_manager_index = self
                        .server_manager_index
                        .min(self.config.mqtt.servers.len().saturating_sub(1));
                    if let Err(err) = self.save_config() {
                        self.set_status(&format!("Save failed: {}", err));
                    } else {
                        if was_active {
                            self.pending_server_switch = self.config.mqtt.active_index();
                        }
                        self.set_status("Server deleted");
                    }
                }
            }
            KeyCode::Char(' ') => {
                if let Some(server) = self.config.mqtt.servers.get(self.server_manager_index) {
                    self.config.mqtt.active_server = server.name.clone();
                    if let Err(err) = self.save_config() {
                        self.set_status(&format!("Save failed: {}", err));
                    } else {
                        self.pending_server_switch = Some(self.server_manager_index);
                    }
                }
            }
            _ => {}
        }
    }

    fn start_server_edit(&mut self, index: Option<usize>) {
        self.server_edit.active = true;
        self.server_edit.is_new = index.is_none();
        self.server_edit.field = ServerField::Name;
        self.server_edit.cursor = 0;
        if let Some(index) = index {
            let server = &self.config.mqtt.servers[index];
            self.server_edit.index = index;
            self.server_edit.name = server.name.clone();
            self.server_edit.host = server.host.clone();
            self.server_edit.port = server.port.to_string();
            self.server_edit.use_tls = server.use_tls;
            self.server_edit.client_id = server.client_id.clone();
            self.server_edit.username = server.username.clone().unwrap_or_default();
            self.server_edit.token = server.token.clone().unwrap_or_default();
            self.server_edit.subscribe_topic = server.subscribe_topic.clone();
            self.server_edit.keep_alive_secs = server.keep_alive_secs.to_string();
            self.server_edit.cursor = self.server_edit_field_value(self.server_edit.field).len();
        } else {
            self.server_edit.index = self.config.mqtt.servers.len();
            self.server_edit.name.clear();
            self.server_edit.host.clear();
            self.server_edit.port = "1883".to_string();
            self.server_edit.use_tls = false;
            self.server_edit.client_id.clear();
            self.server_edit.username.clear();
            self.server_edit.token.clear();
            self.server_edit.subscribe_topic = "#".to_string();
            self.server_edit.keep_alive_secs = "30".to_string();
            self.server_edit.cursor = self.server_edit_field_value(self.server_edit.field).len();
        }
    }

    fn handle_server_edit_input(&mut self, code: KeyCode) {
        match code {
            KeyCode::Esc => {
                self.server_edit.active = false;
            }
            KeyCode::Enter => match self.apply_server_edit() {
                Ok(()) => {
                    self.server_edit.active = false;
                }
                Err(err) => {
                    self.set_status(&format!("Invalid: {}", err));
                }
            },
            KeyCode::Tab => {
                self.server_edit.field = self.next_server_field(self.server_edit.field);
                self.server_edit.cursor =
                    self.server_edit_field_value(self.server_edit.field).len();
            }
            KeyCode::BackTab => {
                self.server_edit.field = self.prev_server_field(self.server_edit.field);
                self.server_edit.cursor =
                    self.server_edit_field_value(self.server_edit.field).len();
            }
            KeyCode::Left => {
                if self.server_edit.cursor > 0 {
                    self.server_edit.cursor -= 1;
                }
            }
            KeyCode::Right => {
                let max = self.server_edit_field_value(self.server_edit.field).len();
                if self.server_edit.cursor < max {
                    self.server_edit.cursor += 1;
                }
            }
            KeyCode::Home => {
                self.server_edit.cursor = 0;
            }
            KeyCode::End => {
                self.server_edit.cursor =
                    self.server_edit_field_value(self.server_edit.field).len();
            }
            KeyCode::Char(' ') if self.server_edit.field == ServerField::UseTls => {
                self.server_edit.use_tls = !self.server_edit.use_tls;
            }
            KeyCode::Backspace => {
                if self.server_edit.field != ServerField::UseTls {
                    self.server_edit_backspace();
                }
            }
            KeyCode::Delete => {
                if self.server_edit.field != ServerField::UseTls {
                    self.server_edit_delete();
                }
            }
            KeyCode::Char(c) => {
                if self.server_edit.field != ServerField::UseTls {
                    self.server_edit_insert(c);
                }
            }
            _ => {}
        }
    }

    fn server_edit_mut_field(&mut self) -> &mut String {
        match self.server_edit.field {
            ServerField::Name => &mut self.server_edit.name,
            ServerField::Host => &mut self.server_edit.host,
            ServerField::Port => &mut self.server_edit.port,
            ServerField::UseTls => &mut self.server_edit.host,
            ServerField::ClientId => &mut self.server_edit.client_id,
            ServerField::Username => &mut self.server_edit.username,
            ServerField::Token => &mut self.server_edit.token,
            ServerField::SubscribeTopic => &mut self.server_edit.subscribe_topic,
            ServerField::KeepAlive => &mut self.server_edit.keep_alive_secs,
        }
    }

    fn server_edit_insert(&mut self, ch: char) {
        let mut cursor = self.server_edit.cursor;
        let value = self.server_edit_mut_field();
        if cursor > value.len() {
            cursor = value.len();
        }
        value.insert(cursor, ch);
        self.server_edit.cursor = cursor.saturating_add(1);
    }

    fn server_edit_backspace(&mut self) {
        let mut cursor = self.server_edit.cursor;
        let value = self.server_edit_mut_field();
        if cursor == 0 || value.is_empty() {
            return;
        }
        if cursor > value.len() {
            cursor = value.len();
        }
        let remove_at = cursor.saturating_sub(1);
        value.remove(remove_at);
        self.server_edit.cursor = cursor.saturating_sub(1);
    }

    fn server_edit_delete(&mut self) {
        let cursor = self.server_edit.cursor;
        let value = self.server_edit_mut_field();
        if value.is_empty() {
            return;
        }
        if cursor >= value.len() {
            return;
        }
        value.remove(cursor);
        self.server_edit.cursor = cursor;
    }

    pub fn server_edit_field_value(&self, field: ServerField) -> String {
        match field {
            ServerField::Name => self.server_edit.name.clone(),
            ServerField::Host => self.server_edit.host.clone(),
            ServerField::Port => self.server_edit.port.clone(),
            ServerField::UseTls => {
                if self.server_edit.use_tls {
                    "on".to_string()
                } else {
                    "off".to_string()
                }
            }
            ServerField::ClientId => self.server_edit.client_id.clone(),
            ServerField::Username => self.server_edit.username.clone(),
            ServerField::Token => {
                if self.server_edit.token.is_empty() {
                    String::new()
                } else {
                    "********".to_string()
                }
            }
            ServerField::SubscribeTopic => self.server_edit.subscribe_topic.clone(),
            ServerField::KeepAlive => self.server_edit.keep_alive_secs.clone(),
        }
    }

    fn next_server_field(&self, field: ServerField) -> ServerField {
        let idx = ServerField::ALL
            .iter()
            .position(|f| *f == field)
            .unwrap_or(0);
        let next = (idx + 1) % ServerField::ALL.len();
        ServerField::ALL[next]
    }

    fn prev_server_field(&self, field: ServerField) -> ServerField {
        let idx = ServerField::ALL
            .iter()
            .position(|f| *f == field)
            .unwrap_or(0);
        let prev = idx.checked_sub(1).unwrap_or(ServerField::ALL.len() - 1);
        ServerField::ALL[prev]
    }

    fn apply_server_edit(&mut self) -> Result<()> {
        let port: u16 = self
            .server_edit
            .port
            .trim()
            .parse()
            .context("Port must be a number")?;
        let keep_alive_secs: u64 = self
            .server_edit
            .keep_alive_secs
            .trim()
            .parse()
            .context("Keep alive must be a number")?;

        let server = MqttServerConfig {
            name: self.server_edit.name.trim().to_string(),
            host: self.server_edit.host.trim().to_string(),
            port,
            use_tls: self.server_edit.use_tls,
            client_id: self.server_edit.client_id.trim().to_string(),
            username: if self.server_edit.username.trim().is_empty() {
                None
            } else {
                Some(self.server_edit.username.trim().to_string())
            },
            token: if self.server_edit.token.trim().is_empty() {
                None
            } else {
                Some(self.server_edit.token.trim().to_string())
            },
            subscribe_topic: if self.server_edit.subscribe_topic.trim().is_empty() {
                "#".to_string()
            } else {
                self.server_edit.subscribe_topic.trim().to_string()
            },
            keep_alive_secs,
        };

        if server.name.is_empty() || server.host.is_empty() || server.client_id.is_empty() {
            return Err(anyhow!("Name, host, and client ID are required"));
        }

        if self
            .config
            .mqtt
            .servers
            .iter()
            .enumerate()
            .any(|(idx, existing)| idx != self.server_edit.index && existing.name == server.name)
        {
            return Err(anyhow!("Server name must be unique"));
        }

        if server.port == 0 {
            return Err(anyhow!("Port must be greater than 0"));
        }

        let prev_active = self.config.mqtt.active_server.clone();
        if self.server_edit.is_new {
            self.config.mqtt.servers.push(server);
            self.server_manager_index = self.config.mqtt.servers.len().saturating_sub(1);
        } else if let Some(existing) = self.config.mqtt.servers.get_mut(self.server_edit.index) {
            *existing = server;
        }

        if self.config.mqtt.active_server.is_empty() {
            if let Some(server) = self.config.mqtt.servers.first() {
                self.config.mqtt.active_server = server.name.clone();
            }
        }

        self.save_config()?;
        if prev_active != self.config.mqtt.active_server {
            if let Some(index) = self.config.mqtt.active_index() {
                self.pending_server_switch = Some(index);
            }
        }
        self.set_status("Server saved");
        Ok(())
    }

    pub fn save_config(&self) -> Result<()> {
        self.config
            .save_with_backup(&self.config_path, CONFIG_BACKUP_LIMIT)
            .context("Failed to save config")?;
        Ok(())
    }

    /// Get messages for currently selected topic
    pub fn get_current_messages(&self) -> Vec<&MqttMessage> {
        self.selected_topic
            .as_ref()
            .map(|t| self.message_buffer.get_messages(t))
            .unwrap_or_default()
    }

    /// Get formatted payload for a message
    pub fn format_payload(&self, msg: &MqttMessage) -> String {
        match self.payload_mode {
            PayloadMode::Auto => {
                if let Some(json) = msg.payload_json_pretty() {
                    json
                } else if let Some(s) = msg.payload_str() {
                    s.to_string()
                } else {
                    msg.payload_hex()
                }
            }
            PayloadMode::Raw => msg
                .payload_str()
                .map(|s| s.to_string())
                .unwrap_or_else(|| msg.payload_hex()),
            PayloadMode::Hex => msg.payload_hex(),
            PayloadMode::Json => msg
                .payload_json_pretty()
                .unwrap_or_else(|| "<not valid JSON>".to_string()),
        }
    }

    /// Get connection status string
    pub fn connection_status(&self) -> &'static str {
        match self.connection_state {
            ConnectionState::Disconnected => "Disconnected",
            ConnectionState::Connecting => "Connecting...",
            ConnectionState::Connected => "Connected",
            ConnectionState::Reconnecting => "Reconnecting...",
        }
    }

    /// Get connection status color
    pub fn connection_color(&self) -> ratatui::style::Color {
        use ratatui::style::Color;
        match self.connection_state {
            ConnectionState::Disconnected => Color::Red,
            ConnectionState::Connecting => Color::Yellow,
            ConnectionState::Connected => Color::Green,
            ConnectionState::Reconnecting => Color::Yellow,
        }
    }
}

/// Create a wildcard pattern from a specific topic
/// Replaces segments that look like IDs with + wildcards
/// e.g., "telemetry/zap-0000d8c467e385a0/meter/zap/json" -> "telemetry/+/meter/+/json"
fn create_wildcard_pattern(topic: &str) -> String {
    topic
        .split('/')
        .map(|segment| {
            // Replace segments that look like device IDs or UUIDs
            if segment.len() > 8
                && (
                    segment.contains('-') ||  // UUIDs or device IDs like zap-0000d8c467e385a0
                segment.chars().all(|c| c.is_ascii_hexdigit()) ||  // Hex strings
                segment.starts_with("zap-") ||
                segment.starts_with("dev-") ||
                segment.parse::<u64>().is_ok()
                    // Numeric IDs
                )
            {
                "+".to_string()
            } else {
                segment.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("/")
}

/// Get a short version of a topic for display
fn short_topic(topic: &str) -> String {
    let parts: Vec<&str> = topic.split('/').collect();
    if parts.len() <= 2 {
        topic.to_string()
    } else {
        // Show first and last parts
        format!("{}/..", parts[0])
    }
}
