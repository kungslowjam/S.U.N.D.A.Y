use serde_json::Value;

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

    pub fn process(&self, root: &Value) -> String {
        let mut output = String::new();
        self.format_node(root, 0, &mut output);
        output
    }

    fn format_node(&self, node: &Value, depth: usize, output: &mut String) {
        if depth >= self.max_depth {
            return;
        }

        let role = node.get("role").and_then(|v| v.as_str()).unwrap_or("unknown");
        let name = node.get("name").and_then(|v| v.as_str()).unwrap_or("");
        let value = node.get("value").and_then(|v| v.as_str()).unwrap_or("");
        
        // Semantic Filtering: Skip unimportant nodes if enabled
        if self.filter_unimportant && self.is_unimportant(role, name, value, node) {
            // But still process children if it's a generic container
            if let Some(children) = node.get("children").and_then(|v| v.as_array()) {
                for child in children {
                    self.format_node(child, depth, output); // Keep same depth for skipped containers
                }
            }
            return;
        }

        let indent = "  ".repeat(depth);
        output.push_str(&format!("{}[{}]", indent, role));
        
        if !name.is_empty() {
            output.push_str(&format!(" \"{}\"", name));
        }
        
        if !value.is_empty() {
            output.push_str(&format!(" value={}", value));
        }
        
        output.push('\n');

        if let Some(children) = node.get("children").and_then(|v| v.as_array()) {
            for child in children {
                self.format_node(child, depth + 1, output);
            }
        }
    }

    fn is_unimportant(&self, role: &str, name: &str, value: &str, _node: &Value) -> bool {
        // Roles that are usually just layout containers
        let is_container = matches!(role, "generic" | "none" | "group" | "paragraph" | "listitem" | "list");
        
        // If it's a container with no name or value, and it has children, it's likely just layout
        if is_container && name.is_empty() && value.is_empty() {
            return true;
        }

        // Static text with no name is redundant
        if role == "StaticText" && name.is_empty() {
            return true;
        }

        false
    }
}
