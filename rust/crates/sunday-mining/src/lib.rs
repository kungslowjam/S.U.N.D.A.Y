use scraper::{Html, Selector, ElementRef, Node};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DOMNode {
    pub id: usize,
    pub tag: String,
    pub attributes: HashMap<String, String>,
    pub text: Option<String>,
    pub children: Vec<DOMNode>,
}

pub struct DOMMiner {
    // Tags to completely ignore (including children)
    ignored_tags: Vec<&'static str>,
    // Attributes to preserve
    interesting_attrs: Vec<&'static str>,
}

impl DOMMiner {
    pub fn new() -> Self {
        Self {
            ignored_tags: vec!["script", "style", "head", "meta", "link", "noscript", "svg", "path"],
            interesting_attrs: vec!["id", "class", "role", "aria-label", "placeholder", "href", "title", "alt", "name", "value"],
        }
    }

    /// Build a hierarchical tree of "interesting" elements from HTML.
    pub fn extract_tree(&self, html: &str) -> Vec<DOMNode> {
        let fragment = Html::parse_fragment(html);
        let mut id_counter = 0;
        let mut root_nodes = Vec::new();

        for node in fragment.tree.root().children() {
            if let Some(dom_node) = self.process_node(node, &mut id_counter) {
                root_nodes.push(dom_node);
            }
        }

        root_nodes
    }

    fn process_node(&self, node: ego_tree::NodeRef<Node>, id_counter: &mut usize) -> Option<DOMNode> {
        match node.value() {
            Node::Element(el) => {
                let tag_name = el.name();
                
                // 1. Filter ignored tags
                if self.ignored_tags.contains(&tag_name) {
                    return None;
                }

                // 2. Extract interesting attributes
                let mut attributes = HashMap::new();
                for attr_name in &self.interesting_attrs {
                    if let Some(val) = el.attr(attr_name) {
                        if !val.is_empty() {
                            attributes.insert(attr_name.to_string(), val.to_string());
                        }
                    }
                }

                // 3. Process children recursively
                let mut children = Vec::new();
                for child in node.children() {
                    if let Some(child_node) = self.process_node(child, id_counter) {
                        children.push(child_node);
                    }
                }

                // 4. Optimization: If a node is a generic container (div/span) with no attributes and one child,
                // we can potentially flatten it. But for now, let's keep it simple and just prune empty leaves.
                if tag_name == "div" || tag_name == "span" || tag_name == "section" {
                    if attributes.is_empty() && children.is_empty() {
                        // Check if there's any text
                        let text = self.get_direct_text(node);
                        if text.is_empty() {
                            return None;
                        }
                    }
                }

                *id_counter += 1;
                Some(DOMNode {
                    id: *id_counter,
                    tag: tag_name.to_string(),
                    attributes,
                    text: Some(self.get_direct_text(node)).filter(|s| !s.is_empty()),
                    children,
                })
            }
            Node::Text(_) => None, // Text is handled by parent element via get_direct_text
            _ => None,
        }
    }

    fn get_direct_text(&self, node: ego_tree::NodeRef<Node>) -> String {
        node.children()
            .filter_map(|child| {
                if let Node::Text(t) = child.value() {
                    Some(t.to_string())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
            .trim()
            .chars()
            .take(200) // Truncate very long text
            .collect()
    }

    /// Formats the tree into a dense "Agent-readable" Markdown-like format.
    /// Uses indentation to show hierarchy without bulky JSON syntax.
    pub fn format_for_llm(&self, nodes: &[DOMNode], depth: usize) -> String {
        let mut output = String::new();
        let indent = "  ".repeat(depth);

        for node in nodes {
            let attr_str = node.attributes.iter()
                .map(|(k, v)| format!(" {}={}", k, v))
                .collect::<String>();

            let text_content = node.text.as_ref()
                .map(|t| format!(" \"{}\"", t))
                .unwrap_or_default();

            output.push_str(&format!("{indent}<{} id={}{}{}>\n", node.tag, node.id, attr_str, text_content));
            
            if !node.children.is_empty() {
                output.push_str(&self.format_for_llm(&node.children, depth + 1));
            }
        }
        output
    }
}

#[cfg(feature = "python")]
use pyo3::prelude::*;

#[cfg(feature = "python")]
#[pyclass(name = "NativeMiner")]
pub struct NativeMiner {
    inner: DOMMiner,
}

#[cfg(feature = "python")]
#[pymethods]
impl NativeMiner {
    #[new]
    fn new() -> Self {
        Self { inner: DOMMiner::new() }
    }

    /// Mines HTML and returns a compressed, hierarchical representation.
    /// This is significantly faster than doing the same in Python/Playwright.
    fn mine_html(&self, html: &str) -> PyResult<String> {
        let nodes = self.inner.extract_tree(html);
        Ok(self.inner.format_for_llm(&nodes, 0))
    }

    /// Similar to mine_html but returns raw JSON if needed.
    fn mine_json(&self, html: &str) -> PyResult<String> {
        let nodes = self.inner.extract_tree(html);
        serde_json::to_string(&nodes)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))
    }
}

#[cfg(feature = "python")]
#[pymodule]
fn sunday_mining(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<NativeMiner>()?;
    Ok(())
}
