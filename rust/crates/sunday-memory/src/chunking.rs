use regex::Regex;
use serde::{Deserialize, Serialize};
use once_cell::sync::Lazy;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MdChunk {
    pub content: String,
    pub source: String,
    pub breadcrumb: String,
    pub start_line: usize,
}

static FENCE_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"^```").unwrap());
static HEADER_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"^(#{1,6})\s+(.+?)\s*$").unwrap());

pub fn chunk_markdown(
    text: &str,
    source: &str,
    max_section_tokens: usize,
    paragraph_overlap_tokens: usize,
    max_section_chars: usize,
) -> Vec<MdChunk> {
    if text.trim().is_empty() {
        return vec![];
    }

    let mut h1: Option<String> = None;
    let mut h2: Option<String> = None;
    let mut h3: Option<String> = None;
    let mut buffered: Vec<String> = Vec::new();
    let mut sections: Vec<(String, String, usize)> = Vec::new();
    let mut section_start_line = 0;
    let mut in_code = false;

    for (lineno, line) in text.lines().enumerate() {
        let stripped = line.trim();
        if FENCE_RE.is_match(stripped) {
            in_code = !in_code;
            buffered.push(line.to_string());
            continue;
        }

        if in_code {
            buffered.push(line.to_string());
            continue;
        }

        if let Some(caps) = HEADER_RE.captures(stripped) {
            let hashes = caps.get(1).unwrap().as_str();
            let title = caps.get(2).unwrap().as_str().trim().to_string();
            let level = hashes.len();

            if level <= 3 {
                // Flush current buffer
                if !buffered.is_empty() {
                    let body = buffered.join("\n").trim().to_string();
                    if !body.is_empty() {
                        let breadcrumb = vec![h1.as_ref(), h2.as_ref(), h3.as_ref()]
                            .into_iter()
                            .flatten()
                            .cloned()
                            .collect::<Vec<_>>()
                            .join(" > ");
                        let bc = if breadcrumb.is_empty() { source.to_string() } else { breadcrumb };
                        sections.push((bc, body, section_start_line));
                    }
                    buffered.clear();
                }

                match level {
                    1 => { h1 = Some(title); h2 = None; h3 = None; }
                    2 => { h2 = Some(title); h3 = None; }
                    3 => { h3 = Some(title); }
                    _ => unreachable!(),
                }
                section_start_line = lineno;
            } else {
                buffered.push(line.to_string());
            }
        } else {
            buffered.push(line.to_string());
        }
    }

    // Final flush
    if !buffered.is_empty() {
        let body = buffered.join("\n").trim().to_string();
        if !body.is_empty() {
            let breadcrumb = vec![h1.as_ref(), h2.as_ref(), h3.as_ref()]
                .into_iter()
                .flatten()
                .cloned()
                .collect::<Vec<_>>()
                .join(" > ");
            let bc = if breadcrumb.is_empty() { source.to_string() } else { breadcrumb };
            sections.push((bc, body, section_start_line));
        }
    }

    if sections.is_empty() {
        let bc = h1.unwrap_or_else(|| source.to_string());
        sections.push((bc, text.trim().to_string(), 0));
    }

    let mut chunks = Vec::new();

    for (breadcrumb, body, start_line) in sections {
        let body_tokens = body.split_whitespace().count();
        if body_tokens <= max_section_tokens && body.len() <= max_section_chars {
            chunks.push(MdChunk {
                content: format!("{}\n\n{}", breadcrumb, body),
                source: source.to_string(),
                breadcrumb,
                start_line,
            });
            continue;
        }

        // Oversized: split by paragraphs
        let paragraphs: Vec<&str> = body.split("\n\n").filter(|p| !p.trim().is_empty()).collect();
        let mut window_paragraphs = Vec::new();
        let mut window_tokens = 0;
        let mut window_chars = 0;

        for para in paragraphs {
            let p_tokens = para.split_whitespace().count();
            let p_chars = para.len();

            if window_tokens + p_tokens > max_section_tokens || window_chars + p_chars + 2 > max_section_chars {
                if !window_paragraphs.is_empty() {
                    let chunk_body = window_paragraphs.join("\n\n");
                    chunks.push(MdChunk {
                        content: format!("{}\n\n{}", breadcrumb, chunk_body),
                        source: source.to_string(),
                        breadcrumb: breadcrumb.clone(),
                        start_line,
                    });

                    // Carry overlap
                    let words: Vec<&str> = chunk_body.split_whitespace().collect();
                    let overlap_start = if words.len() > paragraph_overlap_tokens {
                        words.len() - paragraph_overlap_tokens
                    } else {
                        0
                    };
                    let tail = words[overlap_start..].join(" ");
                    window_tokens = tail.split_whitespace().count();
                    window_chars = tail.len();
                    window_paragraphs = if !tail.is_empty() { vec![tail] } else { vec![] };
                }
            }
            
            window_paragraphs.push(para.to_string());
            window_tokens += p_tokens;
            window_chars += p_chars + 2;
        }

        if !window_paragraphs.is_empty() {
            chunks.push(MdChunk {
                content: format!("{}\n\n{}", breadcrumb, window_paragraphs.join("\n\n")),
                source: source.to_string(),
                breadcrumb,
                start_line,
            });
        }
    }

    chunks
}
