//! User modeling / peer memory system — inspired by Honcho's entity-centric memory.
//!
//! Stores structured conclusions about users (preferences, patterns, expertise)
//! extracted from conversations. Supports multi-level reasoning:
//! - Explicit: directly stated facts
//! - Deductive: logically derived
//! - Inductive: pattern-based generalizations

use std::collections::HashMap;
use once_cell::sync::Lazy;
use regex::Regex;

/// Types of conclusions with different certainty levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConclusionLevel {
    Explicit,   // Directly stated by user
    Deductive,  // Logically derived
    Inductive,  // Pattern-based generalization
}

impl ConclusionLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            ConclusionLevel::Explicit => "explicit",
            ConclusionLevel::Deductive => "deductive",
            ConclusionLevel::Inductive => "inductive",
        }
    }
}

/// A single conclusion about a user
#[derive(Debug, Clone)]
pub struct Conclusion {
    pub category: String,       // e.g., "preference", "expertise", "pattern"
    pub key: String,            // e.g., "programming_language"
    pub value: String,          // e.g., "Python"
    pub level: ConclusionLevel,
    pub confidence: f64,        // 0.0 - 1.0
    pub session_count: usize,   // how many sessions support this
    pub last_seen: u64,         // timestamp
}

/// User model — structured representation of a peer
#[derive(Debug, Clone, Default)]
pub struct UserModel {
    pub peer_id: String,
    pub conclusions: Vec<Conclusion>,
    pub session_count: usize,
    pub first_seen: u64,
    pub last_seen: u64,
}

impl UserModel {
    pub fn new(peer_id: String) -> Self {
        let now = current_timestamp();
        Self {
            peer_id,
            conclusions: Vec::new(),
            session_count: 0,
            first_seen: now,
            last_seen: now,
        }
    }

    /// Add or update a conclusion
    pub fn add_conclusion(&mut self, mut conclusion: Conclusion) {
        // Update timestamp
        self.last_seen = current_timestamp();
        conclusion.last_seen = self.last_seen;

        // Check if we already have this conclusion
        if let Some(existing) = self.conclusions.iter_mut().find(|c| {
            c.category == conclusion.category && c.key == conclusion.key
        }) {
            // Merge: boost confidence if consistent, reduce if conflicting
            if existing.value == conclusion.value {
                existing.confidence = (existing.confidence + conclusion.confidence).min(1.0);
                existing.session_count += 1;
                existing.last_seen = conclusion.last_seen;
                // Upgrade level if possible
                if conclusion.level as usize > existing.level as usize {
                    existing.level = conclusion.level;
                }
            } else {
                // Conflicting — reduce confidence of old, add new with lower confidence
                existing.confidence *= 0.7;
                conclusion.confidence *= 0.5;
                self.conclusions.push(conclusion);
            }
        } else {
            self.conclusions.push(conclusion);
        }
    }

    /// Get conclusions by category
    pub fn get_by_category(&self, category: &str) -> Vec<&Conclusion> {
        self.conclusions
            .iter()
            .filter(|c| c.category == category)
            .collect()
    }

    /// Get conclusions above confidence threshold
    pub fn get_confident(&self, threshold: f64) -> Vec<&Conclusion> {
        self.conclusions
            .iter()
            .filter(|c| c.confidence >= threshold)
            .collect()
    }

    /// Generate a user profile summary for prompt injection
    pub fn to_prompt_context(&self, max_chars: usize) -> String {
        let mut lines = vec![
            format!("## User Profile: {}", self.peer_id),
            format!("Sessions: {} | First seen: {} | Last active: {}",
                self.session_count, self.first_seen, self.last_seen),
            "".to_string(),
        ];

        // Group by category
        let mut by_category: HashMap<String, Vec<&Conclusion>> = HashMap::new();
        for c in &self.conclusions {
            by_category.entry(c.category.clone()).or_default().push(c);
        }

        for (category, items) in by_category {
            lines.push(format!("### {}", category));
            for c in items.iter().filter(|x| x.confidence >= 0.5) {
                lines.push(format!("- {} = {} ({}%, {})",
                    c.key, c.value,
                    (c.confidence * 100.0) as i32,
                    c.level.as_str(),
                ));
            }
            lines.push("".to_string());
        }

        let text = lines.join("\n");
        if text.len() > max_chars {
            text[..max_chars].to_string() + "\n..."
        } else {
            text
        }
    }
}

