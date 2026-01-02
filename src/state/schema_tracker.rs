#![allow(dead_code)]

use std::collections::{HashMap, HashSet};

/// Tracks JSON schema changes for topics
#[derive(Debug, Default)]
pub struct SchemaTracker {
    /// Known schemas by topic (field paths -> type)
    schemas: HashMap<String, Schema>,
    /// Recent schema changes
    changes: Vec<SchemaChange>,
    /// Max changes to keep
    max_changes: usize,
}

/// Represents a JSON schema (simplified)
#[derive(Debug, Clone, PartialEq)]
pub struct Schema {
    /// Field paths with their types (e.g., "data.power" -> "number")
    pub fields: HashMap<String, FieldType>,
}

/// Simple field type classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FieldType {
    Null,
    Boolean,
    Number,
    String,
    Array,
    Object,
}

/// Records a schema change
#[derive(Debug, Clone)]
pub struct SchemaChange {
    pub topic: String,
    pub change_type: ChangeType,
    pub field_path: String,
    pub old_type: Option<FieldType>,
    pub new_type: Option<FieldType>,
    pub timestamp: std::time::Instant,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeType {
    FieldAdded,
    FieldRemoved,
    TypeChanged,
}

impl SchemaTracker {
    pub fn new() -> Self {
        Self {
            schemas: HashMap::new(),
            changes: Vec::new(),
            max_changes: 50,
        }
    }

    /// Process a message and detect schema changes
    pub fn process_message(&mut self, topic: &str, payload: &[u8]) -> Vec<SchemaChange> {
        let json: serde_json::Value = match serde_json::from_slice(payload) {
            Ok(v) => v,
            Err(_) => return Vec::new(),
        };

        let new_schema = Schema::from_json(&json);
        let mut detected_changes = Vec::new();

        if let Some(old_schema) = self.schemas.get(topic) {
            // Compare schemas
            detected_changes = self.compare_schemas(topic, old_schema, &new_schema);

            // Record changes
            for change in &detected_changes {
                if self.changes.len() >= self.max_changes {
                    self.changes.remove(0);
                }
                self.changes.push(change.clone());
            }
        }

        // Update stored schema
        self.schemas.insert(topic.to_string(), new_schema);

        detected_changes
    }

    fn compare_schemas(&self, topic: &str, old: &Schema, new: &Schema) -> Vec<SchemaChange> {
        let mut changes = Vec::new();
        let now = std::time::Instant::now();

        let old_fields: HashSet<_> = old.fields.keys().collect();
        let new_fields: HashSet<_> = new.fields.keys().collect();

        // Check for added fields
        for field in new_fields.difference(&old_fields) {
            changes.push(SchemaChange {
                topic: topic.to_string(),
                change_type: ChangeType::FieldAdded,
                field_path: (*field).clone(),
                old_type: None,
                new_type: new.fields.get(*field).copied(),
                timestamp: now,
            });
        }

        // Check for removed fields
        for field in old_fields.difference(&new_fields) {
            changes.push(SchemaChange {
                topic: topic.to_string(),
                change_type: ChangeType::FieldRemoved,
                field_path: (*field).clone(),
                old_type: old.fields.get(*field).copied(),
                new_type: None,
                timestamp: now,
            });
        }

        // Check for type changes
        for field in old_fields.intersection(&new_fields) {
            let old_type = old.fields.get(*field);
            let new_type = new.fields.get(*field);

            if old_type != new_type {
                changes.push(SchemaChange {
                    topic: topic.to_string(),
                    change_type: ChangeType::TypeChanged,
                    field_path: (*field).clone(),
                    old_type: old_type.copied(),
                    new_type: new_type.copied(),
                    timestamp: now,
                });
            }
        }

        changes
    }

    /// Get recent schema changes
    pub fn recent_changes(&self) -> &[SchemaChange] {
        &self.changes
    }

    /// Check if there are any recent changes
    pub fn has_recent_changes(&self, since_secs: u64) -> bool {
        let cutoff = std::time::Duration::from_secs(since_secs);
        self.changes.iter().any(|c| c.timestamp.elapsed() < cutoff)
    }

    /// Get the current schema for a topic
    pub fn get_schema(&self, topic: &str) -> Option<&Schema> {
        self.schemas.get(topic)
    }

    /// Get number of tracked topics
    pub fn topic_count(&self) -> usize {
        self.schemas.len()
    }

    /// Clear all recorded changes
    pub fn clear_changes(&mut self) {
        self.changes.clear();
    }
}

impl Schema {
    pub fn from_json(value: &serde_json::Value) -> Self {
        let mut fields = HashMap::new();
        Self::extract_fields(value, "", &mut fields);
        Self { fields }
    }

