use xylitol_tui::fuzzy::*;

#[test]
fn test_empty_query_matches_all() {
    let m = fuzzy_match("", "anything").unwrap();
    assert_eq!(m.score, 0.0);
}

#[test]
fn test_query_longer_than_text() {
    assert!(fuzzy_match("longquery", "short").is_none());
}

#[test]
fn test_exact_match_score() {
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
fn test_consecutive_better_than_scattered() {
    // consecutive: -2 per consecutive pair, start bonus -1, 3 bonus chars total = -(2*2) - 1 = -5, then +0.06 = ~-4.94
    // scattered: just 3 bonuses at word boundaries (-1 each = -3), +0.09 = ~-2.91
    let consecutive = fuzzy_match("foo", "foobar").unwrap();
    let scattered = fuzzy_match("foo", "f_o_o_bar").unwrap();
    assert!(
        consecutive.score < scattered.score,
        "consecutive={} should be < scattered={}",
        consecutive.score,
        scattered.score
    );
}

#[test]
fn test_filter_empty_returns_all() {
    let items = vec!["apple", "banana", "cherry"];
    let result = fuzzy_filter(&items, "", |s| s);
    assert_eq!(result, items);
}

#[test]
fn test_filter_removes_non_matches() {
    let items = vec!["apple", "banana", "cherry"];
    let result = fuzzy_filter(&items, "an", |s| s);
    assert!(result.contains(&"banana"));
    assert!(!result.contains(&"apple"));
}

#[test]
fn test_filter_sorts_by_quality() {
    let items = vec!["a_p_p", "app", "application"];
    let result = fuzzy_filter(&items, "app", |s| s);
    assert_eq!(result[0], "app");
}

#[test]
fn test_filter_prioritizes_exact() {
    let items = vec!["clone", "cl"];
    let result = fuzzy_filter(&items, "cl", |s| s);
    assert_eq!(result, vec!["cl", "clone"]);
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
    let result = fuzzy_filter(&items, "foo", |i| &i.name);
    assert_eq!(result.len(), 2);
    assert!(result.iter().any(|i| i.name == "foo"));
    assert!(result.iter().any(|i| i.name == "foobar"));
}
