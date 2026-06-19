//! Prompt templates — `/template:name args` expansion system.
//!
//! Aligns with pi's prompt-templates.ts. Provides:
//! - Template loading from .md files with YAML frontmatter
//! - Positional argument substitution ($1, $2, ...)
//! - All-args placeholder ($@, $ARGUMENTS)
//! - Default-value syntax (${N:-default})
//! - `/template:name` line extraction

/// A loaded prompt template.
///
/// Templates are markdown files with optional YAML frontmatter.
/// The body uses positional placeholders: `$1`, `$2`, `$@`, `${N:-default}`.
#[derive(Debug, Clone)]
pub(crate) struct PromptTemplate {
    /// Template name (derived from filename).
    pub(crate) name: String,
    /// Markdown body with placeholders.
    pub(crate) body: String,
    /// Optional description from frontmatter.
    #[allow(dead_code)]
    pub(crate) description: Option<String>,
    /// Optional argument hint from frontmatter.
    #[allow(dead_code)]
    pub(crate) argument_hint: Option<String>,
    /// Originating file path (provenance tracing).
    #[allow(dead_code)]
    pub(crate) source_path: Option<std::path::PathBuf>,
}

impl PromptTemplate {
    /// Create a new template.
    #[allow(dead_code)]
    pub(crate) fn new(name: impl Into<String>, body: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            body: body.into(),
            description: None,
            argument_hint: None,
            source_path: None,
        }
    }

    /// Substitute positional arguments into the template body.
    ///
    /// Supports:
    /// - `$1`, `$2`, ... `$N` → positional argument (1-indexed)
    /// - `$@` or `$ARGUMENTS` → all arguments joined with spaces
    /// - `${N:-default}` → positional argument with default value
    ///
    /// Unknown `${...}` patterns are left unchanged.
    pub(crate) fn substitute_args(&self, args: &[String]) -> String {
        substitute_template_args(&self.body, args)
    }

    /// Expand by substituting args and appending a soft-newline if needed.
    pub(crate) fn expand(&self, args: &[String]) -> String {
        let content = self.substitute_args(args);
        // Ensure trailing newline for clean prompt injection
        if content.ends_with('\n') {
            content
        } else {
            format!("{content}\n")
        }
    }
}

/// Substitute positional arguments into a template string.
///
/// Handles the full substitution grammar:
/// - `$1`..`$N`: positional argument (1-indexed, empty string if out of range)
/// - `$@`, `$ARGUMENTS`: all arguments space-joined
/// - `${N:-default}`: positional with default
pub(crate) fn substitute_template_args(template: &str, args: &[String]) -> String {
    let mut result = String::with_capacity(template.len() + args.len() * 20);
    let chars: Vec<char> = template.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        if chars[i] == '$' && i + 1 < len {
            match chars[i + 1] {
                '0'..='9' => {
                    // $N — positional argument
                    let num: usize =
                        chars[i + 1].to_digit(10).expect("digit: matched '0'..='9'") as usize;
                    let arg = args
                        .get(num.wrapping_sub(1))
                        .map(|s| s.as_str())
                        .unwrap_or("");
                    result.push_str(arg);
                    i += 2;
                    continue;
                }
                '@' => {
                    // $@ — all args
                    result.push_str(&args.join(" "));
                    i += 2;
                    continue;
                }
                '{' => {
                    // ${N:-default} or ${ARGUMENTS}
                    let start = i;
                    i += 2; // skip ${
                    let mut pattern = String::new();
                    while i < len && chars[i] != '}' {
                        pattern.push(chars[i]);
                        i += 1;
                    }
                    if i < len {
                        i += 1; // skip }
                    }

                    let substituted = expand_braced_pattern(&pattern, args);
                    result.push_str(&substituted);
                    // Check for $ARGUMENTS as a special case (not in braces)
                    i = if substituted.is_empty() && pattern.is_empty() {
                        // malformed ${} — leave as-is
                        result.push_str(&template[start..i.min(len)]);
                        i.min(len)
                    } else {
                        i
                    };
                    continue;
                }
                _ => {
                    // Check for $ARGUMENTS (case-sensitive)
                    if template[i..].starts_with("$ARGUMENTS") {
                        result.push_str(&args.join(" "));
                        i += 10; // "$ARGUMENTS".len()
                        continue;
                    }
                }
            }
        }

        // Special case: $ARGUMENTS
        if template[i..].starts_with("$ARGUMENTS") {
            result.push_str(&args.join(" "));
            i += 10;
            continue;
        }

        result.push(chars[i]);
        i += 1;
    }

    result
}

/// Expand a pattern inside `${...}`.
///
/// Patterns:
/// - `N:-default` → positional arg N with default fallback
/// - `N:-` → positional arg N with empty default
/// - `N` → positional arg N (no default)
fn expand_braced_pattern(pattern: &str, args: &[String]) -> String {
    if let Some((n_str, default)) = pattern.split_once(":-") {
        if let Ok(n) = n_str.trim().parse::<usize>() {
            match args.get(n.wrapping_sub(1)) {
                Some(arg) => arg.clone(),
                None => default.to_string(),
            }
        } else {
            // Not a number — leave as-is
            format!("${{{pattern}}}")
        }
    } else if let Ok(n) = pattern.trim().parse::<usize>() {
        args.get(n.wrapping_sub(1)).cloned().unwrap_or_default()
    } else {
        format!("${{{pattern}}}")
    }
}

