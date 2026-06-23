//! File operation tracking — extract read/write/edit ops from tool calls.

use crate::infra::session::types::CompactionEntry;

/// Tracked file operations from tool calls.
#[derive(Debug, Clone, Default)]
pub struct FileOps {
    pub read: Vec<String>,
    pub written: Vec<String>,
    pub edited: Vec<String>,
}

impl FileOps {
    fn read_set(&self) -> std::collections::BTreeSet<&str> {
        self.read.iter().map(|s| s.as_str()).collect()
    }
    fn written_set(&self) -> std::collections::BTreeSet<&str> {
        self.written.iter().map(|s| s.as_str()).collect()
    }
    fn edited_set(&self) -> std::collections::BTreeSet<&str> {
        self.edited.iter().map(|s| s.as_str()).collect()
    }
}

/// Extract file operations from `AgentMessage` messages.
pub fn extract_file_ops_from_messages(
    messages: &[crate::core::message::AgentMessage],
    prev_compaction: Option<&CompactionEntry>,
) -> FileOps {
    let mut ops = FileOps::default();

    if let Some(comp) = prev_compaction
        && let Some(ref details) = comp.details
    {
        if let Some(read_files) = details.get("readFiles").and_then(|v| v.as_array()) {
            for f in read_files {
                if let Some(s) = f.as_str() {
                    ops.read.push(s.to_string());
                }
            }
        }
        if let Some(modified) = details.get("modifiedFiles").and_then(|v| v.as_array()) {
            for f in modified {
                if let Some(s) = f.as_str() {
                    ops.edited.push(s.to_string());
                }
            }
        }
    }

    for msg in messages {
        for part in msg.content() {
            if let crate::core::message::AgentPart::ToolCall {
                name, arguments, ..
            } = part
            {
                let path = arguments.get("path").and_then(|v| v.as_str());
                let path = path.or_else(|| arguments.get("file_path").and_then(|v| v.as_str()));
                if let Some(p) = path {
                    match name.as_str() {
                        "read" => ops.read.push(p.to_string()),
                        "write" => ops.written.push(p.to_string()),
                        "edit" => ops.edited.push(p.to_string()),
                        _ => {}
                    }
                }
            }
        }
    }

    ops.read.sort();
    ops.read.dedup();
    ops.written.sort();
    ops.written.dedup();
    ops.edited.sort();
    ops.edited.dedup();

    ops
}

/// Compute final file lists: read_only = read \ (edited ∪ written), modified = edited ∪ written.
pub fn compute_file_lists(ops: &FileOps) -> (Vec<String>, Vec<String>) {
    let modified_set: std::collections::BTreeSet<&str> = ops
        .edited_set()
        .union(&ops.written_set())
        .copied()
        .collect();
    let read_only: Vec<String> = ops
        .read_set()
        .difference(&modified_set)
        .map(|s| s.to_string())
        .collect();
    let modified: Vec<String> = modified_set.iter().map(|s| s.to_string()).collect();
    (read_only, modified)
}

/// Format file operations as XML tags for the summary suffix.
pub fn format_file_ops_xml(read_files: &[String], modified_files: &[String]) -> String {
    let mut parts = Vec::new();
    if !read_files.is_empty() {
        parts.push(format!(
            "<read-files>\n{}\n</read-files>",
            read_files.join("\n")
        ));
    }
    if !modified_files.is_empty() {
        parts.push(format!(
            "<modified-files>\n{}\n</modified-files>",
            modified_files.join("\n")
        ));
    }
    if parts.is_empty() {
        String::new()
    } else {
        format!("\n\n{}", parts.join("\n\n"))
    }
}
