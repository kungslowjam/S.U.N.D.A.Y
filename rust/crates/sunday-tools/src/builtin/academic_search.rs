//! Academic search tools — Semantic Scholar, arXiv, OpenAlex.

use crate::traits::BaseTool;
use sunday_core::{SUNDAYError, ToolResult, ToolSpec};
use once_cell::sync::Lazy;
use serde_json::Value;
use std::collections::HashMap;

static USER_AGENT: &str = "SUNDAY/0.1 academic-search (local user agent)";

fn year_ok(year: Option<i64>, start_year: Option<i64>, end_year: Option<i64>) -> bool {
    if year.is_none() {
        return true;
    }
    let y = year.unwrap();
    if let Some(sy) = start_year {
        if y < sy { return false; }
    }
    if let Some(ey) = end_year {
        if y > ey { return false; }
    }
    true
}

// ---------------------------------------------------------------------------
// Semantic Scholar
// ---------------------------------------------------------------------------

static SEMANTIC_SCHOLAR_SPEC: Lazy<ToolSpec> = Lazy::new(|| ToolSpec {
    name: "semantic_scholar_search".into(),
    description: "Search Semantic Scholar for academic papers. Use this before generic web_search when the user asks for research papers, literature, DOI, citations, authors, or publication years.".into(),
    parameters: serde_json::json!({
        "type": "object",
        "properties": {
            "query": { "type": "string", "description": "Academic paper search query." },
            "limit": { "type": "integer", "description": "Maximum number of papers to return." },
            "start_year": { "type": "integer", "description": "Optional earliest publication year." },
            "end_year": { "type": "integer", "description": "Optional latest publication year." }
        },
        "required": ["query"]
    }),
    category: "academic_search".into(),
    cost_estimate: 0.0,
    latency_estimate: 0.0,
    requires_confirmation: false,
    timeout_seconds: 20.0,
    required_capabilities: vec!["network:fetch".into()],
    metadata: HashMap::new(),
});

pub struct SemanticScholarSearchTool;

impl BaseTool for SemanticScholarSearchTool {
    fn tool_id(&self) -> &str { "semantic_scholar_search" }
    fn spec(&self) -> &ToolSpec { &SEMANTIC_SCHOLAR_SPEC }
    fn execute(&self, params: &Value) -> Result<ToolResult, SUNDAYError> {
        let query = params["query"].as_str().unwrap_or("").trim();
        if query.is_empty() {
            return Ok(ToolResult::failure("semantic_scholar_search", "No query provided."));
        }
        let limit = params["limit"].as_i64().unwrap_or(5).clamp(1, 20) as usize;
        let start_year = params["start_year"].as_i64();
        let end_year = params["end_year"].as_i64();
        let request_limit = (limit * 3).clamp(limit, 50);

        let fields = "title,authors,year,abstract,citationCount,influentialCitationCount,publicationVenue,externalIds,openAccessPdf,url";
        let url = "https://api.semanticscholar.org/graph/v1/paper/search";

        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(20))
            .build()
            .map_err(|e| SUNDAYError::Io(std::io::Error::other(e.to_string())))?;

        let mut resp: Option<reqwest::blocking::Response> = None;
        for attempt in 0..3 {
            match client.get(url)
                .query(&[("query", query), ("limit", &request_limit.to_string()), ("fields", fields)])
                .header("User-Agent", USER_AGENT)
                .send() {
                Ok(r) => {
                    if r.status() != 429 {
                        resp = Some(r);
                        break;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(1500 * (attempt + 1) as u64));
                }
                Err(e) => {
                    if attempt == 2 {
                        return Ok(ToolResult::failure("semantic_scholar_search", format!("Request failed: {}", e)));
                    }
                }
            }
        }

        let resp = match resp {
            Some(r) => r,
            None => return Ok(ToolResult::failure("semantic_scholar_search", "Rate limited after retries.")),
        };

        if let Err(e) = resp.error_for_status_ref() {
            return Ok(ToolResult::failure("semantic_scholar_search", format!("HTTP error: {}", e)));
        }

        let data: Value = match resp.json() {
            Ok(v) => v,
            Err(e) => return Ok(ToolResult::failure("semantic_scholar_search", format!("JSON parse error: {}", e))),
        };

