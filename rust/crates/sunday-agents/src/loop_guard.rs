//! Loop guard — detect and prevent agent loops.

use sha2::{Digest, Sha256};
use std::collections::{HashSet, VecDeque};

#[allow(dead_code)]
pub struct LoopGuard {
    seen_hashes: HashSet<String>,
    seen_fingerprints: Vec<u64>,
    recent_calls: VecDeque<String>,
    poll_budget: usize,
    poll_count: usize,
    max_identical: usize,
    max_ping_pong: usize,
    fuzzy_threshold: u32,
}

impl LoopGuard {
    pub fn new(max_identical: usize, max_ping_pong: usize, poll_budget: usize) -> Self {
        Self {
            seen_hashes: HashSet::new(),
            seen_fingerprints: Vec::new(),
            recent_calls: VecDeque::new(),
            poll_budget,
            poll_count: 0,
            max_identical,
            max_ping_pong,
            fuzzy_threshold: 3, // Allow up to 3 bits difference
        }
    }

    /// Check a tool call for loop patterns. Returns an error message if a loop is detected.
    pub fn check(&mut self, tool_name: &str, arguments: &str) -> Option<String> {
        let hash = self.hash_call(tool_name, arguments);

        // Check identical calls (exact)
        if self.seen_hashes.contains(&hash) {
            return Some(format!(
                "Loop detected: identical call to '{}' with same arguments",
                tool_name
            ));
        }
        self.seen_hashes.insert(hash);

        // --- FUZZY MATCHING (SimHash) ---
        // We only apply this to arguments that are large enough to be meaningful
        if arguments.len() > 100 {
            let fp = self.calculate_simhash(arguments);
            for &seen_fp in &self.seen_fingerprints {
                if self.hamming_distance(fp, seen_fp) <= self.fuzzy_threshold {
                    return Some(format!(
                        "Loop detected: call to '{}' is semantically identical to a previous call.",
                        tool_name
                    ));
                }
            }
            self.seen_fingerprints.push(fp);
        }

        // Check ping-pong pattern (A-B-A-B)
        self.recent_calls.push_back(tool_name.to_string());
        if self.recent_calls.len() > self.max_ping_pong * 2 {
            self.recent_calls.pop_front();
        }

        if self.recent_calls.len() >= 4 {
            let len = self.recent_calls.len();
            let calls: Vec<&String> = self.recent_calls.iter().collect();
            if len >= 4
                && calls[len - 1] == calls[len - 3]
                && calls[len - 2] == calls[len - 4]
                && calls[len - 1] != calls[len - 2]
            {
                return Some(format!(
                    "Ping-pong loop detected between '{}' and '{}'",
                    calls[len - 1],
                    calls[len - 2]
                ));
            }
        }

        // Check poll budget
        self.poll_count += 1;
        if self.poll_count > self.poll_budget {
            return Some(format!(
                "Poll budget exceeded: {} calls made (budget: {})",
                self.poll_count, self.poll_budget
            ));
        }

        None
    }

    /// Check an observation (tool output) for repeating semantic states.
    pub fn check_observation(&mut self, content: &str) -> Option<String> {
        if content.len() < 200 {
            return None;
        }

        let fp = self.calculate_simhash(content);
        for &seen_fp in &self.seen_fingerprints {
            if self.hamming_distance(fp, seen_fp) <= self.fuzzy_threshold {
                return Some("Loop detected: webpage structure is semantically identical to a previous state.".to_string());
            }
        }
        self.seen_fingerprints.push(fp);
        None
    }

    pub fn reset(&mut self) {
        self.seen_hashes.clear();
        self.recent_calls.clear();
        self.poll_count = 0;
    }

    fn hash_call(&self, tool_name: &str, arguments: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(tool_name.as_bytes());
        hasher.update(b"|");
        hasher.update(arguments.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Calculate a 64-bit SimHash fingerprint for fuzzy matching.
    pub fn calculate_simhash(&self, text: &str) -> u64 {
        let mut v = [0i32; 64];
        // Simple word-based shingling
        for word in text.split_whitespace() {
            let mut h = Sha256::new();
            h.update(word.as_bytes());
            let hash_bytes = h.finalize();
            
            // Use the first 8 bytes as a 64-bit hash
            let mut hash_val = 0u64;
            for i in 0..8 {
                hash_val = (hash_val << 8) | hash_bytes[i] as u64;
            }

            for i in 0..64 {
                if (hash_val >> i) & 1 == 1 {
                    v[i] += 1;
                } else {
                    v[i] -= 1;
                }
            }
        }

        let mut fingerprint = 0u64;
        for i in 0..64 {
            if v[i] > 0 {
                fingerprint |= 1 << i;
            }
        }
        fingerprint
    }

    /// Calculate Hamming distance between two fingerprints.
    pub fn hamming_distance(&self, f1: u64, f2: u64) -> u32 {
        (f1 ^ f2).count_ones()
    }
}

impl Default for LoopGuard {
    fn default() -> Self {
        Self::new(50, 4, 100)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identical_call_detection() {
        let mut guard = LoopGuard::default();
        assert!(guard.check("calc", r#"{"expr":"2+2"}"#).is_none());
        assert!(guard.check("calc", r#"{"expr":"2+2"}"#).is_some());
    }

    #[test]
    fn test_different_calls_ok() {
        let mut guard = LoopGuard::default();
        assert!(guard.check("calc", r#"{"expr":"2+2"}"#).is_none());
        assert!(guard.check("calc", r#"{"expr":"3+3"}"#).is_none());
    }

    #[test]
    fn test_ping_pong_detection() {
        let mut guard = LoopGuard::new(100, 4, 100);
        assert!(guard.check("tool_a", "1").is_none());
        assert!(guard.check("tool_b", "2").is_none());
        assert!(guard.check("tool_a", "3").is_none());
        let result = guard.check("tool_b", "4");
        assert!(result.is_some());
        assert!(result.unwrap().contains("Ping-pong"));
    }

    #[test]
    fn test_poll_budget() {
        let mut guard = LoopGuard::new(1000, 100, 3);
        assert!(guard.check("t1", "a1").is_none());
        assert!(guard.check("t2", "a2").is_none());
        assert!(guard.check("t3", "a3").is_none());
        let result = guard.check("t4", "a4");
        assert!(result.is_some());
        assert!(result.unwrap().contains("Poll budget"));
    }
}
