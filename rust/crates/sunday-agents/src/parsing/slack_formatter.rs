//! Convert markdown to Slack mrkdwn format.

use regex::Regex;
use once_cell::sync::Lazy;

static HEADERS_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?m)^#{1,6}\s+(.+)$").unwrap()
});

static BOLD_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\*\*(.+?)\*\*").unwrap()
});

static STRIKETHROUGH_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"~~(.+?)~~").unwrap()
});

static LINKS_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\[(.+?)\]\((.+?)\)").unwrap()
});

static LATEX_BLOCK_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\$\$.+?\$\$").unwrap()
});

static LATEX_INLINE_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\$(.+?)\$").unwrap()
});

static WHITESPACE_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\n{3,}").unwrap()
});

/// Convert markdown to Slack mrkdwn format.
pub fn to_slack_fmt(text: &str) -> String {
    let mut result = text.to_string();

    // Headers → bold
    result = HEADERS_RE.replace_all(&result, "*$1*").to_string();

    // Bold: **text** → *text*
    result = BOLD_RE.replace_all(&result, "*$1*").to_string();

    // Strikethrough: ~~text~~ → ~text~
    result = STRIKETHROUGH_RE.replace_all(&result, "~$1~").to_string();

    // Links: [text](url) → <url|text>
    result = LINKS_RE.replace_all(&result, "<$2|$1>").to_string();

    // Remove LaTeX blocks
    result = LATEX_BLOCK_RE.replace_all(&result, "").to_string();

    // Inline LaTeX: $x$ → x
    result = LATEX_INLINE_RE.replace_all(&result, "$1").to_string();

    // Clean whitespace
    result = WHITESPACE_RE.replace_all(&result, "\n\n").to_string();

    result.trim().to_string()
}
