use std::collections::{HashMap, HashSet};
use crate::chunking::MdChunk;
use serde::{Deserialize, Serialize};
use regex::Regex;
use once_cell::sync::Lazy;

#[derive(Debug, Serialize, Deserialize)]
pub struct DuplicateGroup {
    pub kept_index: usize,
    pub kept_source: String,
    pub dropped_indices: Vec<usize>,
    pub dropped_sources: Vec<String>,
    pub distinct_files: usize,
    pub sample_text: String,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct DedupeReport {
    pub input_count: usize,
    pub output_count: usize,
    pub groups: Vec<DuplicateGroup>,
}

static NORM_WS_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"\s+").unwrap());
static NORM_NONALPHA_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"[^a-z0-9\s]+").unwrap());

fn normalize(text: &str) -> String {
    let text = text.to_lowercase();
    let text = NORM_NONALPHA_RE.replace_all(&text, " ");
    let text = NORM_WS_RE.replace_all(&text, " ");
    text.trim().to_string()
}

fn get_ngrams(text: &str, n: usize) -> HashSet<Vec<String>> {
    let tokens: Vec<String> = text.split_whitespace().map(|s| s.to_string()).collect();
    if tokens.is_empty() {
        return HashSet::new();
    }
    if tokens.len() < n {
        let mut hs = HashSet::new();
        hs.insert(tokens);
        return hs;
    }
    let mut hs = HashSet::new();
    for i in 0..=(tokens.len() - n) {
        hs.insert(tokens[i..i + n].to_vec());
    }
    hs
}

fn jaccard(a: &HashSet<Vec<String>>, b: &HashSet<Vec<String>>) -> f32 {
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    let inter = a.intersection(b).count();
    if inter == 0 {
        return 0.0;
    }
    let union = a.union(b).count();
    inter as f32 / union as f32
}

pub fn dedupe_chunks(
    chunks: Vec<MdChunk>,
    ngram_n: usize,
    similarity_threshold: f32,
    min_files_for_dup: usize,
) -> (Vec<MdChunk>, DedupeReport) {
    let n = chunks.len();
    if n == 0 {
        return (vec![], DedupeReport::default());
    }

    // 1) Normalize and get ngrams
    let chunk_ngrams: Vec<HashSet<Vec<String>>> = chunks.iter()
        .map(|c| {
            let body = c.content.split("\n\n").nth(1).unwrap_or(&c.content);
            get_ngrams(&normalize(body), ngram_n)
        })
        .collect();

    // 2) Inverted Index for candidate selection
    let mut inverted: HashMap<Vec<String>, Vec<usize>> = HashMap::new();
    for (i, ngs) in chunk_ngrams.iter().enumerate() {
        for ng in ngs {
            inverted.entry(ng.clone()).or_default().push(i);
        }
    }

    // 3) Candidate pairs
    let mut candidate_pairs = HashSet::new();
    for ids in inverted.values() {
        if ids.len() < 2 { continue; }
        // Limit candidates to avoid quadratic blowup
        let ids = if ids.len() > 200 { &ids[..200] } else { ids };
        for i in 0..ids.len() {
            for j in (i + 1)..ids.len() {
                let (a, b) = if ids[i] < ids[j] { (ids[i], ids[j]) } else { (ids[j], ids[i]) };
                candidate_pairs.insert((a, b));
            }
        }
    }

    // 4) Union Find
    let mut parents: Vec<usize> = (0..n).collect();
    fn find(parents: &mut Vec<usize>, i: usize) -> usize {
        if parents[i] == i {
            i
        } else {
            let root = find(parents, parents[i]);
            parents[i] = root;
            root
        }
    }
    fn union(parents: &mut Vec<usize>, i: usize, j: usize) {
        let root_i = find(parents, i);
        let root_j = find(parents, j);
        if root_i != root_j {
            parents[root_i] = root_j;
        }
    }

    for (a, b) in candidate_pairs {
        if jaccard(&chunk_ngrams[a], &chunk_ngrams[b]) >= similarity_threshold {
            union(&mut parents, a, b);
        }
    }

    // 5) Grouping
    let mut clusters: HashMap<usize, Vec<usize>> = HashMap::new();
    for i in 0..n {
        let root = find(&mut parents, i);
        clusters.entry(root).or_default().push(i);
    }

    // 6) Final selection
    let mut keep_mask = vec![true; n];
    let mut report = DedupeReport { input_count: n, ..Default::default() };

    for members in clusters.values() {
        if members.len() < 2 { continue; }
        
        let distinct_files: HashSet<&String> = members.iter().map(|&i| &chunks[i].source).collect();
        if distinct_files.len() < min_files_for_dup { continue; }

        // Pick canonical (deepest path, etc - simplified to first one for now)
        let mut sorted_members = members.clone();
        sorted_members.sort_by(|&a, &b| chunks[b].source.len().cmp(&chunks[a].source.len())); // Proxy for specificity
        
        let kept_idx = sorted_members[0];
        let dropped_indices: Vec<usize> = sorted_members[1..].to_vec();
        
        for &d in &dropped_indices {
            keep_mask[d] = false;
        }

        report.groups.push(DuplicateGroup {
            kept_index: kept_idx,
            kept_source: chunks[kept_idx].source.clone(),
            dropped_indices: dropped_indices.clone(),
            dropped_sources: dropped_indices.iter().map(|&i| chunks[i].source.clone()).collect(),
            distinct_files: distinct_files.len(),
            sample_text: chunks[kept_idx].content.chars().take(120).collect(),
        });
    }

    let survivors: Vec<MdChunk> = chunks.into_iter().enumerate()
        .filter(|(i, _)| keep_mask[*i])
        .map(|(_, c)| c)
        .collect();

    report.output_count = survivors.len();
    (survivors, report)
}
