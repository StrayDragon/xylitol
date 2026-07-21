//! Source-info vocabulary — relocated to `domain::source_info`.
//!
//! This file remains as a thin re-export so existing `crate::infra::source_info`
//! references (infra-internal + tests) keep resolving while imports are migrated
//! in phases. New code should import from `crate::protocol::source_info`.

pub use crate::protocol::source_info::{
    SourceInfo, SourceOrigin, SourceScope, create_source_info, create_synthetic_source_info,
};
