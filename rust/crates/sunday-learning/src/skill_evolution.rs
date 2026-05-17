//! SkillEvolutionEngine — closed-loop self-learning skill system.
//!
//! Inspired by Hermes Agent's background review mechanism:
//! 1. Read completed traces from TraceStore
//! 2. Mine recurring tool sequences via SkillDiscovery
//! 3. Generate SKILL.md manifests with performance metadata
//! 4. Write to disk (~/.sunday/skills/discovered/)
//! 5. Track skill usage outcomes and iterate/refine over time

use parking_lot::Mutex;
use rusqlite::Connection;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use sunday_core::{StepType, Trace, SUNDAYError};
use sunday_traces::TraceStore;

use crate::skill_discovery::{DiscoveredSkill, SkillDiscovery};

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug, thiserror::Error)]
pub enum EvolutionError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("SUNDAY error: {0}")]
    Sunday(#[from] SUNDAYError),
    #[error("TOML serialization error: {0}")]
    TomlSerialize(#[from] toml::ser::Error),
}

// ---------------------------------------------------------------------------
// SkillPerformanceTracker
// ---------------------------------------------------------------------------

/// Tracks every invocation of a skill and whether it succeeded.
pub struct SkillPerformanceTracker {
    conn: Mutex<Connection>,
}

#[derive(Debug, Clone, Default)]
pub struct SkillStats {
    pub total_uses: usize,
    pub success_count: usize,
    pub failure_count: usize,
    pub success_rate: f64,
    pub last_used_at: f64,
}

impl SkillPerformanceTracker {
    pub fn new(db_path: &Path) -> Result<Self, EvolutionError> {
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(db_path)?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS skill_outcomes (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                skill_name TEXT NOT NULL,
                trace_id TEXT NOT NULL,
                success INTEGER NOT NULL,
                used_at REAL DEFAULT 0
            );
            CREATE INDEX IF NOT EXISTS idx_skill_name ON skill_outcomes(skill_name);
            CREATE INDEX IF NOT EXISTS idx_trace_id ON skill_outcomes(trace_id);
            "
        )?;
        Ok(Self { conn: Mutex::new(conn) })
    }

    pub fn record(&self, name: &str, trace_id: &str, success: bool) -> Result<(), EvolutionError> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs_f64();
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO skill_outcomes (skill_name, trace_id, success, used_at)
             VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![name, trace_id, if success { 1 } else { 0 }, now],
        )?;
        Ok(())
    }

    pub fn stats(&self, name: &str) -> Result<SkillStats, EvolutionError> {
        let conn = self.conn.lock();
        let total: i64 = conn.query_row(
            "SELECT COUNT(*) FROM skill_outcomes WHERE skill_name = ?1",
            [name],
            |row| row.get(0),
        )?;
        let success: i64 = conn.query_row(
            "SELECT COUNT(*) FROM skill_outcomes WHERE skill_name = ?1 AND success = 1",
            [name],
            |row| row.get(0),
        )?;
        let last_used: f64 = conn.query_row(
            "SELECT COALESCE(MAX(used_at), 0) FROM skill_outcomes WHERE skill_name = ?1",
            [name],
            |row| row.get(0),
        )?;

        let total = total as usize;
        let success = success as usize;
        Ok(SkillStats {
            total_uses: total,
            success_count: success,
            failure_count: total.saturating_sub(success),
            success_rate: if total > 0 { success as f64 / total as f64 } else { 0.0 },
            last_used_at: last_used,
        })
    }

    pub fn all_stats(&self) -> Result<Vec<(String, SkillStats)>, EvolutionError> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT skill_name, COUNT(*), SUM(success), MAX(used_at)
             FROM skill_outcomes
             GROUP BY skill_name"
        )?;
        let rows = stmt.query_map([], |row| {
            let name: String = row.get(0)?;
            let total: i64 = row.get(1)?;
            let success: i64 = row.get(2)?;
            let last: f64 = row.get(3)?;
            let total = total as usize;
            let success = success as usize;
            Ok((name, SkillStats {
                total_uses: total,
                success_count: success,
                failure_count: total.saturating_sub(success),
                success_rate: if total > 0 { success as f64 / total as f64 } else { 0.0 },
                last_used_at: last,
            }))
        })?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(EvolutionError::Sqlite)
    }
}