        let mut papers: Vec<Value> = Vec::new();
        if let Some(data_arr) = data["data"].as_array() {
            for paper in data_arr {
                let year = paper["year"].as_i64();
                if !year_ok(year, start_year, end_year) {
                    continue;
                }
                papers.push(paper.clone());
                if papers.len() >= limit {
                    break;
                }
            }
        }

        if papers.is_empty() {
            return Ok(ToolResult {
                tool_name: "semantic_scholar_search".into(),
                content: "No matching academic papers found.".into(),
                success: true,
                usage: HashMap::new(),
                cost_usd: 0.0,
                latency_seconds: 0.0,
                metadata: {
                    let mut m = HashMap::new();
                    m.insert("query".into(), query.into());
                    m.insert("num_results".into(), 0.into());
                    m
                },
            });
        }

        let mut lines: Vec<String> = Vec::new();
        for (idx, paper) in papers.iter().enumerate() {
            let title = paper["title"].as_str().unwrap_or("Untitled");
            let year = paper["year"].as_i64().map(|y| y.to_string()).unwrap_or_else(|| "unknown".into());
            let venue = paper["publicationVenue"]["name"].as_str().unwrap_or("");
            let authors_arr = paper["authors"].as_array();
            let mut authors: Vec<String> = Vec::new();
            if let Some(arr) = authors_arr {
                authors = arr.iter().filter_map(|a| a["name"].as_str().map(|s| s.to_string())).take(4).collect();
                if arr.len() > 4 {
                    authors.push("et al.".into());
                }
            }
            let citation_count = paper["citationCount"].as_i64().unwrap_or(0);
            let influential = paper["influentialCitationCount"].as_i64().unwrap_or(0);
            let paper_url = paper["url"].as_str().unwrap_or("");
            let external = paper["externalIds"].as_object();
            let doi = external.and_then(|e| e.get("DOI")).and_then(|v| v.as_str()).unwrap_or("");
            let arxiv = external.and_then(|e| e.get("ArXiv")).and_then(|v| v.as_str()).unwrap_or("");
            let pdf = paper["openAccessPdf"]["url"].as_str().unwrap_or("");
            let abstract_text = paper["abstract"].as_str().unwrap_or("").replace('\n', " ").trim().to_string();
            let abstract_text = if abstract_text.len() > 450 { format!("{}...", &abstract_text[..450]) } else { abstract_text };

            lines.push(format!("{}. {}", idx + 1, title));
            lines.push(format!("   Year: {}{}", year, if venue.is_empty() { "".into() } else { format!(" | Venue: {}", venue) }));
            lines.push(format!("   Authors: {}", if authors.is_empty() { "unknown".into() } else { authors.join(", ") }));
            lines.push(format!("   Citations: {} | Influential: {}", citation_count, influential));
            lines.push(format!("   URL: {}", paper_url));
            if !doi.is_empty() { lines.push(format!("   DOI: {}", doi)); }
            if !arxiv.is_empty() { lines.push(format!("   arXiv: https://arxiv.org/abs/{}", arxiv)); }
            if !pdf.is_empty() { lines.push(format!("   PDF: {}", pdf)); }
            if !abstract_text.is_empty() { lines.push(format!("   Abstract: {}", abstract_text)); }
            lines.push("".into());
        }

        Ok(ToolResult {
            tool_name: "semantic_scholar_search".into(),
            content: lines.join("\n").trim().into(),
            success: true,
            usage: HashMap::new(),
            cost_usd: 0.0,
            latency_seconds: 0.0,
            metadata: {
                let mut m = HashMap::new();
                m.insert("query".into(), query.into());
                m.insert("num_results".into(), papers.len().into());
                m.insert("source".into(), "semantic_scholar".into());
                m
            },
        })
    }
}

// ---------------------------------------------------------------------------
// arXiv
// ---------------------------------------------------------------------------

static ARXIV_SPEC: Lazy<ToolSpec> = Lazy::new(|| ToolSpec {
    name: "arxiv_search".into(),
    description: "Search arXiv for academic preprints. Use only when the user explicitly asks for arXiv/preprints, or when openalex_search returns no useful results. For broad research paper searches such as water management, smart water, IoT water monitoring, or applied engineering papers, prefer openalex_search first.".into(),
    parameters: serde_json::json!({
        "type": "object",
        "properties": {
            "query": { "type": "string", "description": "arXiv query." },
            "limit": { "type": "integer", "description": "Maximum papers to return." },
            "start_year": { "type": "integer", "description": "Optional earliest publication year." },
            "end_year": { "type": "integer", "description": "Optional latest publication year." }
        },
        "required": ["query"]
    }),
    category: "academic_search".into(),
    cost_estimate: 0.0,
    latency_estimate: 0.0,
    requires_confirmation: false,
    timeout_seconds: 20.0,
    required_capabilities: vec!["network:fetch".into()],
    metadata: HashMap::new(),
});

