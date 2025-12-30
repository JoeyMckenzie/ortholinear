use nucleo_matcher::{
    pattern::{CaseMatching, Normalization, Pattern},
    Config, Matcher, Utf32Str,
};

#[derive(Debug, Clone)]
pub struct FilteredItem<T> {
    pub item: T,
    pub original_index: usize,
    pub score: u32,
}

pub fn filter_items<T, F>(items: &[T], query: &str, get_text: F) -> Vec<FilteredItem<T>>
where
    T: Clone,
    F: Fn(&T) -> String,
{
    if query.is_empty() {
        return items
            .iter()
            .enumerate()
            .map(|(i, item)| FilteredItem {
                item: item.clone(),
                original_index: i,
                score: 0,
            })
            .collect();
    }

    let mut matcher = Matcher::new(Config::DEFAULT);
    let pattern = Pattern::parse(query, CaseMatching::Ignore, Normalization::Smart);

    let mut results: Vec<FilteredItem<T>> = items
        .iter()
        .enumerate()
        .filter_map(|(i, item)| {
            let text = get_text(item);
            let mut buf = Vec::new();
            let haystack = Utf32Str::new(&text, &mut buf);

            pattern
                .score(haystack, &mut matcher)
                .map(|score| FilteredItem {
                    item: item.clone(),
                    original_index: i,
                    score,
                })
        })
        .collect();

    results.sort_by(|a, b| b.score.cmp(&a.score));

    results
}
