//! Apply-patch tool — apply unified diff patches to files.

use crate::traits::BaseTool;
use sunday_core::{SUNDAYError, ToolResult, ToolSpec};
use sunday_security::file_policy::is_sensitive_file;
use once_cell::sync::Lazy;
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;

static SPEC: Lazy<ToolSpec> = Lazy::new(|| ToolSpec {
    name: "apply_patch".into(),
    description: "Apply a unified diff patch to a file. Supports standard unified diff format with context lines, additions, and removals.".into(),
    parameters: serde_json::json!({
        "type": "object",
        "properties": {
            "patch": { "type": "string", "description": "The unified diff patch text to apply." },
            "path": { "type": "string", "description": "Target file path. If omitted, auto-detected from the patch +++ header." },
            "backup": { "type": "boolean", "description": "Create a .bak backup before applying (default: true)." }
        },
        "required": ["patch"]
    }),
    category: "filesystem".into(),
    cost_estimate: 0.0,
    latency_estimate: 0.0,
    requires_confirmation: false,
    timeout_seconds: 10.0,
    required_capabilities: vec!["file:write".into()],
    metadata: HashMap::new(),
});

pub struct ApplyPatchTool;

#[derive(Debug)]
struct Hunk {
    old_start: usize,
    #[allow(dead_code)]
    old_count: usize,
    #[allow(dead_code)]
    new_start: usize,
    #[allow(dead_code)]
    new_count: usize,
    lines: Vec<String>,
}

fn parse_patch(patch_text: &str) -> Result<(Option<String>, Vec<Hunk>), String> {
    let mut target_path: Option<String> = None;
    let mut hunks: Vec<Hunk> = Vec::new();

    for raw_line in patch_text.lines() {
        let line = raw_line.trim_end_matches('\r');

        if line.starts_with("+++ ") {
            let path_part = line[4..].trim();
            let path_part = if path_part.starts_with("b/") {
                &path_part[2..]
            } else {
                path_part
            };
            if path_part != "/dev/null" {
                target_path = Some(path_part.to_string());
            }
            continue;
        }

        if line.starts_with("--- ") {
            continue;
        }

        if let Some(caps) = regex_hunk_header(line) {
            let current_hunk = Hunk {
                old_start: caps.0,
                old_count: caps.1,
                new_start: caps.2,
                new_count: caps.3,
                lines: Vec::new(),
            };
            hunks.push(current_hunk);
            continue;
        }

        if let Some(ref mut hunk) = hunks.last_mut() {
            if line.starts_with(' ') || line.starts_with('+') || line.starts_with('-') {
                hunk.lines.push(line.to_string());
            } else if line == "\\ No newline at end of file" {
                continue;
            } else if line.is_empty() {
                hunk.lines.push(" ".to_string());
            }
        }
    }

    if hunks.is_empty() {
        return Err("No hunks found in patch".into());
    }

    Ok((target_path, hunks))
}

fn regex_hunk_header(line: &str) -> Option<(usize, usize, usize, usize)> {
    // @@ -old_start[,old_count] +new_start[,new_count] @@
    if !line.starts_with("@@ -") {
        return None;
    }
    let rest = &line[4..];
    let space_idx = rest.find(" @@")?;
    let inner = &rest[..space_idx];
    let parts: Vec<&str> = inner.split(' ').collect();
    if parts.len() != 2 {
        return None;
    }
    let old_part = parts[0].strip_prefix('-')?;
    let new_part = parts[1].strip_prefix('+')?;

    let (old_start, old_count) = parse_range(old_part)?;
    let (new_start, new_count) = parse_range(new_part)?;
    Some((old_start, old_count, new_start, new_count))
}

fn parse_range(s: &str) -> Option<(usize, usize)> {
    if let Some((start, count)) = s.split_once(',') {
        Some((start.parse().ok()?, count.parse().ok()?))
    } else {
        let start: usize = s.parse().ok()?;
        Some((start, 1))
    }
}

fn apply_hunks(original: &str, hunks: &[Hunk]) -> Result<String, String> {
    let mut orig_lines: Vec<String> = original.lines().map(|l| l.to_string()).collect();
    let mut offset: isize = 0;

    for (hunk_idx, hunk) in hunks.iter().enumerate() {
        let pos = (hunk.old_start as isize - 1 + offset) as usize;
        let mut new_lines: Vec<String> = Vec::new();
        let mut check_pos = pos;

        for diff_line in &hunk.lines {
            if diff_line.is_empty() {
                continue;
            }
            let tag = diff_line.chars().next().unwrap();
            let content = &diff_line[1..];

            match tag {
                ' ' => {
                    if check_pos >= orig_lines.len() {
                        return Err(format!(
                            "Hunk {}: context line beyond end of file (line {})",
                            hunk_idx + 1,
                            check_pos + 1
                        ));
                    }
                    let orig_content = orig_lines[check_pos].trim_end_matches('\r');
                    if orig_content != content {
                        return Err(format!(
                            "Hunk {}: context mismatch at line {}: expected {:?}, got {:?}",
                            hunk_idx + 1,
                            check_pos + 1,
                            content,
                            orig_content
                        ));
                    }
                    new_lines.push(orig_lines[check_pos].clone());
                    check_pos += 1;
                }
                '-' => {
                    if check_pos >= orig_lines.len() {
                        return Err(format!(
                            "Hunk {}: removal line beyond end of file (line {})",
                            hunk_idx + 1,
                            check_pos + 1
                        ));
                    }
                    let orig_content = orig_lines[check_pos].trim_end_matches('\r');
                    if orig_content != content {
                        return Err(format!(
                            "Hunk {}: removal mismatch at line {}: expected {:?}, got {:?}",
                            hunk_idx + 1,
                            check_pos + 1,
                            content,
                            orig_content
                        ));
                    }
                    check_pos += 1;
                }
                '+' => {
                    new_lines.push(content.to_string());
                }
                _ => {}
            }
        }

        let consumed = check_pos - pos;
        orig_lines.splice(pos..pos + consumed, new_lines.clone());
        offset += new_lines.len() as isize - consumed as isize;
    }

    Ok(orig_lines.join("\n"))
}

