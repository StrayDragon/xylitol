//! Shared data structures for the diff review system.
//!
//! These types are used by the CLI (ratatui) review backend.

/// Severity level of a review comment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CommentSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

impl CommentSeverity {
    /// Parse from a string.
    pub fn parse(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "warning" | "warn" => CommentSeverity::Warning,
            "error" | "err" => CommentSeverity::Error,
            "critical" | "crit" => CommentSeverity::Critical,
            _ => CommentSeverity::Info,
        }
    }

    /// Return a short label.
    pub fn label(&self) -> &'static str {
        match self {
            CommentSeverity::Info => "info",
            CommentSeverity::Warning => "warning",
            CommentSeverity::Error => "error",
            CommentSeverity::Critical => "critical",
        }
    }
}

/// A single line-level review comment.
#[derive(Debug, Clone)]
pub struct ReviewComment {
    /// File path the comment refers to.
    pub file: String,
    /// Start line number (1-based).
    pub line_start: u32,
    /// End line number (1-based, inclusive).
    pub line_end: u32,
    /// Comment body text.
    pub content: String,
    /// Severity of the comment.
    pub severity: CommentSeverity,
}

/// The final verdict after reviewing.
#[derive(Debug, Clone)]
pub enum ReviewVerdict {
    /// Accept all changes (comments are still captured).
    AcceptAll(Vec<ReviewComment>),
    /// Reject with attached comments.
    RejectWithComments(Vec<ReviewComment>),
}

/// A single line in a diff hunk.
#[derive(Debug, Clone)]
pub struct DiffLine {
    /// The kind of change.
    pub kind: DiffLineKind,
    /// The line content (without prefix).
    pub content: String,
    /// Original file line number (0 = N/A for insertions).
    pub old_line_no: u32,
    /// New file line number (0 = N/A for deletions).
    pub new_line_no: u32,
}

/// Whether a diff line is an addition, deletion, or context.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffLineKind {
    Add,
    Delete,
    Context,
}

impl DiffLineKind {
    /// Return the diff prefix character.
    pub fn prefix(&self) -> &'static str {
        match self {
            DiffLineKind::Add => "+",
            DiffLineKind::Delete => "-",
            DiffLineKind::Context => " ",
        }
    }
}

/// A contiguous hunk of changes in a diff.
#[derive(Debug, Clone)]
pub struct DiffHunk {
    /// File path this hunk belongs to.
    pub file: String,
    /// Start line in the old file (1-based).
    pub old_start: u32,
    /// Number of lines in the old file.
    pub old_count: u32,
    /// Start line in the new file (1-based).
    pub new_start: u32,
    /// Number of lines in the new file.
    pub new_count: u32,
    /// The individual lines in this hunk.
    pub lines: Vec<DiffLine>,
}

/// A complete diff review session.
#[derive(Debug, Clone)]
pub struct ReviewSession {
    /// Unique identifier for this review session.
    pub review_id: String,
    /// All hunks in this session.
    pub hunks: Vec<DiffHunk>,
    /// Comments collected during review.
    pub comments: Vec<ReviewComment>,
    /// Final verdict, set when the user completes the review.
    pub verdict: Option<ReviewVerdict>,
}

impl ReviewSession {
    /// Create a new review session.
    pub fn new(review_id: String, hunks: Vec<DiffHunk>) -> Self {
        Self {
            review_id,
            hunks,
            comments: Vec::new(),
            verdict: None,
        }
    }

    /// Add a comment to the session.
    pub fn add_comment(&mut self, comment: ReviewComment) {
        self.comments.push(comment);
    }

    /// Return the total number of changed lines across all hunks.
    pub fn total_changes(&self) -> usize {
        self.hunks.iter().flat_map(|h| &h.lines).count()
    }

    /// Return the number of files touched.
    pub fn file_count(&self) -> usize {
        let mut files: Vec<&str> = self.hunks.iter().map(|h| h.file.as_str()).collect();
        files.sort();
        files.dedup();
        files.len()
    }
}