pub struct ArxivSearchTool;

impl BaseTool for ArxivSearchTool {
    fn tool_id(&self) -> &str { "arxiv_search" }
    fn spec(&self) -> &ToolSpec { &ARXIV_SPEC }
    fn execute(&self, params: &Value) -> Result<ToolResult, SUNDAYError> {
        let query = params["query"].as_str().unwrap_or("").trim();
        if query.is_empty() {
            return Ok(ToolResult::failure("arxiv_search", "No query provided."));
        }
        let limit = params["limit"].as_i64().unwrap_or(5).clamp(1, 20) as usize;
        let start_year = params["start_year"].as_i64();
        let end_year = params["end_year"].as_i64();
        let request_limit = (limit * 3).clamp(limit, 50);
        let encoded_query: String = query.split_whitespace().map(|part| urlencoding::encode(part)).collect::<Vec<_>>().join("+");
        let url = format!(
            "https://export.arxiv.org/api/query?search_query=all:{}&sortBy=submittedDate&sortOrder=descending&max_results={}",
            encoded_query, request_limit
        );

        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(20))
            .build()
            .map_err(|e| SUNDAYError::Io(std::io::Error::other(e.to_string())))?;

        let mut resp: Option<reqwest::blocking::Response> = None;
        for attempt in 0..3 {
            match client.get(&url).header("User-Agent", USER_AGENT).send() {
                Ok(r) => {
                    if r.status() != 429 {
                        resp = Some(r);
                        break;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(3200 * (attempt + 1) as u64));
                }
                Err(e) => {
                    if attempt == 2 {
                        return Ok(ToolResult::failure("arxiv_search", format!("Request failed: {}", e)));
                    }
                }
            }
        }

        let resp = match resp {
            Some(r) => r,
            None => return Ok(ToolResult::failure("arxiv_search", "Rate limited after retries.")),
        };

        if let Err(e) = resp.error_for_status_ref() {
            return Ok(ToolResult::failure("arxiv_search", format!("HTTP error: {}", e)));
        }

        let text = match resp.text() {
            Ok(t) => t,
            Err(e) => return Ok(ToolResult::failure("arxiv_search", format!("Read error: {}", e))),
        };

        let mut papers: Vec<(String, String, String, String, String)> = Vec::new();
        for entry in text.split("<entry>").skip(1) {
            let end = entry.find("</entry>").unwrap_or(entry.len());
            let entry = &entry[..end];

            let published = extract_xml_tag(entry, "published").unwrap_or_default();
            let year = published[..4.min(published.len())].parse::<i64>().ok();
            if !year_ok(year, start_year, end_year) {
                continue;
            }

            let title = extract_xml_tag(entry, "title").unwrap_or_default().split_whitespace().collect::<Vec<_>>().join(" ");
            let arxiv_url = extract_xml_tag(entry, "id").unwrap_or_default();
            let arxiv_id = arxiv_url.split("/abs/").last().unwrap_or("").to_string();
            let authors = entry.split("<author>")
                .skip(1)
                .filter_map(|a| a.split("<name>").nth(1).and_then(|n| n.split("</name>").next()))
                .take(4)
                .collect::<Vec<_>>()
                .join(", ");
            let authors = if entry.matches("<author>").count() > 4 {
                format!("{}, et al.", authors)
            } else {
                authors
            };
            let summary = extract_xml_tag(entry, "summary").unwrap_or_default().split_whitespace().collect::<Vec<_>>().join(" ");
            let summary = if summary.len() > 450 { format!("{}...", &summary[..450]) } else { summary };

            papers.push((title, published[..10.min(published.len())].to_string(), authors, arxiv_id, summary));
            if papers.len() >= limit {
                break;
            }
        }

        if papers.is_empty() {
            return Ok(ToolResult {
                tool_name: "arxiv_search".into(),
                content: "No matching arXiv papers found.".into(),
                success: true,
                usage: HashMap::new(),
                cost_usd: 0.0,
                latency_seconds: 0.0,
                metadata: {
                    let mut m = HashMap::new();
                    m.insert("query".into(), query.into());
                    m.insert("num_results".into(), 0.into());
                    m
                },
            });
        }

