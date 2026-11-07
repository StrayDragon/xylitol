use serde_json::Value;
use xylitol_plugin_sdk::mcp::{McpCatalogEntry, McpResourceRef};

const TOKEN_HEAD: &str = "@mcp:\"";

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AttachmentTray {
    refs: Vec<McpResourceRef>,
}

impl AttachmentTray {
    pub fn add(&mut self, server: &str, uri: &str) -> bool {
        if self.contains(server, uri) {
            return false;
        }
        self.refs.push(McpResourceRef {
            server: server.to_string(),
            uri: uri.to_string(),
        });
        true
    }

    pub fn contains(&self, server: &str, uri: &str) -> bool {
        self.refs.iter().any(|r| r.server == server && r.uri == uri)
    }

    pub fn remove_last(&mut self) -> bool {
        self.refs.pop().is_some()
    }

    pub fn remove_at(&mut self, index: usize) -> bool {
        if index < self.refs.len() {
            self.refs.remove(index);
            true
        } else {
            false
        }
    }

    pub fn clear(&mut self) -> bool {
        let had = !self.refs.is_empty();
        self.refs.clear();
        had
    }

    pub fn is_empty(&self) -> bool {
        self.refs.is_empty()
    }

    pub fn len(&self) -> usize {
        self.refs.len()
    }

