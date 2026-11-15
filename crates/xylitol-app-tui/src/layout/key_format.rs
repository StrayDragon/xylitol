pub(crate) fn format_keys(keys: &[String]) -> String {
    if keys.is_empty() {
        return String::new();
    }
    let pretty: Vec<String> = keys.iter().map(|key| pretty_key(key)).collect();
    compact_keys(&pretty)
}

fn compact_keys(keys: &[String]) -> String {
    if keys.len() == 1 {
        return keys[0].clone();
    }
    let parts: Vec<(String, String)> = keys
        .iter()
        .map(|key| match key.rfind('+') {
            Some(i) => (key[..=i].to_string(), key[i + 1..].to_string()),
            None => (String::new(), key.clone()),
        })
        .collect();
    let prefix = parts[0].0.clone();
    if !prefix.is_empty() && parts.iter().all(|part| part.0 == prefix) {
        format!(
            "{prefix}{}",
            parts
                .iter()
                .map(|part| part.1.as_str())
                .collect::<Vec<_>>()
                .join("/")
        )
    } else {
        keys.join(" / ")
    }
}

fn pretty_key(raw: &str) -> String {
    let mut parts: Vec<&str> = raw.split('+').collect();
    let last = parts.pop().unwrap_or(raw);
    let last = match last {
        "escape" => "Esc".to_string(),
        "enter" => "Enter".to_string(),
        "tab" => "Tab".to_string(),
        "backspace" => "Backspace".to_string(),
        "pageUp" => "PgUp".to_string(),
        "pageDown" => "PgDn".to_string(),
        "up" => "↑".to_string(),
        "down" => "↓".to_string(),
        "left" => "←".to_string(),
        "right" => "→".to_string(),
        "home" => "Home".to_string(),
        "end" => "End".to_string(),
        "delete" => "Del".to_string(),
        "space" => "Space".to_string(),
        other if other.chars().count() == 1 => other.to_uppercase(),
        other => other.to_string(),
    };
    let mods: Vec<&str> = parts
        .into_iter()
        .map(|part| match part {
            "ctrl" => "Ctrl",
            "alt" => "Alt",
            "shift" => "Shift",
            other => other,
        })
        .collect();
    if mods.is_empty() {
        last
    } else {
        format!("{}+{}", mods.join("+"), last)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compacts_shared_modifier_prefix() {
        assert_eq!(
            format_keys(&["ctrl+up".into(), "ctrl+down".into()]),
            "Ctrl+↑/↓"
        );
    }

    #[test]
    fn keeps_distinct_prefixes_separated() {
        assert_eq!(
            format_keys(&["ctrl+left".into(), "alt+left".into()]),
            "Ctrl+← / Alt+←"
        );
    }

    #[test]
    fn maps_named_keys() {
        assert_eq!(pretty_key("pageUp"), "PgUp");
        assert_eq!(pretty_key("escape"), "Esc");
        assert_eq!(pretty_key("shift+tab"), "Shift+Tab");
    }
}