        let mut lines: Vec<String> = Vec::new();
        for (idx, (title, published, authors, arxiv_id, summary)) in papers.iter().enumerate() {
            lines.push(format!("{}. {}", idx + 1, title));
            lines.push(format!("   Published: {}", published));
            lines.push(format!("   Authors: {}", if authors.is_empty() { "unknown" } else { authors }));
            lines.push(format!("   URL: https://arxiv.org/abs/{}", arxiv_id));
            lines.push(format!("   PDF: https://arxiv.org/pdf/{}", arxiv_id));
            if !summary.is_empty() { lines.push(format!("   Abstract: {}", summary)); }
            lines.push("".into());
        }

        Ok(ToolResult {
            tool_name: "arxiv_search".into(),
            content: lines.join("\n").trim().into(),
            success: true,
            usage: HashMap::new(),
            cost_usd: 0.0,
            latency_seconds: 0.0,
            metadata: {
                let mut m = HashMap::new();
                m.insert("query".into(), query.into());
                m.insert("num_results".into(), papers.len().into());
                m.insert("source".into(), "arxiv".into());
                m
            },
        })
    }
}

fn extract_xml_tag(xml: &str, tag: &str) -> Option<String> {
    let start = format!("<{}>", tag);
    let end = format!("</{}>", tag);
    let s = xml.find(&start)? + start.len();
    let e = xml[s..].find(&end)?;
    Some(xml[s..s + e].to_string())
}

// ---------------------------------------------------------------------------
// OpenAlex
// ---------------------------------------------------------------------------

static OPENALEX_SPEC: Lazy<ToolSpec> = Lazy::new(|| ToolSpec {
    name: "openalex_search".into(),
    description: "Search OpenAlex for scholarly papers and metadata. This is a good fallback or primary tool for research-paper requests when Semantic Scholar is rate-limited.".into(),
    parameters: serde_json::json!({
        "type": "object",
        "properties": {
            "query": { "type": "string", "description": "Paper search query." },
            "limit": { "type": "integer", "description": "Maximum papers to return." },
            "start_year": { "type": "integer", "description": "Optional earliest publication year." },
            "end_year": { "type": "integer", "description": "Optional latest publication year." }
        },
        "required": ["query"]
    }),
    category: "academic_search".into(),
    cost_estimate: 0.0,
    latency_estimate: 0.0,
    requires_confirmation: false,
    timeout_seconds: 20.0,
    required_capabilities: vec!["network:fetch".into()],
    metadata: HashMap::new(),
});

pub struct OpenAlexSearchTool;

impl BaseTool for OpenAlexSearchTool {
    fn tool_id(&self) -> &str { "openalex_search" }
    fn spec(&self) -> &ToolSpec { &OPENALEX_SPEC }
    fn execute(&self, params: &Value) -> Result<ToolResult, SUNDAYError> {
        let query = params["query"].as_str().unwrap_or("").trim();
        if query.is_empty() {
            return Ok(ToolResult::failure("openalex_search", "No query provided."));
        }
        let limit = params["limit"].as_i64().unwrap_or(5).clamp(1, 20) as usize;
        let start_year = params["start_year"].as_i64();
        let end_year = params["end_year"].as_i64();

        let mut filters: Vec<String> = Vec::new();
        if let Some(sy) = start_year {
            filters.push(format!("from_publication_date:{}-01-01", sy));
        }
        if let Some(ey) = end_year {
            filters.push(format!("to_publication_date:{}-12-31", ey));
        }

        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(20))
            .build()
            .map_err(|e| SUNDAYError::Io(std::io::Error::other(e.to_string())))?;

        let mut req = client.get("https://api.openalex.org/works")
            .query(&[("search", query), ("per-page", &limit.to_string()), ("sort", "relevance_score:desc"), ("mailto", "local@sunday.local")])
            .header("User-Agent", USER_AGENT);

        if !filters.is_empty() {
            req = req.query(&[("filter", &filters.join(","))]);
        }

        let resp = match req.send() {
            Ok(r) => r,
            Err(e) => return Ok(ToolResult::failure("openalex_search", format!("Request failed: {}", e))),
        };

        if let Err(e) = resp.error_for_status_ref() {
            return Ok(ToolResult::failure("openalex_search", format!("HTTP error: {}", e)));
        }

