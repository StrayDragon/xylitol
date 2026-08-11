//! Source-info vocabulary re-export.
//!
//! Prefer `crate::protocol::source_info` in new code. This path remains for
//! existing infra-internal imports.

pub use crate::protocol::source_info::{
    SourceInfo, SourceOrigin, SourceScope, create_source_info, create_synthetic_source_info,
};