// ---------------------------------------------------------------------------
// SkillEvolutionEngine
// ---------------------------------------------------------------------------

/// Result of iterating an existing skill.
#[derive(Debug, Clone, serde::Serialize)]
pub struct SkillIteration {
    pub name: String,
    pub previous_confidence: f64,
    pub new_confidence: f64,
    pub action: IterationAction,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub enum IterationAction {
    Upgraded,
    Merged { into: String },
    Kept,
    Deprecated,
}

pub struct SkillEvolutionEngine {
    store: Arc<TraceStore>,
    tracker: SkillPerformanceTracker,
    output_dir: PathBuf,
    min_freq: usize,
    min_len: usize,
    max_len: usize,
    min_outcome: f64,
    /// Highest trace_id (as started_at) we've processed.
    last_processed_watermark: f64,
}

impl SkillEvolutionEngine {
    pub fn new(
        store: Arc<TraceStore>,
        output_dir: impl AsRef<Path>,
    ) -> Result<Self, EvolutionError> {
        let output_dir = output_dir.as_ref().to_path_buf();
        std::fs::create_dir_all(&output_dir)?;

        let tracker_db = output_dir.parent()
            .map(|p| p.join("skill_evolution.db"))
            .unwrap_or_else(|| PathBuf::from("skill_evolution.db"));

        Ok(Self {
            store,
            tracker: SkillPerformanceTracker::new(&tracker_db)?,
            output_dir,
            min_freq: 3,
            min_len: 2,
            max_len: 6,
            min_outcome: 0.6,
            last_processed_watermark: -1.0,
        })
    }

    /// Configure discovery thresholds.
    pub fn with_thresholds(
        mut self,
        min_freq: usize,
        min_len: usize,
        max_len: usize,
        min_outcome: f64,
    ) -> Self {
        self.min_freq = min_freq;
        self.min_len = min_len;
        self.max_len = max_len;
        self.min_outcome = min_outcome;
        self
    }

    /// Process a batch of new traces, discover skills, write them to disk.
    ///
    /// Returns the list of newly discovered (or updated) skills.
    pub fn process_batch(&mut self, limit: usize) -> Result<Vec<DiscoveredSkill>, EvolutionError> {
        let traces = self.store.list_traces(limit, 0)?;
        if traces.is_empty() {
            return Ok(Vec::new());
        }

        let new_traces: Vec<Trace> = traces
            .into_iter()
            .filter(|t| t.started_at > self.last_processed_watermark)
            .collect();

        if new_traces.is_empty() {
            return Ok(Vec::new());
        }

        // Update watermark to the newest trace we processed
        let max_ts = new_traces.iter().map(|t| t.started_at).fold(0.0_f64, f64::max);
        if max_ts > 0.0 {
            self.last_processed_watermark = max_ts;
        }

        // Extract (tool_sequence, outcome_score, query) from each trace
        let mut inputs: Vec<(Vec<String>, f64, String)> = Vec::new();
        for trace in &new_traces {
            let tool_seq: Vec<String> = trace
                .steps
                .iter()
                .filter(|s| s.step_type == StepType::ToolCall)
                .filter_map(|s| {
                    s.metadata
                        .get("tool_name")
                        .and_then(|v| v.as_str())
                        .map(String::from)
                })
                .collect();

            if tool_seq.len() < self.min_len {
                continue;
            }

            let outcome = Self::outcome_score(trace);
            inputs.push((tool_seq, outcome, trace.query.clone()));
        }

        if inputs.is_empty() {
            return Ok(Vec::new());
        }

        let mut discovery = SkillDiscovery::new(self.min_freq, self.min_len, self.max_len, self.min_outcome);
        let discovered = discovery.analyze(&inputs).to_vec();

        // Write each discovered skill to disk
        let mut written = Vec::new();
        for skill in &discovered {
            if let Err(e) = self.write_skill(skill) {
                tracing::warn!("Failed to write skill {}: {}", skill.name, e);
            } else {
                written.push(skill.clone());
            }
        }

        Ok(written)
    }