/// Parse a `/template:name arg1 arg2 ...` line.
///
/// Returns `Some((template_name, args))` if the line is a template reference.
pub(crate) fn parse_template_line(line: &str) -> Option<(String, Vec<String>)> {
    let line = line.trim();
    let name_part = line.strip_prefix("/template:")?;

    // Split on first whitespace
    let (name, rest) = match name_part.find(char::is_whitespace) {
        Some(pos) => (&name_part[..pos], name_part[pos + 1..].trim()),
        None => (name_part, ""),
    };

    let args: Vec<String> = if rest.is_empty() {
        vec![]
    } else {
        // Simple whitespace split (matches pi behavior)
        rest.split_whitespace().map(String::from).collect()
    };

    Some((name.to_string(), args))
}

/// Test if a line is a `/template:` reference.
pub(crate) fn is_template_line(line: &str) -> bool {
    line.trim().starts_with("/template:")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_substitute_positional() {
        let template = "Review file: $1\nWith option: $2";
        let args: Vec<String> = vec!["main.rs".into(), "--fix".into()];
        let result = substitute_template_args(template, &args);
        assert_eq!(result, "Review file: main.rs\nWith option: --fix");
    }

    #[test]
    fn test_substitute_missing_args() {
        let template = "File: $1, Dir: $2";
        let args: Vec<String> = vec!["main.rs".into()];
        let result = substitute_template_args(template, &args);
        assert_eq!(result, "File: main.rs, Dir: ");
    }

    #[test]
    fn test_substitute_all_args() {
        let template = "Args: $@";
        let args: Vec<String> = vec!["a".into(), "b".into(), "c".into()];
        let result = substitute_template_args(template, &args);
        assert_eq!(result, "Args: a b c");
    }

    #[test]
    fn test_substitute_arguments_keyword() {
        let template = "All: $ARGUMENTS";
        let args: Vec<String> = vec!["x".into(), "y".into()];
        let result = substitute_template_args(template, &args);
        assert_eq!(result, "All: x y");
    }

    #[test]
    fn test_substitute_default_value() {
        let template = "File: ${1:-src/main.rs}";
        let args: Vec<String> = vec![];
        let result = substitute_template_args(template, &args);
        assert_eq!(result, "File: src/main.rs");
    }

    #[test]
    fn test_substitute_default_value_with_existing_arg() {
        let template = "File: ${1:-src/main.rs}";
        let args: Vec<String> = vec!["lib.rs".into()];
        let result = substitute_template_args(template, &args);
        assert_eq!(result, "File: lib.rs");
    }

    #[test]
    fn test_substitute_multiple_defaults() {
        let template = "Review ${1:-*.rs} in ${2:-src/}";
        let args: Vec<String> = vec![];
        let result = substitute_template_args(template, &args);
        assert_eq!(result, "Review *.rs in src/");
    }

    #[test]
    fn test_template_expand() {
        let tmpl = PromptTemplate::new("review", "Review: $1\n---\n");
        let result = tmpl.expand(&["test.rs".into()]);
        assert_eq!(result, "Review: test.rs\n---\n");
    }

    #[test]
    fn test_template_expand_adds_trailing_newline() {
        let tmpl = PromptTemplate::new("review", "Review: $1");
        let result = tmpl.expand(&["test.rs".into()]);
        assert_eq!(result, "Review: test.rs\n");
    }

    #[test]
    fn test_parse_template_line_basic() {
        let (name, args) = parse_template_line("/template:review main.rs").unwrap();
        assert_eq!(name, "review");
        assert_eq!(args, vec!["main.rs"]);
    }

    #[test]
    fn test_parse_template_line_no_args() {
        let (name, args) = parse_template_line("/template:hello").unwrap();
        assert_eq!(name, "hello");
        assert!(args.is_empty());
    }

    #[test]
    fn test_parse_template_line_multiple_args() {
        let (name, args) = parse_template_line("/template:review src/main.rs --check").unwrap();
        assert_eq!(name, "review");
        assert_eq!(args, vec!["src/main.rs", "--check"]);
    }

    #[test]
    fn test_parse_template_line_not_a_template() {
        assert!(parse_template_line("/model gpt-4o").is_none());
    }

    #[test]
    fn test_is_template_line() {
        assert!(is_template_line("/template:review main.rs"));
        assert!(!is_template_line("/model gpt-4o"));
        assert!(!is_template_line("plain text"));
    }

    #[test]
    fn test_expand_integration() {
        let tmpl = PromptTemplate {
            name: "review".into(),
            body: "Review files matching: ${1:-*.rs}\nTarget dir: ${2:-src/}\nArgs: $@".into(),
            description: Some("Code review".into()),
            argument_hint: Some("<pattern> <dir>".into()),
            source_path: None,
        };
        let result = tmpl.expand(&["*.py".into(), "tests/".into(), "--verbose".into()]);
        assert!(result.contains("*.py"));
        assert!(result.contains("tests/"));
        assert!(result.contains("*.py tests/ --verbose"));
    }
}
