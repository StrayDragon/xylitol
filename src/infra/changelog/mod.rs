//! CHANGELOG.md parsing.

#![allow(dead_code)]

/// A parsed changelog entry.
#[derive(Debug, Clone)]
pub struct ChangelogEntry {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
    pub content: String,
}

/// Parse a CHANGELOG.md file into versioned entries.
pub fn parse_changelog(content: &str) -> Vec<ChangelogEntry> {
    let mut entries = Vec::new();
    let mut current_version: Option<(u32, u32, u32)> = None;
    let mut current_lines: Vec<String> = Vec::new();

    for line in content.lines() {
        if line.starts_with("## ") {
            // Save previous entry
            if let Some((maj, min, pat)) = current_version.take() {
                if !current_lines.is_empty() {
                    entries.push(ChangelogEntry {
                        major: maj,
                        minor: min,
                        patch: pat,
                        content: current_lines.join("\n"),
                    });
                }
                current_lines.clear();
            }

            // Try to parse version from header
            if let Some(caps) = parse_version_header(line) {
                current_version = Some(caps);
            }
        } else if current_version.is_some() {
            current_lines.push(line.to_string());
        }
    }

    // Last entry
    if let Some((maj, min, pat)) = current_version {
        if !current_lines.is_empty() {
            entries.push(ChangelogEntry {
                major: maj,
                minor: min,
                patch: pat,
                content: current_lines.join("\n"),
            });
        }
    }

    entries
}

fn parse_version_header(line: &str) -> Option<(u32, u32, u32)> {
    let re = regex::Regex::new(r"##\s+\[?(\d+)\.(\d+)\.(\d+)\]?").ok()?;
    let caps = re.captures(line)?;
    Some((
        caps[1].parse().ok()?,
        caps[2].parse().ok()?,
        caps[3].parse().ok()?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_changelog_basic() {
        let content = "## [1.0.0]\n\n- Initial release\n- Feature two\n\n## [0.9.0]\n\n- Beta features";
        let entries = parse_changelog(content);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].major, 1);
        assert_eq!(entries[0].minor, 0);
        assert_eq!(entries[0].patch, 0);
        assert!(entries[1].content.contains("Beta"));
    }

    #[test]
    fn test_parse_changelog_empty() {
        let entries = parse_changelog("");
        assert!(entries.is_empty());
    }
}
