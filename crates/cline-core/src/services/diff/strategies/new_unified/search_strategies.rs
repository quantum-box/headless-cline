use super::types::{Change, ChangeType, Hunk};
use strsim::normalized_levenshtein;

const LARGE_FILE_THRESHOLD: usize = 1000;
const UNIQUE_CONTENT_BOOST: f64 = 0.05;
const DEFAULT_OVERLAP_SIZE: usize = 3;
const MAX_WINDOW_SIZE: usize = 500;

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub index: i32,
    pub confidence: f64,
    pub strategy: String,
}

pub fn prepare_search_string(changes: &[Change]) -> String {
    changes
        .iter()
        .filter(|c| matches!(c.change_type, ChangeType::Context | ChangeType::Remove))
        .filter_map(|c| c.original_line.as_ref())
        .map(|s| s.as_str())
        .collect::<Vec<_>>()
        .join("\n")
}

fn evaluate_similarity(original: &str, modified: &str) -> f64 {
    normalized_levenshtein(original, modified)
}

fn get_adaptive_threshold(content_length: usize, base_threshold: f64) -> f64 {
    if content_length <= LARGE_FILE_THRESHOLD {
        base_threshold
    } else {
        (base_threshold - 0.07).max(0.8)
    }
}

fn evaluate_content_uniqueness(search_str: &str, content: &[String]) -> f64 {
    let search_lines: Vec<&str> = search_str.lines().collect();
    let unique_lines: std::collections::HashSet<_> = search_lines.iter().copied().collect();
    let content_str = content.join("\n");

    let mut unique_count = 0;
    for line in unique_lines.iter() {
        let escaped_line = regex::escape(line);
        if let Ok(re) = regex::Regex::new(&escaped_line) {
            if let Some(matches) = re.find_iter(&content_str).take(3).count().checked_sub(1) {
                if matches <= 1 {
                    unique_count += 1;
                }
            }
        }
    }

    unique_count as f64 / unique_lines.len() as f64
}

fn validate_context_lines(search_str: &str, content: &str, confidence_threshold: f64) -> f64 {
    let context_lines: Vec<_> = search_str
        .lines()
        .filter(|line| !line.starts_with('-'))
        .collect();

    let similarity = evaluate_similarity(&context_lines.join("\n"), content);
    let threshold = get_adaptive_threshold(content.lines().count(), confidence_threshold);
    let uniqueness_score = evaluate_content_uniqueness(
        search_str,
        &content.lines().map(String::from).collect::<Vec<_>>(),
    );
    let uniqueness_boost = uniqueness_score * UNIQUE_CONTENT_BOOST;

    if similarity < threshold {
        similarity * 0.3 + uniqueness_boost
    } else {
        similarity + uniqueness_boost
    }
}

fn create_overlapping_windows(
    content: &[String],
    search_size: usize,
    overlap_size: Option<usize>,
) -> Vec<(Vec<String>, usize)> {
    let overlap_size = overlap_size.unwrap_or(DEFAULT_OVERLAP_SIZE);
    let mut windows = Vec::new();

    let effective_window_size = search_size.max(search_size.min(MAX_WINDOW_SIZE) * 2);
    let effective_overlap_size = overlap_size.min(effective_window_size - 1);
    let step_size = (effective_window_size - effective_overlap_size).max(1);

    for i in (0..content.len()).step_by(step_size) {
        let window_content: Vec<_> = content[i..]
            .iter()
            .take(effective_window_size)
            .cloned()
            .collect();
        if window_content.len() >= search_size {
            windows.push((window_content, i));
        }
    }

    windows
}

pub fn find_exact_match(
    search_str: &str,
    content: &[String],
    start_index: usize,
    confidence_threshold: f64,
) -> SearchResult {
    let search_lines: Vec<_> = search_str.lines().collect();
    let windows =
        create_overlapping_windows(&content[start_index..].to_vec(), search_lines.len(), None);
    let mut best_result = SearchResult {
        index: -1,
        confidence: 0.0,
        strategy: "exact".to_string(),
    };

    for (window, window_index) in windows {
        let window_str = window.join("\n");
        if let Some(exact_match) = window_str.find(search_str) {
            let matched_content = window[exact_match..exact_match + search_lines.len()].join("\n");
            let similarity = evaluate_similarity(search_str, &matched_content);
            let context_similarity =
                validate_context_lines(search_str, &matched_content, confidence_threshold);
            let confidence = similarity.min(context_similarity);

            if confidence > best_result.confidence {
                best_result = SearchResult {
                    index: (start_index + window_index + exact_match) as i32,
                    confidence,
                    strategy: "exact".to_string(),
                };
            }
        }
    }

    best_result
}

pub fn find_similarity_match(
    search_str: &str,
    content: &[String],
    start_index: usize,
    confidence_threshold: f64,
) -> SearchResult {
    let search_lines: Vec<_> = search_str.lines().collect();
    let mut best_result = SearchResult {
        index: -1,
        confidence: 0.0,
        strategy: "similarity".to_string(),
    };

    for i in start_index..content.len().saturating_sub(search_lines.len() - 1) {
        let window_str = content[i..i + search_lines.len()].join("\n");
        let score = evaluate_similarity(search_str, &window_str);

        if score > best_result.confidence && score >= confidence_threshold {
            let context_similarity =
                validate_context_lines(search_str, &window_str, confidence_threshold);
            let adjusted_score = score.min(context_similarity);

            if adjusted_score > best_result.confidence {
                best_result = SearchResult {
                    index: i as i32,
                    confidence: adjusted_score,
                    strategy: "similarity".to_string(),
                };
            }
        }
    }

    best_result
}

pub fn find_best_match(
    search_str: &str,
    content: &[String],
    start_index: usize,
    confidence_threshold: f64,
) -> SearchResult {
    let strategies = [find_exact_match, find_similarity_match];
    let mut best_result = SearchResult {
        index: -1,
        confidence: 0.0,
        strategy: "none".to_string(),
    };

    for strategy in strategies.iter() {
        let result = strategy(search_str, content, start_index, confidence_threshold);
        if result.confidence > best_result.confidence {
            best_result = result;
        }
    }

    best_result
}

pub fn validate_edit_result(hunk: &Hunk, result: &str) -> f64 {
    let expected_text = hunk
        .changes
        .iter()
        .filter(|change| matches!(change.change_type, ChangeType::Context | ChangeType::Add))
        .map(|change| {
            if !change.indent.is_empty() {
                format!("{}{}", change.indent, change.content)
            } else {
                change.content.clone()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");

    let similarity = evaluate_similarity(&expected_text, result);

    let original_text = hunk
        .changes
        .iter()
        .filter(|change| matches!(change.change_type, ChangeType::Context | ChangeType::Remove))
        .map(|change| {
            if !change.indent.is_empty() {
                format!("{}{}", change.indent, change.content)
            } else {
                change.content.clone()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");

    let original_similarity = evaluate_similarity(&original_text, result);
    if original_similarity > 0.97 && similarity != 1.0 {
        0.8 * similarity
    } else {
        similarity
    }
}