    /// Record that a skill was used and whether it succeeded.
    pub fn record_skill_usage(&self, name: &str, trace_id: &str, success: bool) -> Result<(), EvolutionError> {
        self.tracker.record(name, trace_id, success)
    }

    /// Process all pending traces in repeated batches until caught up.
    pub fn process_all_pending(&mut self, batch_size: usize) -> Result<Vec<DiscoveredSkill>, EvolutionError> {
        let mut all_discovered = Vec::new();
        loop {
            let batch = self.process_batch(batch_size)?;
            if batch.is_empty() {
                break;
            }
            all_discovered.extend(batch);
        }
        Ok(all_discovered)
    }

    /// Re-evaluate existing discovered skills using performance data.
    ///
    /// - Upgrades confidence on skills with high success_rate
    /// - Deprecates skills with very low success_rate (< 0.3)
    /// - Merges near-duplicate skills (same tool_sequence, different names)
    pub fn iterate_skills(&self) -> Result<Vec<SkillIteration>, EvolutionError> {
        let mut iterations = Vec::new();
        let stats = self.tracker.all_stats()?;

        // Deprecate poorly performing skills
        for (name, stat) in &stats {
            if stat.success_rate < 0.3 && stat.total_uses >= 5 {
                iterations.push(SkillIteration {
                    name: name.clone(),
                    previous_confidence: 0.5,
                    new_confidence: 0.1,
                    action: IterationAction::Deprecated,
                });
                if let Err(e) = self.deprecate_skill(name) {
                    tracing::warn!("Failed to deprecate skill {}: {}", name, e);
                }
            }
        }

        // Merge duplicate skills (same tool_sequence)
        let manifests = self.load_existing_manifests()?;
        let mut seq_to_best: HashMap<Vec<String>, (String, f64)> = HashMap::new();

        for (name, manifest) in &manifests {
            let seq: Vec<String> = manifest.steps.iter().map(|s| s.tool_name.clone()).collect();
            let confidence = manifest
                .metadata
                .get("confidence")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.5);

            match seq_to_best.get(&seq) {
                Some((best_name, best_conf)) if confidence > *best_conf => {
                    seq_to_best.insert(seq.clone(), (name.clone(), confidence));
                }
                None => {
                    seq_to_best.insert(seq.clone(), (name.clone(), confidence));
                }
                _ => {}
            }
        }

        // Identify duplicates to merge
        let mut seen_seqs: HashMap<Vec<String>, String> = HashMap::new();
        for (name, manifest) in &manifests {
            let seq: Vec<String> = manifest.steps.iter().map(|s| s.tool_name.clone()).collect();
            if let Some(best_name) = seen_seqs.get(&seq) {
                if best_name != name {
                    iterations.push(SkillIteration {
                        name: name.clone(),
                        previous_confidence: 0.5,
                        new_confidence: 0.5,
                        action: IterationAction::Merged { into: best_name.clone() },
                    });
                    let _ = self.remove_skill_dir(name);
                    continue;
                }
            } else {
                seen_seqs.insert(seq, name.clone());
            }
        }

        Ok(iterations)
    }

    // ------------------------------------------------------------------
    // Helpers
    // ------------------------------------------------------------------

    fn outcome_score(trace: &Trace) -> f64 {
        match trace.outcome.as_deref() {
            Some("success") => 1.0,
            Some("failure") => 0.0,
            _ => trace.feedback.unwrap_or(0.5),
        }
    }

    fn write_skill(&self, skill: &DiscoveredSkill) -> Result<(), EvolutionError> {
        let dir = self.output_dir.join(&skill.name);
        std::fs::create_dir_all(&dir)?;

        let path = dir.join("SKILL.md");

        // Check if existing skill has higher confidence
        if path.exists() {
            let existing = std::fs::read_to_string(&path)?;
            if let Some(existing_conf) = Self::extract_confidence(&existing) {
                if existing_conf >= skill.avg_outcome {
                    tracing::debug!("Skipping skill {} — existing confidence {:.2} >= {:.2}",
                        skill.name, existing_conf, skill.avg_outcome);
                    return Ok(());
                }
            }
        }

        let content = Self::generate_skill_md(skill);
        std::fs::write(&path, content)?;
        tracing::info!("Wrote skill {} to {}", skill.name, path.display());
        Ok(())
    }

