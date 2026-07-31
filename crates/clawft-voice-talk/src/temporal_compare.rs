//! Vendored midstream temporal-compare DP subset (WEFT-714 / Phase A).
//!
//! Lifted and trimmed from [ruvnet/midstream](https://github.com/ruvnet/midstream)
//! crate `midstreamer-temporal-compare` (MIT OR Apache-2.0). Only tick-safe
//! primitives remain:
//!
//! - Dynamic Time Warping (DTW)
//! - Longest Common Subsequence (LCS)
//! - Edit distance (Levenshtein)
//! - Sliding-window `find_similar` (normalised DTW)
//!
//! **Excluded** (not tick-safe / not needed): LRU/dashmap caches, O(n³)
//! `detect_recurring_patterns` / `detect_fuzzy_patterns`, serde wrappers.
//!
//! Window sizes on the CognitiveTick path must stay ≤
//! [`crate::TICK_WINDOW_CAP`] (and haystack ≤ [`crate::TICK_HAYSTACK_CAP`]).

// Attribution: midstreamer-temporal-compare © ruvnet/midstream contributors,
// dual-licensed MIT OR Apache-2.0. Algorithms are standard textbook DP;
// structure follows the upstream crate (reviewed 2026-07-03 / crates.io 0.2.1).

use std::cmp::Ordering;

/// Result of a sequence comparison (distance + optional DTW alignment path).
#[derive(Debug, Clone, PartialEq)]
pub struct ComparisonResult {
    /// Algorithm-specific distance (0 = identical for DTW/edit/LCS distance).
    pub distance: f64,
    /// Backtrack path for DTW as `(i, j)` index pairs, when computed.
    pub alignment: Option<Vec<(usize, usize)>>,
}

/// One sliding-window similarity hit.
#[derive(Debug, Clone, PartialEq)]
pub struct SimilarityMatch {
    /// Start index in the haystack.
    pub start_index: usize,
    /// Similarity in `(0, 1]` (higher is better); derived from distance.
    pub similarity: f64,
    /// Raw DTW distance (lower is better).
    pub distance: f64,
}

impl SimilarityMatch {
    /// Build from start index and DTW distance (inverse-exp similarity).
    pub fn new(start_index: usize, distance: f64) -> Self {
        let similarity = (-distance / 10.0).exp();
        Self {
            start_index,
            similarity,
            distance,
        }
    }
}

/// Dynamic Time Warping over equal-cost (match=0 / mismatch=1) symbols.
///
/// O(n·m) full matrix with backtrack. Empty sequences: distance = n + m.
pub fn dtw<T: PartialEq>(a: &[T], b: &[T]) -> ComparisonResult {
    let n = a.len();
    let m = b.len();
    if n == 0 || m == 0 {
        return ComparisonResult {
            distance: (n + m) as f64,
            alignment: None,
        };
    }

    let mut mat = vec![vec![f64::INFINITY; m + 1]; n + 1];
    mat[0][0] = 0.0;

    for i in 1..=n {
        for j in 1..=m {
            let cost = if a[i - 1] == b[j - 1] { 0.0 } else { 1.0 };
            mat[i][j] = cost + mat[i - 1][j - 1].min(mat[i - 1][j]).min(mat[i][j - 1]);
        }
    }

    let mut alignment = Vec::new();
    let (mut i, mut j) = (n, m);
    while i > 0 && j > 0 {
        alignment.push((i - 1, j - 1));
        let min_val = mat[i - 1][j - 1].min(mat[i - 1][j]).min(mat[i][j - 1]);
        if mat[i - 1][j - 1] == min_val {
            i -= 1;
            j -= 1;
        } else if mat[i - 1][j] == min_val {
            i -= 1;
        } else {
            j -= 1;
        }
    }
    alignment.reverse();

    ComparisonResult {
        distance: mat[n][m],
        alignment: Some(alignment),
    }
}

/// Longest Common Subsequence length, returned as a *distance*
/// `(n + m − 2·lcs)` so identical sequences score 0.
pub fn lcs_distance<T: PartialEq>(a: &[T], b: &[T]) -> ComparisonResult {
    let n = a.len();
    let m = b.len();
    let mut dp = vec![vec![0usize; m + 1]; n + 1];
    for i in 1..=n {
        for j in 1..=m {
            if a[i - 1] == b[j - 1] {
                dp[i][j] = dp[i - 1][j - 1] + 1;
            } else {
                dp[i][j] = dp[i - 1][j].max(dp[i][j - 1]);
            }
        }
    }
    let lcs_len = dp[n][m];
    ComparisonResult {
        distance: (n + m - 2 * lcs_len) as f64,
        alignment: None,
    }
}

