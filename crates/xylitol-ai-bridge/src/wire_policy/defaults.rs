//! Unexposed wire-strategy defaults — **only** edit this file to change defaults.
//!
//! Named YAML `models.*.compat` selects a [`super::WirePolicy`] profile (c1940);
//! free-form `extra_policy` YAML remains forbidden — edit these constants or
//! profile constructors instead.

use super::Compat;

/// Default compatibility profile when YAML `compat` is omitted.
pub const COMPAT_DEFAULT: Compat = Compat::Generic;

/// When false, do not assume first-language prompt-cache usage fields.
pub const PROMPT_CACHE_USAGE: bool = true;

/// When false, do not send / rely on prompt_cache_key.
pub const PROMPT_CACHE_KEY: bool = false;

/// When false, do not enable previous_response_id chaining.
pub const PREVIOUS_RESPONSE_ID: bool = false;