    fn generate_skill_md(skill: &DiscoveredSkill) -> String {
        let mut lines = vec![
            "---".to_string(),
            format!("name: {}", skill.name),
            format!("description: {}", skill.description),
            "version: 0.1.0".to_string(),
            "author: sunday (auto-discovered)".to_string(),
            format!("tags: [\"auto-discovered\", \"evolved\"]"),
            "metadata:".to_string(),
            format!("  auto_discovered: true"),
            format!("  confidence: {:.4}", skill.avg_outcome),
            format!("  frequency: {}", skill.frequency),
            format!("  iteration_count: 1"),
        ];

        if !skill.example_inputs.is_empty() {
            lines.push("  example_inputs:".to_string());
            for ex in &skill.example_inputs {
                lines.push(format!("    - \"{}\"", ex.replace('"', "\\\"")));
            }
        }

        lines.push("---".to_string());
        lines.push("".to_string());
        lines.push("## Workflow".to_string());
        lines.push("".to_string());

        for (i, tool) in skill.tool_sequence.iter().enumerate() {
            lines.push(format!("{}. Run `{}`", i + 1, tool));
        }

        lines.push("".to_string());
        lines.push("## When to Use".to_string());
        lines.push("".to_string());
        lines.push(format!(
            "Use this skill when you need to perform the sequence: {}",
            skill.tool_sequence.join(" → ")
        ));

        lines.join("\n")
    }

    fn extract_confidence(content: &str) -> Option<f64> {
        for line in content.lines() {
            if line.trim_start().starts_with("confidence:") {
                return line.split(':').nth(1)?.trim().parse().ok();
            }
        }
        None
    }

    fn deprecate_skill(&self, name: &str) -> Result<(), EvolutionError> {
        let path = self.output_dir.join(name).join("SKILL.md");
        if !path.exists() {
            return Ok(());
        }
        let content = std::fs::read_to_string(&path)?;
        let deprecated = content
            .replace("auto_discovered: true", "auto_discovered: true\n  deprecated: true");
        std::fs::write(&path, deprecated)?;
        Ok(())
    }

    fn remove_skill_dir(&self, name: &str) -> Result<(), EvolutionError> {
        let dir = self.output_dir.join(name);
        if dir.exists() {
            std::fs::remove_dir_all(&dir)?;
        }
        Ok(())
    }

    fn load_existing_manifests(&self) -> Result<HashMap<String, sunday_skills::SkillManifest>, EvolutionError> {
        let mut result = HashMap::new();
        if !self.output_dir.exists() {
            return Ok(result);
        }
        for entry in std::fs::read_dir(&self.output_dir)? {
            let entry = entry?;
            if !entry.file_type()?.is_dir() {
                continue;
            }
            let md_path = entry.path().join("SKILL.md");
            if !md_path.exists() {
                continue;
            }
            // Parse YAML frontmatter only (sunday-skills uses TOML, so parse manually)
            let raw = std::fs::read_to_string(&md_path)?;
            if let Some(name) = Self::extract_frontmatter_field(&raw, "name") {
                // Build a minimal manifest from the markdown
                let steps = Self::extract_steps_from_md(&raw);
                let manifest = sunday_skills::SkillManifest {
                    name: name.clone(),
                    version: Self::extract_frontmatter_field(&raw, "version").unwrap_or_else(|| "0.1.0".into()),
                    description: Self::extract_frontmatter_field(&raw, "description").unwrap_or_default(),
                    author: "sunday".into(),
                    steps,
                    required_capabilities: Vec::new(),
                    signature: String::new(),
                    metadata: HashMap::new(),
                    tags: Vec::new(),
                    depends: Vec::new(),
                    user_invocable: true,
                    disable_model_invocation: false,
                    markdown_content: raw.clone(),
                };
                result.insert(name, manifest);
            }
        }
        Ok(result)
    }

