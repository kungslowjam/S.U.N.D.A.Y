//! Autonomous skill creation — analyze conversation context and generate skill manifests.
//!
//! Inspired by Hermes Agent's background review mechanism:
//! - Track tool-call count per session
//! - Trigger review when threshold reached
//! - Generate SKILL.md-compatible manifests from successful patterns

use regex::Regex;
use once_cell::sync::Lazy;
use std::collections::HashMap;

/// Threshold for triggering skill review (Hermes uses 10)
pub const DEFAULT_SKILL_NUDGE_INTERVAL: usize = 10;

/// Minimum tool calls to consider a task "complex"
pub const MIN_COMPLEX_TOOL_CALLS: usize = 5;

/// Detect success indicators in text
static SUCCESS_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)(?:success|completed|done|finished|result|output|answer)")
        .unwrap()
});

/// Detect error/failure indicators
static ERROR_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)(?:error|failed|failure|exception|timeout|unable|cannot)")
        .unwrap()
});

/// Session tracker for skill auto-creation triggers
#[derive(Debug, Default)]
pub struct SkillAutoCreator {
    tool_call_count: usize,
    nudge_interval: usize,
    last_reviewed_count: usize,
}

impl SkillAutoCreator {
    pub fn new(nudge_interval: usize) -> Self {
        Self {
            tool_call_count: 0,
            nudge_interval,
            last_reviewed_count: 0,
        }
    }

    /// Record a tool call and return true if review should trigger
    pub fn record_tool_call(&mut self) -> bool {
        self.tool_call_count += 1;
        if self.tool_call_count - self.last_reviewed_count >= self.nudge_interval {
            self.last_reviewed_count = self.tool_call_count;
            true
        } else {
            false
        }
    }

    /// Reset counter (e.g., on new session)
    pub fn reset(&mut self) {
        self.tool_call_count = 0;
        self.last_reviewed_count = 0;
    }

    pub fn tool_call_count(&self) -> usize {
        self.tool_call_count
    }
}

/// Analyze a conversation block to determine if it contains a skill-worthy pattern
pub fn analyze_conversation_for_skill(
    conversation: &str,
    tool_sequence: &[String],
) -> Option<SkillCandidate> {
    // Need enough tool calls
    if tool_sequence.len() < MIN_COMPLEX_TOOL_CALLS {
        return None;
    }

    // Check success vs error ratio
    let success_count = SUCCESS_RE.find_iter(conversation).count();
    let error_count = ERROR_RE.find_iter(conversation).count();

    // Must be mostly successful
    if error_count > success_count {
        return None;
    }

    // Must have a clear outcome
    if success_count == 0 {
        return None;
    }

    // Generate skill name from tool sequence
    let name = generate_skill_name(tool_sequence);
    let description = generate_skill_description(tool_sequence, success_count > error_count * 2);

    Some(SkillCandidate {
        name,
        description,
        tool_sequence: tool_sequence.to_vec(),
        confidence: calculate_confidence(tool_sequence.len(), success_count, error_count),
    })
}

/// A candidate skill discovered from conversation analysis
#[derive(Debug, Clone)]
pub struct SkillCandidate {
    pub name: String,
    pub description: String,
    pub tool_sequence: Vec<String>,
    pub confidence: f64,
}

/// Generate a kebab-case skill name from tool sequence
fn generate_skill_name(tools: &[String]) -> String {
    if tools.len() >= 3 {
        format!("{}-{}-workflow", tools[0], tools[tools.len() - 1])
    } else if tools.len() == 2 {
        format!("{}-then-{}", tools[0], tools[1])
    } else {
        format!("{}-task", tools[0])
    }
    .to_lowercase()
    .replace('_', "-")
}

/// Generate a human-readable description
fn generate_skill_description(tools: &[String], high_confidence: bool) -> String {
    let seq = tools.join(" → ");
    let quality = if high_confidence {
        "reliable"
    } else {
        "common"
    };
    format!(
        "Auto-discovered {quality} workflow: {seq} ({} steps). \
         Use this skill when you need to perform a multi-step operation \
         involving these tools in sequence.",
        tools.len()
    )
}

/// Calculate confidence score (0.0 - 1.0)
fn calculate_confidence(tool_count: usize, success: usize, error: usize) -> f64 {
    let base = (tool_count as f64 / 10.0).min(1.0) * 0.5;
    let success_ratio = if success + error > 0 {
        success as f64 / (success + error) as f64
    } else {
        0.5
    };
    base + success_ratio * 0.5
}

/// Generate a SKILL.md-compatible manifest
pub fn generate_skill_manifest(candidate: &SkillCandidate) -> String {
    let mut lines = vec![
        "---".to_string(),
        format!("name: {}", candidate.name),
        format!("description: {}", candidate.description),
        "version: 0.1.0".to_string(),
        format!("metadata:"),
        format!("  auto_created: true"),
        format!("  confidence: {:.2}", candidate.confidence),
        format!("  tool_count: {}", candidate.tool_sequence.len()),
        "---".to_string(),
        "".to_string(),
        "## Workflow".to_string(),
        "".to_string(),
    ];

    for (i, tool) in candidate.tool_sequence.iter().enumerate() {
        lines.push(format!("{}. Run `{}`", i + 1, tool));
    }

    lines.push("".to_string());
    lines.push("## When to Use".to_string());
    lines.push("".to_string());
    lines.push(format!(
        "Use this skill when you need to perform the sequence: {}",
        candidate.tool_sequence.join(" → ")
    ));

    lines.join("\n")
}

/// Batch analyze multiple conversations for skill candidates
pub fn batch_analyze(
    conversations: &[(String, Vec<String>)],  // (conversation_text, tool_sequence)
) -> Vec<SkillCandidate> {
    let mut candidates: Vec<SkillCandidate> = Vec::new();
    let mut seen_names: HashMap<String, usize> = HashMap::new();

    for (text, tools) in conversations {
        if let Some(candidate) = analyze_conversation_for_skill(text, tools) {
            // Deduplicate by name, keep highest confidence
            let entry = seen_names.entry(candidate.name.clone()).or_insert(candidates.len());
            if *entry < candidates.len() && candidates[*entry].confidence < candidate.confidence {
                candidates[*entry] = candidate;
            } else if *entry == candidates.len() {
                candidates.push(candidate);
            }
        }
    }

    // Sort by confidence descending
    candidates.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());
    candidates
}