/// Extract conclusions from a single message/conversation turn
pub fn extract_conclusions(text: &str) -> Vec<Conclusion> {
    let mut conclusions = Vec::new();
    let lower = text.to_lowercase();

    // Preference patterns
    static PREF_RE: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"(?i)(?:i (?:like|prefer|love|enjoy|hate|dislike)|my favorite|i always|i never|i usually)\s+(.+?)(?:[.!;]|\z)").unwrap()
    });
    for cap in PREF_RE.captures_iter(text) {
        if let Some(m) = cap.get(1) {
            conclusions.push(Conclusion {
                category: "preference".to_string(),
                key: "general".to_string(),
                value: m.as_str().trim().to_string(),
                level: ConclusionLevel::Explicit,
                confidence: 0.8,
                session_count: 1,
                last_seen: current_timestamp(),
            });
        }
    }

    // Expertise patterns
    static EXPERT_RE: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"(?i)(?:i (?:know|use|work with|am familiar with|have experience with|specialize in))\s+(.+?)(?:[.!;]|\z)").unwrap()
    });
    for cap in EXPERT_RE.captures_iter(text) {
        if let Some(m) = cap.get(1) {
            conclusions.push(Conclusion {
                category: "expertise".to_string(),
                key: "domain".to_string(),
                value: m.as_str().trim().to_string(),
                level: ConclusionLevel::Explicit,
                confidence: 0.75,
                session_count: 1,
                last_seen: current_timestamp(),
            });
        }
    }

    // Goal patterns
    static GOAL_RE: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"(?i)(?:i want to|i need to|my goal is|i'm trying to|i aim to)\s+(.+?)(?:[.!;]|\z)").unwrap()
    });
    for cap in GOAL_RE.captures_iter(text) {
        if let Some(m) = cap.get(1) {
            conclusions.push(Conclusion {
                category: "goal".to_string(),
                key: "current".to_string(),
                value: m.as_str().trim().to_string(),
                level: ConclusionLevel::Explicit,
                confidence: 0.7,
                session_count: 1,
                last_seen: current_timestamp(),
            });
        }
    }

    // Communication style patterns (inductive)
    if lower.contains("please") && lower.contains("thank") {
        conclusions.push(Conclusion {
            category: "communication_style".to_string(),
            key: "formality".to_string(),
            value: "polite/formal".to_string(),
            level: ConclusionLevel::Inductive,
            confidence: 0.6,
            session_count: 1,
            last_seen: current_timestamp(),
        });
    }
    if text.chars().filter(|c| *c == '!').count() > 2 {
        conclusions.push(Conclusion {
            category: "communication_style".to_string(),
            key: "tone".to_string(),
            value: "enthusiastic".to_string(),
            level: ConclusionLevel::Inductive,
            confidence: 0.5,
            session_count: 1,
            last_seen: current_timestamp(),
        });
    }

    conclusions
}

/// In-memory store for user models
#[derive(Debug, Clone, Default)]
pub struct UserModelStore {
    models: HashMap<String, UserModel>,
}

impl UserModelStore {
    pub fn new() -> Self {
        Self {
            models: HashMap::new(),
        }
    }

    pub fn get_or_create(&mut self, peer_id: &str) -> &mut UserModel {
        self.models.entry(peer_id.to_string()).or_insert_with(|| {
            UserModel::new(peer_id.to_string())
        })
    }

    pub fn get(&self, peer_id: &str) -> Option<&UserModel> {
        self.models.get(peer_id)
    }

    pub fn process_message(&mut self, peer_id: &str, message: &str) {
        let model = self.get_or_create(peer_id);
        model.session_count += 1;
        for conclusion in extract_conclusions(message) {
            model.add_conclusion(conclusion);
        }
    }

    pub fn get_prompt_context(&self, peer_id: &str, max_chars: usize) -> Option<String> {
        self.models.get(peer_id).map(|m| m.to_prompt_context(max_chars))
    }

    pub fn all_peer_ids(&self) -> Vec<String> {
        self.models.keys().cloned().collect()
    }
}

fn current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