    pub fn refs(&self) -> &[McpResourceRef] {
        &self.refs
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PendingMcpAttachments {
    pub tray: Vec<McpResourceRef>,
    pub tokens: Vec<McpResourceRef>,
}

impl PendingMcpAttachments {
    pub fn is_empty(&self) -> bool {
        self.tray.is_empty() && self.tokens.is_empty()
    }

    pub fn combined(&self) -> Vec<McpResourceRef> {
        let mut out = self.tray.clone();
        for r in &self.tokens {
            if !out.iter().any(|e| e.server == r.server && e.uri == r.uri) {
                out.push(r.clone());
            }
        }
        out
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ParsedMcpTokens {
    pub refs: Vec<McpResourceRef>,
    pub cleaned: String,
}

pub fn parse_quoted_mcp_tokens(text: &str) -> ParsedMcpTokens {
    let mut refs: Vec<McpResourceRef> = Vec::new();
    let mut cleaned = String::with_capacity(text.len());
    let mut in_fence = false;
    for line in text.split_inclusive('\n') {
        if line.trim_start().starts_with("```") {
            in_fence = !in_fence;
            cleaned.push_str(line);
            continue;
        }
        if in_fence {
            cleaned.push_str(line);
            continue;
        }
        parse_line(line, &mut refs, &mut cleaned);
    }
    ParsedMcpTokens { refs, cleaned }
}

fn parse_line(line: &str, refs: &mut Vec<McpResourceRef>, cleaned: &mut String) {
    let chars: Vec<char> = line.chars().collect();
    let mut i = 0usize;
    let mut in_inline_code = false;
    while i < chars.len() {
        let ch = chars[i];
        if ch == '`' {
            in_inline_code = !in_inline_code;
            cleaned.push(ch);
            i += 1;
            continue;
        }
        if in_inline_code {
            cleaned.push(ch);
            i += 1;
            continue;
        }
        if ch == '\\' && token_head_at(&chars, i + 1) {
            let literal_end = literal_span_end(&chars, i + 1);
            for c in &chars[i..literal_end] {
                cleaned.push(*c);
            }
            i = literal_end;
            continue;
        }
        if token_head_at(&chars, i) {
            let Some((content, end)) = quoted_span(&chars, i) else {
                cleaned.push(ch);
                i += 1;
                continue;
            };
            match split_ref(&content) {
                Some(r) if !refs.contains(&r) => {
                    refs.push(r);
                    i = end;
                }
                Some(_) => {
                    i = end;
                }
                None => {
                    for c in &chars[i..end] {
                        cleaned.push(*c);
                    }
                    i = end;
                }
            }
            continue;
        }
        cleaned.push(ch);
        i += 1;
    }
}

fn token_head_at(chars: &[char], start: usize) -> bool {
    let head: Vec<char> = TOKEN_HEAD.chars().collect();
    if start + head.len() > chars.len() {
        return false;
    }
    chars[start..start + head.len()] == head[..]
}

fn quoted_span(chars: &[char], start: usize) -> Option<(String, usize)> {
    let head_len = TOKEN_HEAD.chars().count();
    let mut content = String::new();
    let mut i = start + head_len;
    while i < chars.len() {
        match chars[i] {
            '\\' => {
                let &next = chars.get(i + 1)?;
                if next == '"' || next == '\\' {
                    content.push(next);
                    i += 2;
                } else {
                    content.push('\\');
                    content.push(next);
                    i += 2;
                }
            }
            '"' => return Some((content, i + 1)),
            c => {
                content.push(c);
                i += 1;
            }
        }
    }
    None
}

fn literal_span_end(chars: &[char], head_start: usize) -> usize {
    let head_len = TOKEN_HEAD.chars().count();
    let mut i = head_start + head_len;
    while i < chars.len() {
        match chars[i] {
            '\\' => i += 2,
            '"' => return i + 1,
            _ => i += 1,
        }
    }
    chars.len()
}

fn split_ref(content: &str) -> Option<McpResourceRef> {
    let (server, uri) = content.split_once('/')?;
    if server.is_empty() || uri.is_empty() {
        return None;
    }
    if server
        .chars()
        .any(|c| !c.is_ascii_alphanumeric() && c != '_' && c != '-')
    {
        return None;
    }
    Some(McpResourceRef {
        server: server.to_string(),
        uri: uri.to_string(),
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpPromptTarget {
    pub server: String,
    pub name: String,
    pub fields: Vec<PromptArgField>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromptArgField {
    pub name: String,
    pub required: bool,
}

pub fn prompt_arg_fields(schema: Option<&Value>) -> Vec<PromptArgField> {
    let Some(Value::Object(map)) = schema else {
        return Vec::new();
    };
    let Some(Value::Object(properties)) = map.get("properties") else {
        return Vec::new();
    };
    let required: Vec<&str> = map
        .get("required")
        .and_then(Value::as_array)
        .map(|arr| arr.iter().filter_map(Value::as_str).collect::<Vec<&str>>())
        .unwrap_or_default();
    let mut fields: Vec<PromptArgField> = properties
        .iter()
        .filter(|(name, spec)| {
            let is_string = spec
                .get("type")
                .and_then(Value::as_str)
                .map(|t| t == "string")
                .unwrap_or(true);
            let has_enum = spec.get("enum").is_some();
            is_string && !has_enum && !name.is_empty()
        })
        .map(|(name, _)| PromptArgField {
            name: name.clone(),
            required: required.contains(&name.as_str()),
        })
        .collect();
    fields.sort_by(|a, b| {
        b.required
            .cmp(&a.required)
            .then_with(|| a.name.cmp(&b.name))
    });
    fields
}

pub fn prompt_result_text(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        Value::Array(items) => items
            .iter()
            .map(message_text)
            .collect::<Vec<_>>()
            .join("\n\n"),
        Value::Object(_) => match value.get("messages").and_then(Value::as_array) {
            Some(messages) => messages
                .iter()
                .map(message_text)
                .collect::<Vec<_>>()
                .join("\n\n"),
            None => value.to_string(),
        },
        _ => value.to_string(),
    }
}

fn message_text(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        Value::Object(_) => match &value.get("content") {
            Some(Value::String(text)) => text.clone(),
            Some(Value::Array(parts)) => parts
                .iter()
                .filter_map(|p| p.get("text").and_then(Value::as_str))
                .collect::<Vec<_>>()
                .join("\n"),
            Some(Value::Object(obj)) => obj
                .get("text")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            _ => value
                .get("text")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
        },
        _ => String::new(),
    }
}

pub fn catalog_find_resource<'a>(
    catalog: &'a [McpCatalogEntry],
    server: &str,
    uri: &str,
) -> Option<&'a McpCatalogEntry> {
    catalog
        .iter()
        .find(|e| e.server == server && e.uri.as_deref() == Some(uri))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn refs(pairs: &[(&str, &str)]) -> Vec<McpResourceRef> {
        pairs
            .iter()
            .map(|(s, u)| McpResourceRef {
                server: s.to_string(),
                uri: u.to_string(),
            })
            .collect()
    }

    #[test]
    fn tray_add_dedupes_and_removes() {
        let mut tray = AttachmentTray::default();
        assert!(tray.add("fs", "file:///a"));
        assert!(!tray.add("fs", "file:///a"));
        assert!(tray.add("fs", "file:///b"));
        assert_eq!(tray.len(), 2);
        assert!(tray.remove_at(0));
        assert_eq!(tray.refs().len(), 1);
        assert_eq!(tray.refs()[0].uri, "file:///b");
        assert!(tray.remove_last());
        assert!(tray.is_empty());
        assert!(!tray.remove_last());
    }

    #[test]
    fn quoted_token_extracts_and_removes() {
        let parsed = parse_quoted_mcp_tokens("check @mcp:\"fs/file:///a\" now");
        assert_eq!(parsed.refs, refs(&[("fs", "file:///a")]));
        assert_eq!(parsed.cleaned, "check  now");
    }

    #[test]
    fn quoted_token_escapes_decode() {
        let content = r#"\"x\"\\b"#;
        let raw = format!("@mcp:\"fs/file {content}\" tail");
        let parsed = parse_quoted_mcp_tokens(&raw);
        assert_eq!(parsed.refs, refs(&[("fs", r#"file "x"\b"#)]));
        assert_eq!(parsed.cleaned.trim(), "tail");
    }

    #[test]
    fn unquoted_mcp_stays_plain_text() {
        let parsed = parse_quoted_mcp_tokens("see @mcp:fs/file:///a and @mcp:");
        assert!(parsed.refs.is_empty());
        assert_eq!(parsed.cleaned, "see @mcp:fs/file:///a and @mcp:");
    }

    #[test]
    fn unclosed_quote_stays_plain_text() {
        let parsed = parse_quoted_mcp_tokens("x @mcp:\"fs/file");
        assert!(parsed.refs.is_empty());
        assert_eq!(parsed.cleaned, "x @mcp:\"fs/file");
    }

    #[test]
    fn fenced_and_inline_code_skip_parsing() {
        let fenced = "```\n@mcp:\"fs/a\"\n```";
        assert!(parse_quoted_mcp_tokens(fenced).refs.is_empty());
        let inline = "use `@mcp:\"fs/a\"` here";
        assert!(parse_quoted_mcp_tokens(inline).refs.is_empty());
    }

    #[test]
    fn escaped_at_token_is_literal() {
        let parsed = parse_quoted_mcp_tokens("\\@mcp:\"fs/a\" end");
        assert!(parsed.refs.is_empty());
        assert_eq!(parsed.cleaned, "\\@mcp:\"fs/a\" end");
    }

    #[test]
    fn repeated_token_dedupes() {
        let parsed = parse_quoted_mcp_tokens("@mcp:\"fs/a\" @mcp:\"fs/a\"");
        assert_eq!(parsed.refs.len(), 1);
        assert_eq!(parsed.cleaned.trim(), "");
    }

    #[test]
    fn token_without_slash_is_plain_text() {
        let parsed = parse_quoted_mcp_tokens("@mcp:\"noseparator\" tail");
        assert!(parsed.refs.is_empty());
        assert!(parsed.cleaned.contains("@mcp:\"noseparator\""));
    }

    #[test]
    fn pending_attachments_combine_dedupes() {
        let att = PendingMcpAttachments {
            tray: refs(&[("fs", "a"), ("db", "x")]),
            tokens: refs(&[("fs", "a"), ("fs", "b")]),
        };
        assert_eq!(att.combined().len(), 3);
        assert!(!att.is_empty());
        assert!(PendingMcpAttachments::default().is_empty());
    }

    #[test]
    fn arg_fields_read_schema() {
        let schema: Value = serde_json::json!({
            "type": "object",
            "properties": {
                "topic": { "type": "string" },
                "count": { "type": "number" },
                "note": { "type": "string" }
            },
            "required": ["topic"]
        });
        let fields = prompt_arg_fields(Some(&schema));
        assert_eq!(
            fields,
            vec![
                PromptArgField {
                    name: "topic".into(),
                    required: true
                },
                PromptArgField {
                    name: "note".into(),
                    required: false
                },
            ]
        );
        assert!(prompt_arg_fields(None).is_empty());
    }

    #[test]
    fn prompt_result_text_extracts_messages() {
        let value: Value = serde_json::json!({
            "messages": [
                { "role": "user", "content": { "type": "text", "text": "line one" } },
                { "role": "assistant", "content": "line two" }
            ]
        });
        assert_eq!(prompt_result_text(&value), "line one\n\nline two");
        assert_eq!(prompt_result_text(&Value::String("raw".into())), "raw");
    }
}
