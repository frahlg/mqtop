#![allow(dead_code)]

use std::collections::HashMap;

/// A trie-based data structure for storing MQTT topic hierarchies efficiently.
/// Provides O(k) lookup where k is the number of topic levels.
#[derive(Debug, Default)]
pub struct TopicTree {
    root: TopicNode,
    total_topics: usize,
}

#[derive(Debug, Default)]
struct TopicNode {
    /// Child nodes keyed by topic segment
    children: HashMap<String, TopicNode>,
    /// Whether this node represents a complete topic (has received messages)
    is_topic: bool,
    /// Message count for this topic
    message_count: u64,
    /// Total bytes received on this topic
    bytes_received: u64,
    /// Last message timestamp (unix millis)
    last_message_time: Option<i64>,
}

/// Represents a topic in the tree for display
#[derive(Debug, Clone)]
pub struct TopicInfo {
    pub full_path: String,
    pub segment: String,
    pub depth: usize,
    pub is_expanded: bool,
    pub has_children: bool,
    pub message_count: u64,
    pub bytes_received: u64,
    pub last_message_time: Option<i64>,
}

impl TopicTree {
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert or update a topic in the tree
    pub fn insert(&mut self, topic: &str, payload_size: usize) {
        let segments: Vec<&str> = topic.split('/').collect();
        let mut current = &mut self.root;

        for segment in &segments {
            current = current
                .children
                .entry(segment.to_string())
                .or_default();
        }

        if !current.is_topic {
            current.is_topic = true;
            self.total_topics += 1;
        }

        current.message_count += 1;
        current.bytes_received += payload_size as u64;
        current.last_message_time = Some(chrono::Utc::now().timestamp_millis());
    }

    /// Get the total number of unique topics
    pub fn topic_count(&self) -> usize {
        self.total_topics
    }

    /// Get total message count across all topics
    pub fn total_messages(&self) -> u64 {
        self.count_messages(&self.root)
    }

    fn count_messages(&self, node: &TopicNode) -> u64 {
        let mut count = node.message_count;
        for child in node.children.values() {
            count += self.count_messages(child);
        }
        count
    }

    /// Get flattened list of topics for display (respecting expanded state)
    pub fn get_visible_topics(&self, expanded: &std::collections::HashSet<String>) -> Vec<TopicInfo> {
        let mut result = Vec::new();
        self.collect_visible(&self.root, "", 0, expanded, &mut result);
        result
    }

    fn collect_visible(
        &self,
        node: &TopicNode,
        path: &str,
        depth: usize,
        expanded: &std::collections::HashSet<String>,
        result: &mut Vec<TopicInfo>,
    ) {
        // Sort children for consistent display
        let mut children: Vec<_> = node.children.iter().collect();
        children.sort_by(|a, b| a.0.cmp(b.0));

        for (segment, child) in children {
            let full_path = if path.is_empty() {
                segment.clone()
            } else {
                format!("{}/{}", path, segment)
            };

            let is_expanded = expanded.contains(&full_path);
            let has_children = !child.children.is_empty();

            result.push(TopicInfo {
                full_path: full_path.clone(),
                segment: segment.clone(),
                depth,
                is_expanded,
                has_children,
                message_count: child.message_count,
                bytes_received: child.bytes_received,
                last_message_time: child.last_message_time,
            });

            // Only recurse if expanded
            if is_expanded {
                self.collect_visible(child, &full_path, depth + 1, expanded, result);
            }
        }
    }

    /// Get all topics matching a pattern (simple glob with *)
    pub fn search(&self, pattern: &str) -> Vec<String> {
        let mut results = Vec::new();
        self.search_recursive(&self.root, "", pattern.to_lowercase().as_str(), &mut results);
        results
    }

    fn search_recursive(
        &self,
        node: &TopicNode,
        path: &str,
        pattern: &str,
        results: &mut Vec<String>,
    ) {
        for (segment, child) in &node.children {
            let full_path = if path.is_empty() {
                segment.clone()
            } else {
                format!("{}/{}", path, segment)
            };

            // Simple substring match (case-insensitive)
            if full_path.to_lowercase().contains(pattern) && child.is_topic {
                results.push(full_path.clone());
            }

            self.search_recursive(child, &full_path, pattern, results);
        }
    }

    /// Get stats for a specific topic
    pub fn get_topic_stats(&self, topic: &str) -> Option<(u64, u64, Option<i64>)> {
        let segments: Vec<&str> = topic.split('/').collect();
        let mut current = &self.root;

        for segment in &segments {
            current = current.children.get(*segment)?;
        }

        if current.is_topic {
            Some((
                current.message_count,
                current.bytes_received,
                current.last_message_time,
            ))
        } else {
            None
        }
    }

    /// Clear all data
    pub fn clear(&mut self) {
        self.root = TopicNode::default();
        self.total_topics = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn test_insert_and_count() {
        let mut tree = TopicTree::new();

        tree.insert("sensors/temp/living_room", 10);
        tree.insert("sensors/temp/bedroom", 15);
        tree.insert("sensors/humidity/living_room", 8);

        assert_eq!(tree.topic_count(), 3);
        assert_eq!(tree.total_messages(), 3);
    }

    #[test]
    fn test_multiple_messages_same_topic() {
        let mut tree = TopicTree::new();

        tree.insert("sensors/temp", 10);
        tree.insert("sensors/temp", 12);
        tree.insert("sensors/temp", 11);

        assert_eq!(tree.topic_count(), 1);
        assert_eq!(tree.total_messages(), 3);

        let stats = tree.get_topic_stats("sensors/temp").unwrap();
        assert_eq!(stats.0, 3); // message_count
        assert_eq!(stats.1, 33); // bytes_received
    }

    #[test]
    fn test_hierarchical_structure() {
        let mut tree = TopicTree::new();

        tree.insert("a/b/c", 1);
        tree.insert("a/b/d", 1);
        tree.insert("a/e", 1);

        let mut expanded = HashSet::new();
        let visible = tree.get_visible_topics(&expanded);

        // Only top level visible when nothing expanded
        assert_eq!(visible.len(), 1);
        assert_eq!(visible[0].segment, "a");
        assert!(visible[0].has_children);

        // Expand "a"
        expanded.insert("a".to_string());
        let visible = tree.get_visible_topics(&expanded);

        // Now a, a/b, a/e visible
        assert_eq!(visible.len(), 3);
    }

    #[test]
    fn test_search() {
        let mut tree = TopicTree::new();

        tree.insert("sensors/temperature/room1", 1);
        tree.insert("sensors/temperature/room2", 1);
        tree.insert("sensors/humidity/room1", 1);
        tree.insert("devices/light/kitchen", 1);

        let results = tree.search("temp");
        assert_eq!(results.len(), 2);

        let results = tree.search("room1");
        assert_eq!(results.len(), 2);

        let results = tree.search("kitchen");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_case_insensitive_search() {
        let mut tree = TopicTree::new();
        tree.insert("Sensors/Temperature", 1);

        let results = tree.search("sensors");
        assert_eq!(results.len(), 1);

        let results = tree.search("TEMPERATURE");
        assert_eq!(results.len(), 1);
    }
}
