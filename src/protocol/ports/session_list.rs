//! Session resume list helpers (preview flatten / relative age).

use super::SessionListEntry;

/// Format a unix-secs age for session selectors (`6m`, `19h`, `1d`, …).
pub fn format_session_age(modified_unix: Option<u64>, now_unix: u64) -> String {
    let Some(ts) = modified_unix else {
        return "—".into();
    };
    let diff = now_unix.saturating_sub(ts);
    let mins = diff / 60;
    let hours = diff / 3600;
    let days = diff / 86400;
    if mins < 1 {
        "now".into()
    } else if mins < 60 {
        format!("{mins}m")
    } else if hours < 24 {
        format!("{hours}h")
    } else if days < 7 {
        format!("{days}d")
    } else if days < 30 {
        format!("{}w", days / 7)
    } else if days < 365 {
        format!("{}mo", days / 30)
    } else {
        format!("{}y", days / 365)
    }
}

/// Re-order sessions into a parent/child forest (pi "threaded") and set `tree_prefix`.
///
/// Roots = no parent, or parent missing from the list. Siblings keep mtime order
/// (input should already be mtime-desc among roots / children).
pub fn flatten_session_forest(entries: Vec<SessionListEntry>) -> Vec<SessionListEntry> {
    if entries.is_empty() {
        return entries;
    }
    let by_id: std::collections::HashMap<String, usize> = entries
        .iter()
        .enumerate()
        .map(|(i, e)| (e.id.clone(), i))
        .collect();

    let mut children: std::collections::HashMap<String, Vec<usize>> =
        std::collections::HashMap::new();
    let mut roots: Vec<usize> = Vec::new();
    for (i, e) in entries.iter().enumerate() {
        match e.parent_session_id.as_deref() {
            Some(p) if by_id.contains_key(p) => {
                children.entry(p.to_string()).or_default().push(i);
            }
            _ => roots.push(i),
        }
    }

    // Preserve mtime-desc among siblings (list_sessions already sorted that way).
    let mut out_idx: Vec<(usize /*entry_idx*/, String /*prefix*/)> = Vec::new();
    fn walk(
        idx: usize,
        prefix: String,
        children: &std::collections::HashMap<String, Vec<usize>>,
        entries: &[SessionListEntry],
        out: &mut Vec<(usize, String)>,
    ) {
        out.push((idx, prefix));
        let id = &entries[idx].id;
        let Some(kids) = children.get(id) else {
            return;
        };
        for (k, &child) in kids.iter().enumerate() {
            let is_last = k + 1 == kids.len();
            let branch = if is_last { "└─ " } else { "├─ " };
            // Indent with spaces under ancestors (pi uses │ for continuing; MVP: spaces).
            let child_prefix = format!("   {branch}");
            walk(child, child_prefix, children, entries, out);
        }
    }
    for &r in &roots {
        walk(r, String::new(), &children, &entries, &mut out_idx);
    }

    // Orphans already in roots; if something was missed, append.
    if out_idx.len() < entries.len() {
        let seen: std::collections::HashSet<usize> = out_idx.iter().map(|(i, _)| *i).collect();
        for i in 0..entries.len() {
            if !seen.contains(&i) {
                out_idx.push((i, String::new()));
            }
        }
    }

    out_idx
        .into_iter()
        .map(|(i, prefix)| {
            let mut e = entries[i].clone();
            e.tree_prefix = prefix;
            e
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(id: &str, parent: Option<&str>, mtime: u64) -> SessionListEntry {
        SessionListEntry {
            id: id.into(),
            name: None,
            first_message: Some(format!("msg-{id}")),
            message_count: 1,
            modified_unix: Some(mtime),
            parent_session_id: parent.map(str::to_string),
            tree_prefix: String::new(),
            cwd: None,
            path: None,
        }
    }

    #[test]
    fn age_buckets() {
        assert_eq!(format_session_age(Some(100), 100), "now");
        assert_eq!(format_session_age(Some(100), 100 + 5 * 60), "5m");
        assert_eq!(format_session_age(Some(100), 100 + 3 * 3600), "3h");
        assert_eq!(format_session_age(Some(100), 100 + 2 * 86400), "2d");
    }

    #[test]
    fn forest_puts_child_under_parent() {
        // mtime-desc input: child newer than parent still nests under parent.
        let flat = flatten_session_forest(vec![
            row("child", Some("parent"), 200),
            row("parent", None, 100),
            row("other", None, 50),
        ]);
        let ids: Vec<_> = flat.iter().map(|e| e.id.as_str()).collect();
        assert_eq!(ids, vec!["parent", "child", "other"]);
        assert!(flat[1].tree_prefix.contains('─'));
        assert!(flat[0].tree_prefix.is_empty());
    }
}