    fn extract_fields(
        value: &serde_json::Value,
        prefix: &str,
        fields: &mut HashMap<String, FieldType>,
    ) {
        match value {
            serde_json::Value::Null => {
                if !prefix.is_empty() {
                    fields.insert(prefix.to_string(), FieldType::Null);
                }
            }
            serde_json::Value::Bool(_) => {
                if !prefix.is_empty() {
                    fields.insert(prefix.to_string(), FieldType::Boolean);
                }
            }
            serde_json::Value::Number(_) => {
                if !prefix.is_empty() {
                    fields.insert(prefix.to_string(), FieldType::Number);
                }
            }
            serde_json::Value::String(_) => {
                if !prefix.is_empty() {
                    fields.insert(prefix.to_string(), FieldType::String);
                }
            }
            serde_json::Value::Array(arr) => {
                if !prefix.is_empty() {
                    fields.insert(prefix.to_string(), FieldType::Array);
                }
                // Check first element for nested structure
                if let Some(first) = arr.first() {
                    let elem_prefix = if prefix.is_empty() {
                        "[0]".to_string()
                    } else {
                        format!("{}[0]", prefix)
                    };
                    Self::extract_fields(first, &elem_prefix, fields);
                }
            }
            serde_json::Value::Object(map) => {
                if !prefix.is_empty() {
                    fields.insert(prefix.to_string(), FieldType::Object);
                }
                for (key, val) in map {
                    let field_prefix = if prefix.is_empty() {
                        key.clone()
                    } else {
                        format!("{}.{}", prefix, key)
                    };
                    Self::extract_fields(val, &field_prefix, fields);
                }
            }
        }
    }

    /// Get field count
    pub fn field_count(&self) -> usize {
        self.fields.len()
    }
}

impl std::fmt::Display for FieldType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FieldType::Null => write!(f, "null"),
            FieldType::Boolean => write!(f, "bool"),
            FieldType::Number => write!(f, "number"),
            FieldType::String => write!(f, "string"),
            FieldType::Array => write!(f, "array"),
            FieldType::Object => write!(f, "object"),
        }
    }
}

impl std::fmt::Display for ChangeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChangeType::FieldAdded => write!(f, "+"),
            ChangeType::FieldRemoved => write!(f, "-"),
            ChangeType::TypeChanged => write!(f, "~"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_extraction() {
        let json: serde_json::Value = serde_json::json!({
            "name": "test",
            "value": 42,
            "active": true,
            "data": {
                "nested": "value"
            }
        });

        let schema = Schema::from_json(&json);

        assert_eq!(schema.fields.get("name"), Some(&FieldType::String));
        assert_eq!(schema.fields.get("value"), Some(&FieldType::Number));
        assert_eq!(schema.fields.get("active"), Some(&FieldType::Boolean));
        assert_eq!(schema.fields.get("data"), Some(&FieldType::Object));
        assert_eq!(schema.fields.get("data.nested"), Some(&FieldType::String));
    }

    #[test]
    fn test_schema_change_detection() {
        let mut tracker = SchemaTracker::new();

        // First message establishes schema
        let payload1 = br#"{"name": "test", "value": 42}"#;
        let changes1 = tracker.process_message("topic/test", payload1);
        assert!(changes1.is_empty()); // No changes on first message

        // Second message with same schema - no changes
        let payload2 = br#"{"name": "other", "value": 100}"#;
        let changes2 = tracker.process_message("topic/test", payload2);
        assert!(changes2.is_empty());

        // Third message with added field
        let payload3 = br#"{"name": "test", "value": 42, "new_field": "hello"}"#;
        let changes3 = tracker.process_message("topic/test", payload3);
        assert_eq!(changes3.len(), 1);
        assert_eq!(changes3[0].change_type, ChangeType::FieldAdded);
        assert_eq!(changes3[0].field_path, "new_field");
    }

    #[test]
    fn test_type_change_detection() {
        let mut tracker = SchemaTracker::new();

        let payload1 = br#"{"value": 42}"#;
        tracker.process_message("topic/test", payload1);

        // Change value from number to string
        let payload2 = br#"{"value": "forty-two"}"#;
        let changes = tracker.process_message("topic/test", payload2);

        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].change_type, ChangeType::TypeChanged);
        assert_eq!(changes[0].old_type, Some(FieldType::Number));
        assert_eq!(changes[0].new_type, Some(FieldType::String));
    }
}