impl BaseTool for ApplyPatchTool {
    fn tool_id(&self) -> &str {
        "apply_patch"
    }
    fn spec(&self) -> &ToolSpec {
        &SPEC
    }
    fn execute(&self, params: &Value) -> Result<ToolResult, SUNDAYError> {
        let patch_text = params["patch"].as_str().unwrap_or("");
        if patch_text.is_empty() {
            return Ok(ToolResult::failure("apply_patch", "No patch provided."));
        }

        let (header_path, hunks) = match parse_patch(patch_text) {
            Ok(v) => v,
            Err(e) => return Ok(ToolResult::failure("apply_patch", format!("Malformed patch: {}", e))),
        };

        let target = params["path"].as_str().or_else(|| header_path.as_deref());
        let target = match target {
            Some(t) if !t.is_empty() => t,
            _ => return Ok(ToolResult::failure(
                "apply_patch",
                "No target path provided and could not auto-detect from patch header."
            )),
        };

        let path = Path::new(target);

        if is_sensitive_file(path) {
            return Ok(ToolResult::failure(
                "apply_patch",
                format!("Access denied: {} is a sensitive file.", target),
            ));
        }

        if !path.exists() {
            return Ok(ToolResult::failure("apply_patch", format!("File not found: {}", target)));
        }
        if !path.is_file() {
            return Ok(ToolResult::failure("apply_patch", format!("Not a file: {}", target)));
        }

        let original = match std::fs::read_to_string(path) {
            Ok(s) => s,
            Err(e) => return Ok(ToolResult::failure("apply_patch", format!("Cannot read file: {}", e))),
        };

        let patched = match apply_hunks(&original, &hunks) {
            Ok(s) => s,
            Err(e) => return Ok(ToolResult::failure("apply_patch", format!("Patch failed: {}", e))),
        };

        let backup = params["backup"].as_bool().unwrap_or(true);
        let mut backup_path: Option<String> = None;
        if backup {
            let bak_str = format!("{}.bak", target);
            let bak = Path::new(&bak_str);
            if let Err(e) = std::fs::copy(path, &bak) {
                return Ok(ToolResult::failure("apply_patch", format!("Backup failed: {}", e)));
            }
            backup_path = Some(bak_str);
        }

        if let Err(e) = std::fs::write(path, patched) {
            return Ok(ToolResult::failure("apply_patch", format!("Write failed: {}", e)));
        }

        let mut metadata: HashMap<String, serde_json::Value> = HashMap::new();
        metadata.insert("path".into(), target.into());
        metadata.insert("hunks_applied".into(), hunks.len().into());
        if let Some(bp) = backup_path {
            metadata.insert("backup_path".into(), bp.into());
        }

        Ok(ToolResult {
            tool_name: "apply_patch".into(),
            content: format!("Patch applied successfully ({} hunk(s)).", hunks.len()),
            success: true,
            usage: HashMap::new(),
            cost_usd: 0.0,
            latency_seconds: 0.0,
            metadata,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_patch_basic() {
        let dir = std::env::temp_dir().join("sunday_test_apply_patch");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("hello.txt");
        std::fs::write(&file, "line1\nline2\nline3\n").unwrap();

        let patch = format!(
            "--- a/hello.txt\n+++ b/hello.txt\n@@ -1,3 +1,3 @@\n line1\n-line2\n+line2_modified\n line3\n"
        );

        let tool = ApplyPatchTool;
        let result = tool.execute(&serde_json::json!({
            "patch": patch,
            "path": file.to_str().unwrap(),
            "backup": false
        })).unwrap();

        assert!(result.success, "{}", result.content);
        let content = std::fs::read_to_string(&file).unwrap();
        assert!(content.contains("line2_modified"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_apply_patch_no_hunks() {
        let tool = ApplyPatchTool;
        let result = tool.execute(&serde_json::json!({
            "patch": "just some text",
            "backup": false
        })).unwrap();
        assert!(!result.success);
        assert!(result.content.contains("No hunks"));
    }
}
