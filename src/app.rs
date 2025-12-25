use std::collections::HashSet;

use crossterm::event::{KeyCode, KeyModifiers};

use crate::config::Config;
use crate::mqtt::{ConnectionState, MqttEvent, MqttMessage};
use crate::state::{MessageBuffer, Stats, TopicInfo, TopicTree};

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
}

/// Application state
pub struct App {
    /// Configuration
    pub config: Config,
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
    /// Search query
    pub search_query: String,
    /// Search results
    pub search_results: Vec<String>,
    /// Selected search result index
    pub search_result_index: usize,
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
    /// Currently selected topic (full path)
    pub selected_topic: Option<String>,
    /// Show help overlay
    pub show_help: bool,
    /// Payload display mode
    pub payload_mode: PayloadMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PayloadMode {
    Auto,   // Auto-detect JSON vs raw
    Raw,    // Raw string
    Hex,    // Hex dump
    Json,   // Force JSON pretty-print
}

impl App {
    pub fn new(config: Config) -> Self {
        let message_buffer_size = config.ui.message_buffer_size;
        let stats_window = config.ui.stats_window_secs;

        Self {
            config,
            topic_tree: TopicTree::new(),
            message_buffer: MessageBuffer::new(message_buffer_size),
            stats: Stats::new(stats_window),
            selected_topic_index: 0,
            selected_message_index: 0,
            expanded_topics: HashSet::new(),
            focused_panel: Panel::TopicTree,
            input_mode: InputMode::Normal,
            search_query: String::new(),
            search_results: Vec::new(),
            search_result_index: 0,
            connection_state: ConnectionState::Disconnected,
            last_error: None,
            should_quit: false,
            tree_scroll: 0,
            message_scroll: 0,
            selected_topic: None,
            show_help: false,
            payload_mode: PayloadMode::Auto,
        }
    }

    /// Process an MQTT event
    pub fn handle_mqtt_event(&mut self, event: MqttEvent) {
        match event {
            MqttEvent::Message(msg) => {
                self.stats.record_message(msg.payload_size());
                self.topic_tree.insert(&msg.topic, msg.payload_size());
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
        }
    }

    fn handle_search_input(&mut self, code: KeyCode, _modifiers: KeyModifiers) {
        match code {
            KeyCode::Esc => {
                self.input_mode = InputMode::Normal;
                self.search_query.clear();
                self.search_results.clear();
            }
            KeyCode::Enter => {
                if !self.search_results.is_empty() {
                    // Select the topic and exit search
                    if let Some(topic) = self.search_results.get(self.search_result_index).cloned() {
                        self.selected_topic = Some(topic.clone());
                        // Expand parent topics
                        self.expand_to_topic(&topic);
                    }
                }
                self.input_mode = InputMode::Normal;
                self.search_query.clear();
                self.search_results.clear();
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
                if !self.search_results.is_empty() {
                    self.search_result_index =
                        (self.search_result_index + 1) % self.search_results.len();
                }
            }
            KeyCode::Up => {
                if !self.search_results.is_empty() {
                    self.search_result_index = self.search_result_index
                        .checked_sub(1)
                        .unwrap_or(self.search_results.len() - 1);
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

            // Navigation (vim-style + arrows)
            KeyCode::Down | KeyCode::Char('j') => self.move_down(),
            KeyCode::Up | KeyCode::Char('k') => self.move_up(),
            KeyCode::Left | KeyCode::Char('h') => self.collapse_or_left(),
            KeyCode::Right | KeyCode::Char('l') => self.expand_or_right(),

            // Expand/collapse
            KeyCode::Enter => self.toggle_expand(),

            // Page navigation
            KeyCode::PageDown => self.page_down(),
            KeyCode::PageUp => self.page_up(),
            KeyCode::Home | KeyCode::Char('g') => self.goto_top(),
            KeyCode::End | KeyCode::Char('G') => self.goto_bottom(),

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
            Panel::Stats => {}
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
            Panel::Stats => {}
        }
    }

    fn expand_or_right(&mut self) {
        if self.focused_panel == Panel::TopicTree {
            let visible = self.get_visible_topics();
            if let Some(topic) = visible.get(self.selected_topic_index) {
                if topic.has_children && !topic.is_expanded {
                    self.expanded_topics.insert(topic.full_path.clone());
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
                    // Go to parent
                    let parent_path = topic.full_path
                        .rsplit_once('/')
                        .map(|(p, _)| p.to_string());
                    if let Some(parent) = parent_path {
                        // Find parent index
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
            Panel::Stats => {}
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
            Panel::Stats => {}
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
        } else {
            self.search_results = self.topic_tree.search(&self.search_query);
            self.search_result_index = 0;
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
        self.topic_tree.get_visible_topics(&self.expanded_topics)
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
            PayloadMode::Raw => {
                msg.payload_str()
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| msg.payload_hex())
            }
            PayloadMode::Hex => msg.payload_hex(),
            PayloadMode::Json => {
                msg.payload_json_pretty()
                    .unwrap_or_else(|| "<not valid JSON>".to_string())
            }
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
