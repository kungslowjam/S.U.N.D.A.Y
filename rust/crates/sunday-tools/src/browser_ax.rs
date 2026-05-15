use serde_json::Value;
use std::fmt::Write;
use serde::Serialize;
use sunday_core::shared_mem::SharedMemorySegment;
use sunday_core::events::{emit_event, EventType};

#[derive(Serialize, Debug, Clone)]
pub struct ScoredElement {
    pub id: usize,
    pub role: String,
    pub name: String,
    pub value: String,
    pub score: i32,
    pub x: i32,
    pub y: i32,
}

pub struct AXTreeProcessor {
    pub max_depth: usize,
    pub filter_unimportant: bool,
}

impl AXTreeProcessor {
    pub fn new(max_depth: usize, filter_unimportant: bool) -> Self {
        Self {
            max_depth,
            filter_unimportant,
        }
    }

    /// Process the tree into a human-readable tree string (Legacy mode)
    pub fn process(&self, root: &Value) -> String {
        let mut output = String::with_capacity(4096);
        self.format_node(root, 0, &mut output);
        output
    }

    /// Process the tree and save to Shared Memory
    pub fn process_to_shm(&self, root: &Value, shm_name: &str) -> Result<String, String> {
        let mut elements = Vec::new();
        self.extract_scored_elements(root, 0, &mut elements);
        
        elements.sort_by(|a, b| b.score.cmp(&a.score).then(a.id.cmp(&b.id)));
        let top_elements = elements.into_iter().take(50).collect::<Vec<_>>();

        let mut text_output = String::from("Interactive Elements (Compressed):\n");
        for (i, el) in top_elements.iter().enumerate() {
            let _ = writeln!(
                text_output,
                "{}. [{}] \"{}\"{} (id={})",
                i + 1,
                el.role,
                el.name,
                if !el.value.is_empty() { format!(" value={}", el.value) } else { "".to_string() },
                el.id
            );
        }

        // Save raw JSON to SHM
        let shm = SharedMemorySegment::new(shm_name);
        let raw_data = serde_json::to_vec(&top_elements).map_err(|e| e.to_string())?;
        
        shm.write(&raw_data).map_err(|e| e.to_string())?;

        // Emit event
        emit_event(EventType::SharedMemoryUpdate, serde_json::json!({
            "name": shm_name,
            "type": "ax_tree",
            "size": raw_data.len(),
        }));

        Ok(text_output)
    }

    fn extract_scored_elements(&self, node: &Value, depth: usize, elements: &mut Vec<ScoredElement>) {
        if depth >= self.max_depth {
            return;
        }

        let role = node.get("role").and_then(|v| v.as_str()).unwrap_or("unknown");
        let name = node.get("name").and_then(|v| v.as_str()).unwrap_or("");
        let value = node.get("value").and_then(|v| v.as_str()).unwrap_or("");
        let id = node.get("id").and_then(|v| v.as_u64()).unwrap_or(0) as usize;

        if !self.is_unimportant(role, name, value, node) {
            let mut score = self.calculate_score(role, name, value, node);
            
            // Penalty for depth
            score -= depth as i32;

            elements.push(ScoredElement {
                id,
                role: role.to_string(),
                name: name.to_string(),
                value: value.to_string(),
                score,
                x: 0, // In a real scenario we'd get coordinates from the node
                y: 0,
            });
        }

        if let Some(children) = node.get("children").and_then(|v| v.as_array()) {
            for child in children {
                self.extract_scored_elements(child, depth + 1, elements);
            }
        }
    }

    fn calculate_score(&self, role: &str, name: &str, _value: &str, _node: &Value) -> i32 {
        let mut score = 0;

        // Base scores for roles
        score += match role {
            "button" | "link" => 10,
            "textbox" | "searchbox" | "combobox" => 20, // High priority for inputs
            "checkbox" | "radio" => 15,
            "heading" => 5,
            _ => 0,
        };

        // Name heuristics
        let name_lower = name.to_lowercase();
        if name_lower.contains("search") || name_lower.contains("find") {
            score += 15;
        }
        if name_lower.contains("submit") || name_lower.contains("confirm") || name_lower.contains("login") {
            score += 12;
        }
        if name_lower.contains("close") || name_lower.contains("cancel") {
            score += 5;
        }

        // Noise reduction
        if name_lower.contains("ad") || name_lower.contains("advertisement") {
            score -= 50;
        }

        score
    }

    fn format_node(&self, node: &Value, depth: usize, output: &mut String) {
        if depth >= self.max_depth {
            return;
        }

        let role = node.get("role").and_then(|v| v.as_str()).unwrap_or("unknown");
        let name = node.get("name").and_then(|v| v.as_str()).unwrap_or("");
        let value = node.get("value").and_then(|v| v.as_str()).unwrap_or("");
        
        if self.filter_unimportant && self.is_unimportant(role, name, value, node) {
            if let Some(children) = node.get("children").and_then(|v| v.as_array()) {
                for child in children {
                    self.format_node(child, depth, output);
                }
            }
            return;
        }

        let indent = "  ".repeat(depth);
        write!(output, "{}[{}]", indent, role).unwrap();
        
        if !name.is_empty() {
            write!(output, " \"{}\"", name).unwrap();
        }
        
        if !value.is_empty() {
            write!(output, " value={}", value).unwrap();
        }

        self.append_states(node, output);
        output.push('\n');

        if let Some(children) = node.get("children").and_then(|v| v.as_array()) {
            for child in children {
                self.format_node(child, depth + 1, output);
            }
        }
    }

    fn append_states(&self, node: &Value, output: &mut String) {
        let states = [
            ("checked", "checked"),
            ("pressed", "pressed"),
            ("disabled", "disabled"),
            ("expanded", "expanded"),
            ("focused", "focused"),
            ("required", "required"),
            ("readonly", "readonly"),
        ];

        for (key, label) in states {
            if let Some(val) = node.get(key) {
                match val {
                    Value::Bool(true) => write!(output, " ({})", label).unwrap(),
                    Value::String(s) if s == "true" || s == "mixed" => {
                        write!(output, " ({})", label).unwrap()
                    }
                    _ => {}
                }
            }
        }

        if let Some(level) = node.get("level").and_then(|v| v.as_u64()) {
            write!(output, " L{}", level).unwrap();
        }
    }

    fn is_unimportant(&self, role: &str, name: &str, value: &str, node: &Value) -> bool {
        if matches!(role, "button" | "link" | "heading" | "textbox" | "checkbox" | "combobox" | "listbox" | "menuitem" | "searchbox") {
            return false;
        }

        let is_container = matches!(role, "generic" | "none" | "group" | "paragraph" | "listitem" | "list" | "section" | "article");
        
        if is_container && name.is_empty() && value.is_empty() {
            if node.get("focused") == Some(&Value::Bool(true)) {
                return false;
            }
            return true;
        }

        if role == "StaticText" && name.is_empty() {
            return true;
        }

        false
    }
}