        let data: Value = match resp.json() {
            Ok(v) => v,
            Err(e) => return Ok(ToolResult::failure("openalex_search", format!("JSON parse error: {}", e))),
        };

        let works = if let Some(arr) = data["results"].as_array() {
            arr.iter().take(limit).cloned().collect::<Vec<_>>()
        } else {
            Vec::new()
        };

        if works.is_empty() {
            return Ok(ToolResult {
                tool_name: "openalex_search".into(),
                content: "No matching OpenAlex papers found.".into(),
                success: true,
                usage: HashMap::new(),
                cost_usd: 0.0,
                latency_seconds: 0.0,
                metadata: {
                    let mut m = HashMap::new();
                    m.insert("query".into(), query.into());
                    m.insert("num_results".into(), 0.into());
                    m
                },
            });
        }

        let mut lines: Vec<String> = Vec::new();
        for (idx, work) in works.iter().enumerate() {
            let title = work["title"].as_str().unwrap_or("Untitled");
            let year = work["publication_year"].as_i64().map(|y| y.to_string()).unwrap_or_else(|| "unknown".into());
            let authorships = work["authorships"].as_array();
            let mut authors: Vec<String> = Vec::new();
            if let Some(arr) = authorships {
                authors = arr.iter()
                    .filter_map(|a| a["author"]["display_name"].as_str().map(|s| s.to_string()))
                    .take(4)
                    .collect();
                if arr.len() > 4 {
                    authors.push("et al.".into());
                }
            }
            let doi = work["doi"].as_str().unwrap_or("");
            let host = work["primary_location"]["source"]["display_name"].as_str().unwrap_or("");
            let oa_url = work["open_access"]["oa_url"].as_str().unwrap_or("");
            let landing = work["id"].as_str().unwrap_or("");
            let abstract_text = abstract_from_inverted_index(work.get("abstract_inverted_index"));

            lines.push(format!("{}. {}", idx + 1, title));
            lines.push(format!("   Year: {}{}", year, if host.is_empty() { "".into() } else { format!(" | Venue: {}", host) }));
            lines.push(format!("   Authors: {}", if authors.is_empty() { "unknown".into() } else { authors.join(", ") }));
            lines.push(format!("   Citations: {}", work["cited_by_count"].as_i64().unwrap_or(0)));
            lines.push(format!("   URL: {}", if doi.is_empty() { landing } else { doi }));
            if !oa_url.is_empty() && oa_url != doi { lines.push(format!("   Open access: {}", oa_url)); }
            if !abstract_text.is_empty() { lines.push(format!("   Abstract: {}", abstract_text)); }
            lines.push("".into());
        }

        Ok(ToolResult {
            tool_name: "openalex_search".into(),
            content: lines.join("\n").trim().into(),
            success: true,
            usage: HashMap::new(),
            cost_usd: 0.0,
            latency_seconds: 0.0,
            metadata: {
                let mut m = HashMap::new();
                m.insert("query".into(), query.into());
                m.insert("num_results".into(), works.len().into());
                m.insert("source".into(), "openalex".into());
                m
            },
        })
    }
}

fn abstract_from_inverted_index(value: Option<&Value>) -> String {
    let obj = match value.and_then(|v| v.as_object()) {
        Some(o) => o,
        None => return String::new(),
    };
    let mut positions: Vec<(usize, &str)> = Vec::new();
    for (word, indexes) in obj.iter() {
        if let Some(arr) = indexes.as_array() {
            for idx_val in arr {
                if let Some(idx) = idx_val.as_u64() {
                    positions.push((idx as usize, word.as_str()));
                }
            }
        }
    }
    positions.sort_by_key(|k| k.0);
    let text: Vec<&str> = positions.iter().map(|(_, word)| *word).collect();
    let text = text.join(" ");
    if text.len() > 450 { format!("{}...", &text[..450]) } else { text }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_year_ok() {
        assert!(year_ok(Some(2020), Some(2019), Some(2021)));
        assert!(!year_ok(Some(2018), Some(2019), Some(2021)));
        assert!(year_ok(None, Some(2019), Some(2021)));
    }

    #[test]
    fn test_abstract_from_inverted_index() {
        let json = serde_json::json!({
            "hello": [0, 2],
            "world": [1]
        });
        assert_eq!(abstract_from_inverted_index(Some(&json)), "hello world hello");
    }
}