/// Classic Levenshtein edit distance.
pub fn edit_distance<T: PartialEq>(a: &[T], b: &[T]) -> ComparisonResult {
    let n = a.len();
    let m = b.len();
    let mut dp = vec![vec![0usize; m + 1]; n + 1];

    // Textbook matrix init — range loops are clearer than iter_mut rewrites.
    #[allow(clippy::needless_range_loop)]
    for i in 0..=n {
        dp[i][0] = i;
    }
    #[allow(clippy::needless_range_loop)]
    for j in 0..=m {
        dp[0][j] = j;
    }

    for i in 1..=n {
        for j in 1..=m {
            let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
            dp[i][j] = (dp[i - 1][j] + 1)
                .min(dp[i][j - 1] + 1)
                .min(dp[i - 1][j - 1] + cost);
        }
    }

    ComparisonResult {
        distance: dp[n][m] as f64,
        alignment: None,
    }
}

/// Character-level Levenshtein over two strings (for last-token refine).
pub fn edit_distance_str(a: &str, b: &str) -> usize {
    let aa: Vec<char> = a.chars().collect();
    let bb: Vec<char> = b.chars().collect();
    edit_distance(&aa, &bb).distance as usize
}

/// Sliding-window DTW match of `needle` inside `haystack`.
///
/// Keeps windows where `distance / needle.len() < threshold` (exclusive upper
/// bound — a full-mismatch window is not a hit). Results sorted by distance.
///
/// Callers **must** cap `haystack` / `needle` (see [`crate::TICK_HAYSTACK_CAP`]).
pub fn find_similar<T: PartialEq>(
    haystack: &[T],
    needle: &[T],
    threshold: f64,
) -> Vec<SimilarityMatch> {
    if needle.is_empty() || haystack.len() < needle.len() {
        return Vec::new();
    }
    let needle_len = needle.len();
    let mut matches = Vec::new();
    for start_idx in 0..=(haystack.len() - needle_len) {
        let window = &haystack[start_idx..start_idx + needle_len];
        let result = dtw(window, needle);
        let normalised = result.distance / needle_len as f64;
        if normalised < threshold {
            matches.push(SimilarityMatch::new(start_idx, result.distance));
        }
    }
    matches.sort_by(|a, b| a.distance.partial_cmp(&b.distance).unwrap_or(Ordering::Equal));
    matches
}

/// True when `needle` appears at least once under `threshold`.
pub fn detect_pattern<T: PartialEq>(series: &[T], needle: &[T], threshold: f64) -> bool {
    !find_similar(series, needle, threshold).is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dtw_identical_is_zero() {
        let a = [1, 2, 3];
        let b = [1, 2, 3];
        let r = dtw(&a, &b);
        assert_eq!(r.distance, 0.0);
        assert!(r.alignment.is_some());
    }

    #[test]
    fn dtw_empty() {
        let r = dtw::<i32>(&[], &[1]);
        assert_eq!(r.distance, 1.0);
    }

    #[test]
    fn edit_distance_basic() {
        let a: Vec<char> = "kitten".chars().collect();
        let b: Vec<char> = "sitting".chars().collect();
        assert_eq!(edit_distance(&a, &b).distance, 3.0);
        assert_eq!(edit_distance_str("kitten", "sitting"), 3);
    }

    #[test]
    fn lcs_identical_distance_zero() {
        let a = ['a', 'b', 'c'];
        let b = ['a', 'b', 'c'];
        assert_eq!(lcs_distance(&a, &b).distance, 0.0);
    }

    #[test]
    fn find_similar_exact_hits() {
        let haystack = [1, 2, 3, 4, 5, 3, 4, 5];
        let needle = [3, 4, 5];
        let hits = find_similar(&haystack, &needle, 0.1);
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].start_index, 2);
        assert_eq!(hits[1].start_index, 5);
        assert!(hits[0].similarity > 0.9);
    }

    #[test]
    fn find_similar_threshold_excludes_full_mismatch() {
        let hay = ['a', 'b', 'c'];
        let needle = ['x', 'y', 'z'];
        // normalised distance = 1.0 for full mismatch → not < 1.0
        assert!(find_similar(&hay, &needle, 1.0).is_empty());
    }

    #[test]
    fn detect_pattern_true_on_repeat() {
        let series = ['u', 'm', 'x', 'u', 'm'];
        let needle = ['u', 'm'];
        assert!(detect_pattern(&series, &needle, 0.1));
    }
}
