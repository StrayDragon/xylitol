//! Wire form of session-tree kind (envelope payload, not Host vocabulary).

use serde::{Deserialize, Serialize};

/// Kind of session tree requested over the wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionTreeKind {
    MessageHistory,
    /// Reserved; callers MUST receive an explicit error until implemented.
    FileBrowser,
}
