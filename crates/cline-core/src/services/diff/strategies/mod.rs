mod new_unified;
mod search_replace;
mod unified;

pub use new_unified::NewUnifiedDiffStrategy;
pub use search_replace::SearchReplaceDiffStrategy;

use crate::services::diff::types::*;

#[allow(dead_code)]
pub fn get_diff_strategy(
    _model: &str,
    fuzzy_match_threshold: Option<f64>,
    experimental_diff_strategy: bool,
) -> Box<dyn DiffStrategy> {
    if experimental_diff_strategy {
        Box::new(NewUnifiedDiffStrategy::new(fuzzy_match_threshold))
    } else {
        Box::new(SearchReplaceDiffStrategy::new(fuzzy_match_threshold, None))
    }
}
