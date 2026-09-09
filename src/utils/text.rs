//! Pure text utilities (XML escape, etc.).

/// Escape a string for safe inclusion in XML/HTML text content.
pub fn xml_escape(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => result.push_str("&amp;"),
            '<' => result.push_str("&lt;"),
            '>' => result.push_str("&gt;"),
            '"' => result.push_str("&quot;"),
            '\'' => result.push_str("&apos;"),
            _ => result.push(c),
        }
    }
    result
}

/// Format a byte count as human-readable size (`B` / `KB` / `MB`).
pub fn format_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{bytes}B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1}KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1}MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_xml_special_chars() {
        assert_eq!(xml_escape("a < b & c > d"), "a &lt; b &amp; c &gt; d");
        assert_eq!(xml_escape("\"quote\""), "&quot;quote&quot;");
        assert_eq!(xml_escape("it's"), "it&apos;s");
    }

    #[test]
    fn passes_through_plain_text() {
        assert_eq!(xml_escape("hello world"), "hello world");
    }

    #[test]
    fn format_size_units() {
        assert_eq!(format_size(500), "500B");
        assert_eq!(format_size(2048), "2.0KB");
        assert_eq!(format_size(2_097_152), "2.0MB");
    }
}
