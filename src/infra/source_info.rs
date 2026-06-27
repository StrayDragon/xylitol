//! Source-info vocabulary — relocated to `core::source_info`.
//!
//! This file remains as a thin re-export so existing `crate::infra::source_info`
//! references (infra-internal + tests) keep resolving while imports are migrated
//! in phases. New code should import from `crate::core::source_info`.

pub use crate::core::source_info::{
    SourceInfo, SourceOrigin, SourceScope, create_source_info, create_synthetic_source_info,
};
