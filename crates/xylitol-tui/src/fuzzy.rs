/// FuzzyMatch result.
#[derive(Debug, Clone)]
pub struct FuzzyMatch {
    pub score: f64,
    pub indices: Vec<usize>,
}

/// Fuzzy match: returns a FuzzyMatch or None.
/// Lower (more negative) scores = better match.
pub fn fuzzy_match(query: &str, text: &str) -> Option<FuzzyMatch> {
    if query.is_empty() {
        return Some(FuzzyMatch {
            score: 0.0,
            indices: vec![],
        });
    }
    if query.len() > text.len() {
        return None;
    }

    let text_lower = text.to_lowercase();
    let query_lower = query.to_lowercase();
    let text_chars: Vec<char> = text_lower.chars().collect();
    let query_chars: Vec<char> = query_lower.chars().collect();

    let mut score = 0.0f64;
    let mut indices: Vec<usize> = Vec::new();
    let mut query_idx = 0usize;
    let mut prev_match: Option<usize> = None;

    for (i, tc) in text_chars.iter().enumerate() {
        if query_idx < query_chars.len() && *tc == query_chars[query_idx] {
            indices.push(i);

            // Consecutive bonus: -2
            if let Some(prev) = prev_match
                && i == prev + 1
            {
                score -= 2.0;
            }

            // Word boundary bonus: occurs after separator or case transition
            let is_word_start = i == 0
                || (i > 0
                    && text
                        .chars()
                        .nth(i - 1)
                        .is_some_and(|c| c == '_' || c == '-' || c == '.' || c == ' ' || c == '/'))
                || (i > 0
                    && text.chars().nth(i).is_some_and(|c| c.is_uppercase())
                    && text.chars().nth(i - 1).is_some_and(|c| c.is_lowercase()));

            if i == 0 {
                score -= 1.0; // Start of string bonus
            } else if is_word_start {
                score -= 1.0; // Word boundary bonus
            }

            prev_match = Some(i);
            query_idx += 1;
        }
    }

    if query_idx < query_chars.len() {
        return None;
    }

    // Length penalty: +0.01 per char (positive = worse)
    score += text_chars.len() as f64 * 0.01;

    Some(FuzzyMatch { score, indices })
}

/// Fuzzy filter: returns items that match, sorted by best (lowest) score first.
/// `get_text` extracts the string to match against from each item.
pub fn fuzzy_filter<T>(items: &[T], pattern: &str, get_text: impl Fn(&T) -> &str) -> Vec<T>
where
    T: Clone,
{
    if pattern.is_empty() {
        return items.to_vec();
    }

    let mut matches: Vec<(usize, FuzzyMatch)> = Vec::new();
    for (i, item) in items.iter().enumerate() {
        let text = get_text(item);
        if let Some(m) = fuzzy_match(pattern, text) {
            matches.push((i, m));
        }
    }

    // Sort by score ascending (lower = better)
    matches.sort_by(|a, b| {
        a.1.score
            .partial_cmp(&b.1.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    matches.into_iter().map(|(i, _)| items[i].clone()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_query_matches_all() {
        let m = fuzzy_match("", "anything");
        assert!(m.is_some());
        assert_eq!(m.unwrap().score, 0.0);
    }

    #[test]
    fn test_query_longer_than_text() {
        assert!(fuzzy_match("longquery", "short").is_none());
    }

    #[test]
    fn test_exact_match() {
        let m = fuzzy_match("test", "test").unwrap();
        assert!(m.score < 0.0);
    }

    #[test]
    fn test_chars_in_order() {
        assert!(fuzzy_match("abc", "aXbXc").is_some());
        assert!(fuzzy_match("abc", "cba").is_none());
    }

    #[test]
    fn test_case_insensitive() {
        assert!(fuzzy_match("abc", "ABC").is_some());
        assert!(fuzzy_match("ABC", "abc").is_some());
    }

    #[test]
    fn test_consecutive_better() {
        let c = fuzzy_match("foo", "foobar").unwrap();
        let s = fuzzy_match("foo", "f_o_o_bar").unwrap();
        assert!(c.score < s.score);
    }

    #[test]
    fn test_filter_empty() {
        let items = vec!["apple", "banana"];
        let r = fuzzy_filter(&items, "", |s| s);
        assert_eq!(r, items);
    }

    #[test]
    fn test_filter_removes() {
        let items = vec!["apple", "banana"];
        let r = fuzzy_filter(&items, "an", |s| s);
        assert!(r.contains(&"banana"));
        assert!(!r.contains(&"apple"));
    }

    #[test]
    fn test_filter_sort() {
        let items = vec!["a_p_p", "app"];
        let r = fuzzy_filter(&items, "app", |s| s);
        assert_eq!(r[0], "app");
    }

    #[test]
    fn test_filter_exact_first() {
        let items = vec!["clone", "cl"];
        let r = fuzzy_filter(&items, "cl", |s| s);
        assert_eq!(r, vec!["cl", "clone"]);
    }

    #[test]
    fn test_filter_custom_get_text() {
        #[derive(Clone, PartialEq, Debug)]
        struct Item {
            name: String,
            id: i32,
        }
        let items = vec![
            Item {
                name: "foo".into(),
                id: 1,
            },
            Item {
                name: "bar".into(),
                id: 2,
            },
            Item {
                name: "foobar".into(),
                id: 3,
            },
        ];
        let r = fuzzy_filter(&items, "foo", |i| &i.name);
        assert_eq!(r.len(), 2);
        assert!(r.iter().any(|i| i.name == "foo"));
        assert!(r.iter().any(|i| i.name == "foobar"));
    }
}