    fn extract_frontmatter_field(content: &str, field: &str) -> Option<String> {
        let in_frontmatter = content.starts_with("---");
        if !in_frontmatter {
            return None;
        }
        let after_start = &content[3..];
        let end_idx = after_start.find("\n---")?;
        let yaml_block = &after_start[..end_idx];
        for line in yaml_block.lines() {
            let trimmed = line.trim_start();
            if trimmed.starts_with(&format!("{}:", field)) {
                return Some(trimmed.splitn(2, ':').nth(1)?.trim().trim_matches('"').to_string());
            }
        }
        None
    }

    fn extract_steps_from_md(content: &str) -> Vec<sunday_skills::SkillStep> {
        let mut steps = Vec::new();
        let mut in_workflow = false;
        for line in content.lines() {
            if line.trim() == "## Workflow" {
                in_workflow = true;
                continue;
            }
            if in_workflow && line.starts_with("## ") {
                break;
            }
            if in_workflow {
                // Match "1. Run `tool_name`"
                if let Some(backtick_start) = line.find('`') {
                    if let Some(backtick_end) = line[backtick_start + 1..].find('`') {
                        let tool = &line[backtick_start + 1..backtick_start + 1 + backtick_end];
                        steps.push(sunday_skills::SkillStep {
                            tool_name: tool.to_string(),
                            skill_name: String::new(),
                            arguments_template: "{}".to_string(),
                            output_key: String::new(),
                        });
                    }
                }
            }
        }
        steps
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use sunday_core::{Trace, TraceStep, StepType};
    use std::collections::HashMap;

    fn make_trace(query: &str, tools: Vec<&str>, outcome: &str) -> Trace {
        let steps: Vec<TraceStep> = tools
            .into_iter()
            .map(|t| TraceStep {
                step_type: StepType::ToolCall,
                timestamp: 0.0,
                duration_seconds: 0.0,
                input: HashMap::new(),
                output: HashMap::new(),
                metadata: {
                    let mut m = HashMap::new();
                    m.insert("tool_name".to_string(), serde_json::json!(t));
                    m
                },
            })
            .collect();

        Trace {
            query: query.to_string(),
            outcome: Some(outcome.to_string()),
            steps,
            ..Default::default()
        }
    }

    #[test]
    fn test_process_batch_discovers_skills() {
        let store = Arc::new(TraceStore::in_memory().unwrap());
        let tmp = tempfile::tempdir().unwrap();
        let mut engine = SkillEvolutionEngine::new(store.clone(), tmp.path()).unwrap();

        // Seed 3 similar traces
        for _ in 0..3 {
            let t = make_trace("research", vec!["web_search", "file_write"], "success");
            store.save(&t).unwrap();
        }

        let discovered = engine.process_batch(100).unwrap();
        assert!(!discovered.is_empty(), "should discover at least one skill");

        let skill_dir = tmp.path().join(&discovered[0].name);
        assert!(skill_dir.join("SKILL.md").exists(), "SKILL.md should be written");
    }

    #[test]
    fn test_performance_tracker() {
        let tmp = tempfile::tempdir().unwrap();
        let tracker = SkillPerformanceTracker::new(&tmp.path().join("tracker.db")).unwrap();

        tracker.record("alpha", "t1", true).unwrap();
        tracker.record("alpha", "t2", true).unwrap();
        tracker.record("alpha", "t3", false).unwrap();

        let stats = tracker.stats("alpha").unwrap();
        assert_eq!(stats.total_uses, 3);
        assert_eq!(stats.success_count, 2);
        assert!((stats.success_rate - 0.666).abs() < 0.01);
    }

    #[test]
    fn test_skill_md_generation() {
        let skill = DiscoveredSkill {
            name: "web-research".to_string(),
            description: "Search and save".to_string(),
            tool_sequence: vec!["web_search".into(), "file_write".into()],
            frequency: 5,
            avg_outcome: 0.85,
            example_inputs: vec!["find rust tips".into()],
        };

        let md = SkillEvolutionEngine::generate_skill_md(&skill);
        assert!(md.contains("name: web-research"));
        assert!(md.contains("confidence: 0.8500"));
        assert!(md.contains("## Workflow"));
        assert!(md.contains("Run `web_search`"));
    }
}
