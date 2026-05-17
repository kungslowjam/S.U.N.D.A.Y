use crate::SkillManifest;
use serde_yaml::Value;
use std::collections::HashMap;

pub const MAX_NAME_LENGTH: usize = 64;
pub const MAX_DESCRIPTION_LENGTH: usize = 1024;
pub const MAX_COMPATIBILITY_LENGTH: usize = 500;

pub fn parse_skill_markdown(raw: &str) -> Result<SkillManifest, String> {
    let mut yaml_block = "";
    let mut markdown_content = raw;

    if raw.starts_with("---") {
        let rest = &raw[3..];
        let rest = if rest.starts_with('\n') { &rest[1..] } else { rest };
        if let Some(end_idx) = rest.find("\n---") {
            yaml_block = &rest[..end_idx];
            let mut after = &rest[end_idx + 4..];
            if after.starts_with('\n') {
                after = &after[1..];
            }
            markdown_content = after;
        }
    }

    let mut frontmatter: HashMap<String, Value> = if !yaml_block.is_empty() {
        serde_yaml::from_str(yaml_block).map_err(|e| format!("YAML parse error: {}", e))?
    } else {
        HashMap::new()
    };

    // Strict validation
    let name_val = frontmatter.get("name").ok_or("Missing required field: name")?;
    let name = name_val.as_str().ok_or("Field 'name' must be a string")?.to_string();

    let desc_val = frontmatter.get("description").ok_or("Missing required field: description")?;
    let description = desc_val.as_str().ok_or("Field 'description' must be a string")?.to_string();

    if name.is_empty() || name.len() > MAX_NAME_LENGTH {
        return Err(format!("Field 'name' must be 1-{} chars", MAX_NAME_LENGTH));
    }
    if description.is_empty() || description.len() > MAX_DESCRIPTION_LENGTH {
        return Err(format!("Field 'description' must be 1-{} chars", MAX_DESCRIPTION_LENGTH));
    }

    // Name validation
    if name != name.to_lowercase() {
        return Err(format!("Skill name '{}' must be lowercase", name));
    }
    if name.starts_with('-') || name.ends_with('-') {
        return Err(format!("Skill name '{}' must not start or end with a hyphen", name));
    }
    if name.contains("--") {
        return Err(format!("Skill name '{}' must not contain consecutive hyphens", name));
    }
    for ch in name.chars() {
        if !ch.is_ascii_alphanumeric() && ch != '-' {
            return Err(format!("Skill name '{}' contains invalid character '{}'", name, ch));
        }
    }

    if let Some(compat) = frontmatter.get("compatibility") {
        let compat_str = compat.as_str().ok_or("Field 'compatibility' must be a string")?;
        if compat_str.len() > MAX_COMPATIBILITY_LENGTH {
            return Err(format!("Field 'compatibility' exceeds {} chars", MAX_COMPATIBILITY_LENGTH));
        }
    }

    // Tolerant pass
    let mut manifest = SkillManifest {
        name,
        description,
        version: "0.1.0".to_string(),
        author: "".to_string(),
        steps: vec![],
        required_capabilities: vec![],
        signature: "".to_string(),
        metadata: HashMap::new(),
        tags: vec![],
        depends: vec![],
        user_invocable: true,
        disable_model_invocation: false,
        markdown_content: markdown_content.to_string(),
    };

    let spec_fields = vec!["name", "description", "license", "compatibility", "metadata", "allowed-tools"];

    let raw_meta = match frontmatter.get("metadata") {
        Some(Value::Mapping(m)) => m.clone(),
        _ => serde_yaml::Mapping::new(),
    };

    let mut oj_meta = match raw_meta.get(&Value::String("sunday".to_string())) {
        Some(Value::Mapping(m)) => m.clone(),
        _ => serde_yaml::Mapping::new(),
    };

    let mut unmapped = serde_yaml::Mapping::new();

    for (k, v) in &frontmatter {
        if spec_fields.contains(&k.as_str()) {
            continue;
        }

        match k.as_str() {
            "version" => manifest.version = v.as_str().unwrap_or("0.1.0").to_string(),
            "author" => manifest.author = v.as_str().unwrap_or("").to_string(),
            "tags" => {
                if let Some(seq) = v.as_sequence() {
                    manifest.tags = seq.iter().filter_map(|x| x.as_str().map(|s| s.to_string())).collect();
                }
            }
            "depends" => {
                if let Some(seq) = v.as_sequence() {
                    manifest.depends = seq.iter().filter_map(|x| x.as_str().map(|s| s.to_string())).collect();
                }
            }
            "required_capabilities" => {
                if let Some(seq) = v.as_sequence() {
                    manifest.required_capabilities = seq.iter().filter_map(|x| x.as_str().map(|s| s.to_string())).collect();
                }
            }
            "user_invocable" => manifest.user_invocable = v.as_bool().unwrap_or(true),
            "disable_model_invocation" => manifest.disable_model_invocation = v.as_bool().unwrap_or(false),
            "platforms" | "prerequisites" => {
                oj_meta.insert(Value::String(k.to_string()), v.clone());
            }
            _ => {
                unmapped.insert(Value::String(k.to_string()), v.clone());
            }
        }
    }

    for key in &["version", "author", "tags", "depends", "required_capabilities", "user_invocable", "disable_model_invocation"] {
        if let Some(v) = oj_meta.get(&Value::String(key.to_string())) {
            match *key {
                "version" => manifest.version = v.as_str().unwrap_or("0.1.0").to_string(),
                "author" => manifest.author = v.as_str().unwrap_or("").to_string(),
                "tags" => if let Some(seq) = v.as_sequence() { manifest.tags = seq.iter().filter_map(|x| x.as_str().map(|s| s.to_string())).collect(); },
                "depends" => if let Some(seq) = v.as_sequence() { manifest.depends = seq.iter().filter_map(|x| x.as_str().map(|s| s.to_string())).collect(); },
                "required_capabilities" => if let Some(seq) = v.as_sequence() { manifest.required_capabilities = seq.iter().filter_map(|x| x.as_str().map(|s| s.to_string())).collect(); },
                "user_invocable" => manifest.user_invocable = v.as_bool().unwrap_or(true),
                "disable_model_invocation" => manifest.disable_model_invocation = v.as_bool().unwrap_or(false),
                _ => {}
            }
        }
    }

    if !unmapped.is_empty() {
        oj_meta.insert(Value::String("original_frontmatter".to_string()), Value::Mapping(unmapped));
    }

    let mut final_meta = raw_meta.clone();
    if !oj_meta.is_empty() {
        final_meta.insert(Value::String("sunday".to_string()), Value::Mapping(oj_meta));
    }
    
    // convert serde_yaml mapping to serde_json for compatibility with the struct
    let json_meta: serde_json::Value = serde_yaml::from_value(Value::Mapping(final_meta)).unwrap_or(serde_json::Value::Object(serde_json::Map::new()));
    
    if let serde_json::Value::Object(map) = json_meta {
        for (k, v) in map {
            manifest.metadata.insert(k, v);
        }
    }

    Ok(manifest)
}
