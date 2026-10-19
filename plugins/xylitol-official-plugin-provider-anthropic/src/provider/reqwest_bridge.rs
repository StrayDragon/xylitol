//! reqwest ↔ portable [`crate::hooks::HeaderBag`] bridge.
//!
//! Hook helpers stay client-agnostic; this module is the **only** place that
//! knows reqwest header types for provider adapters. Swap or add bridges when
//! a vendor SDK replaces raw reqwest for a dialect.

use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use serde_json::{Map, Value};

use crate::hooks::HeaderBag;

/// Convert a portable header bag into a reqwest [`HeaderMap`].
///
/// Invalid names/values are skipped (same fail-soft as hook merge).
pub fn to_reqwest_headers(bag: &HeaderBag) -> HeaderMap {
    let mut headers = HeaderMap::new();
    for (key, val) in bag {
        let Some(text) = val.as_str() else {
            continue;
        };
        if let (Ok(name), Ok(header_val)) = (
            HeaderName::from_bytes(key.as_bytes()),
            HeaderValue::from_str(text),
        ) {
            headers.insert(name, header_val);
        }
    }
    headers
}

/// Convert reqwest response/request headers into a portable bag (lowercase keys).
pub fn from_reqwest_headers(headers: &HeaderMap) -> HeaderBag {
    let mut map = Map::new();
    for (name, value) in headers.iter() {
        let key = name.as_str().to_ascii_lowercase();
        let val = value.to_str().unwrap_or("").to_string();
        map.insert(key, Value::String(val));
    }
    map
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::header::CONTENT_TYPE;

    #[test]
    fn roundtrip_content_type() {
        let mut bag = HeaderBag::new();
        bag.insert(
            "content-type".into(),
            Value::String("application/json".into()),
        );
        let headers = to_reqwest_headers(&bag);
        assert_eq!(headers.get(CONTENT_TYPE).unwrap(), "application/json");
        let back = from_reqwest_headers(&headers);
        assert_eq!(back.get("content-type").unwrap(), "application/json");
    }
}
