//! Strip think tags, reasoning blocks, and assistant preambles.

use regex::Regex;
use once_cell::sync::Lazy;

static THOUGHT_TAG_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?is)<thought>.*?</thought>").unwrap()
});

static THOUGHT_HEADER_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?is)(?:THOUGHT|Thinking Process|Reasoning|THOUGHTS):\s*(?:.*?(?:\nTOOL:|\nFINAL|\z))").unwrap()
});

static WAIT_PAREN_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?is)\(Wait,.*?\)").unwrap()
});

static REVISED_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)(?:Revised|Correction|Choice|Final Choice|Wait).*?:.*").unwrap()
});

static PREAMBLE_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)^(?:Certainly|Sure|I can help with that|Of course|Okay|Alright)[^.!?]*[.!?]\s*").unwrap()
});

/// Strip internal thinking/reasoning blocks from model output.
pub fn strip_think_tags(text: &str) -> String {
    if text.is_empty() {
        return String::new();
    }

    let mut result = text.to_string();

    // Remove XML-style tags
    result = THOUGHT_TAG_RE.replace_all(&result, "").to_string();

    // Remove structured headers
    result = THOUGHT_HEADER_RE.replace_all(&result, "").to_string();

    // Remove patterns like "(Wait, ...)"
    result = WAIT_PAREN_RE.replace_all(&result, "").to_string();

    // Remove Revised/Correction/Choice lines
    result = REVISED_RE.replace_all(&result, "").to_string();

    // Remove common assistant preambles
    result = PREAMBLE_RE.replace_all(&result, "").to_string();

    result.trim().to_string()
}
