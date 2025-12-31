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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_query_returns_all_items() {
        let items = vec!["apple", "banana", "cherry"];
        let results = filter_items(&items, "", |s| s.to_string());

        assert_eq!(results.len(), 3);
        assert_eq!(results[0].item, "apple");
        assert_eq!(results[0].original_index, 0);
        assert_eq!(results[1].item, "banana");
        assert_eq!(results[1].original_index, 1);
        assert_eq!(results[2].item, "cherry");
        assert_eq!(results[2].original_index, 2);
    }

    #[test]
    fn filters_matching_items() {
        let items = vec!["apple", "banana", "apricot", "cherry"];
        let results = filter_items(&items, "ap", |s| s.to_string());

        assert_eq!(results.len(), 2);
        let matched: Vec<_> = results.iter().map(|r| r.item).collect();
        assert!(matched.contains(&"apple"));
        assert!(matched.contains(&"apricot"));
    }

    #[test]
    fn no_matches_returns_empty() {
        let items = vec!["apple", "banana", "cherry"];
        let results = filter_items(&items, "xyz", |s| s.to_string());

        assert!(results.is_empty());
    }

    #[test]
    fn case_insensitive_matching() {
        let items = vec!["Apple", "BANANA", "cherry"];
        let results = filter_items(&items, "apple", |s| s.to_string());

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].item, "Apple");
    }

    #[test]
    fn fuzzy_matching_works() {
        let items = vec!["user_service", "user_repository", "product_service"];
        let results = filter_items(&items, "usvc", |s| s.to_string());

        // Should match "user_service" with fuzzy matching
        assert!(!results.is_empty());
        assert_eq!(results[0].item, "user_service");
    }

    #[test]
    fn preserves_original_indices() {
        let items = vec!["aaa", "bbb", "aab", "ccc"];
        let results = filter_items(&items, "aa", |s| s.to_string());

        // Should find "aaa" (index 0) and "aab" (index 2)
        assert_eq!(results.len(), 2);

        let indices: Vec<_> = results.iter().map(|r| r.original_index).collect();
        assert!(indices.contains(&0));
        assert!(indices.contains(&2));
    }

    #[test]
    fn sorts_by_score_descending() {
        let items = vec!["zzz_test_zzz", "test", "a_test"];
        let results = filter_items(&items, "test", |s| s.to_string());

        // All should match, and results should be sorted by score
        assert_eq!(results.len(), 3);

        // Verify scores are in descending order
        for i in 1..results.len() {
            assert!(
                results[i - 1].score >= results[i].score,
                "Results should be sorted by score descending"
            );
        }
    }

    #[test]
    fn works_with_custom_text_extractor() {
        #[derive(Clone, Debug, PartialEq)]
        struct Issue {
            id: String,
            title: String,
        }

        let items = vec![
            Issue {
                id: "ENG-1".to_string(),
                title: "Fix bug".to_string(),
            },
            Issue {
                id: "ENG-2".to_string(),
                title: "Add feature".to_string(),
            },
            Issue {
                id: "ENG-3".to_string(),
                title: "Fix login".to_string(),
            },
        ];

        let results = filter_items(&items, "fix", |issue| {
            format!("{} {}", issue.id, issue.title)
        });

        assert_eq!(results.len(), 2);
        let ids: Vec<_> = results.iter().map(|r| r.item.id.as_str()).collect();
        assert!(ids.contains(&"ENG-1"));
        assert!(ids.contains(&"ENG-3"));
    }

    #[test]
    fn empty_items_returns_empty() {
        let items: Vec<String> = vec![];
        let results = filter_items(&items, "test", |s| s.clone());

        assert!(results.is_empty());
    }
}
