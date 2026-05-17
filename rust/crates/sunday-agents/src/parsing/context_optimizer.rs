//! Context management helpers — compression and windowing without token estimation.

/// Compress large tool outputs/observations.
/// Returns (new_content, was_truncated) for each message.
/// Input: list of (role, content, next_role) where next_role is the role of the following message.
/// Roles: "system", "user", "assistant", "tool"
pub fn compress_tool_outputs(
    messages: &[(String, String, Option<String>)],
    threshold: usize,
) -> Vec<(String, String, bool)> {
    let mut result = Vec::with_capacity(messages.len());

    for (_i, (role, content, next_role)) in messages.iter().enumerate() {
        let is_large_obs = (role == "tool" || (role == "user" && content.starts_with("Observation:")))
            && content.len() > threshold;

        let is_followed_by_assistant = next_role.as_deref() == Some("assistant");

        if is_large_obs && is_followed_by_assistant && content.len() > 1000 {
            let start = &content[..content.len().min(500)];
            let end = &content[content.len().saturating_sub(500)..];
            let truncated = format!(
                "{}\n\n... [TRUNCATED {} CHARS] ...\n\n{}",
                start,
                content.len() - 1000,
                end
            );
            result.push((role.clone(), truncated, true));
        } else {
            result.push((role.clone(), content.clone(), false));
        }
    }

    result
}

/// Apply message windowing — keep only the most recent messages while preserving essentials.
/// Returns indices of messages to keep (in original order).
pub fn apply_window(
    message_roles: &[String],
    max_messages: usize,
    preserve_system: bool,
    preserve_initial_user: bool,
) -> Vec<usize> {
    if message_roles.len() <= max_messages {
        return (0..message_roles.len()).collect();
    }

    // Find system message indices
    let system_indices: Vec<usize> = message_roles
        .iter()
        .enumerate()
        .filter(|(_, r)| *r == "system")
        .map(|(i, _)| i)
        .collect();

    // Find first user message index
    let initial_user_idx = message_roles
        .iter()
        .enumerate()
        .find(|(_, r)| *r == "user")
        .map(|(i, _)| i);

    // Calculate how many recent messages we can keep
    let preserved_count = if preserve_system { system_indices.len() } else { 0 }
        + if preserve_initial_user && initial_user_idx.is_some() { 1 } else { 0 };

    let recent_count = if max_messages > preserved_count {
        max_messages - preserved_count
    } else {
        // Fallback: keep system + last 2
        let mut result = system_indices.clone();
        if message_roles.len() >= 2 {
            result.push(message_roles.len() - 2);
            result.push(message_roles.len() - 1);
        } else if !message_roles.is_empty() {
            result.push(message_roles.len() - 1);
        }
        result.sort_unstable();
        result.dedup();
        return result;
    };

    // Collect indices to keep
    let mut keep = Vec::with_capacity(max_messages);

    // Add system messages
    if preserve_system {
        keep.extend(&system_indices);
    }

    // Add initial user if not in recent window
    if preserve_initial_user {
        if let Some(idx) = initial_user_idx {
            let recent_start = message_roles.len().saturating_sub(recent_count);
            if idx < recent_start && !keep.contains(&idx) {
                keep.push(idx);
            }
        }
    }

    // Add recent history
    let recent_start = message_roles.len().saturating_sub(recent_count);
    for i in recent_start..message_roles.len() {
        if !keep.contains(&i) {
            keep.push(i);
        }
    }

    keep.sort_unstable();
    keep
}
