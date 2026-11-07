use std::time::Duration;
use std::time::Instant;

use xylitol_plugin_sdk::mcp::McpResourceContent;

pub const RESOURCE_CACHE_MAX_ENTRIES: usize = 64;
pub const RESOURCE_CACHE_MAX_ITEM_BYTES: usize = 256 * 1024;
pub const RESOURCE_CACHE_TTL: Duration = Duration::from_secs(5 * 60);

pub(crate) fn truncate_at_char_boundary(text: &str, limit: usize) -> String {
    let mut end = limit.min(text.len());
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }
    text[..end].to_string()
}

pub(crate) fn parse_resource_content(
    server: &str,
    uri: &str,
    value: serde_json::Value,
) -> McpResourceContent {
    let contents = value
        .get("contents")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    for item in contents {
        if let Some(text) = item.get("text").and_then(serde_json::Value::as_str) {
            let mime = item
                .get("mimeType")
                .and_then(serde_json::Value::as_str)
                .map(str::to_string);
            let (text, truncated) = if text.len() > RESOURCE_CACHE_MAX_ITEM_BYTES {
                (
                    truncate_at_char_boundary(text, RESOURCE_CACHE_MAX_ITEM_BYTES),
                    true,
                )
            } else {
                (text.to_string(), false)
            };
            return McpResourceContent {
                server: server.to_string(),
                uri: uri.to_string(),
                mime,
                text,
                truncated,
                binary: false,
            };
        }
        if item.get("blob").is_some() {
            return McpResourceContent {
                server: server.to_string(),
                uri: uri.to_string(),
                mime: item
                    .get("mimeType")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_string),
                text: String::new(),
                truncated: false,
                binary: true,
            };
        }
    }
    McpResourceContent {
        server: server.to_string(),
        uri: uri.to_string(),
        mime: None,
        text: value.to_string(),
        truncated: false,
        binary: false,
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ResourceCacheEntry {
    pub content: McpResourceContent,
    pub fetched_at: Instant,
}

#[derive(Debug)]
pub(crate) struct ResourceCache {
    entries: Vec<((String, String), ResourceCacheEntry)>,
    max_entries: usize,
    ttl: Duration,
}

impl Default for ResourceCache {
    fn default() -> Self {
        Self::new()
    }
}

impl ResourceCache {
    pub(crate) fn new() -> Self {
        Self::with_limits(RESOURCE_CACHE_MAX_ENTRIES, RESOURCE_CACHE_TTL)
    }

    pub(crate) fn with_limits(max_entries: usize, ttl: Duration) -> Self {
        Self {
            entries: Vec::new(),
            max_entries: max_entries.max(1),
            ttl,
        }
    }

    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.entries.len()
    }

    #[cfg(test)]
    pub(crate) fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub(crate) fn get(
        &mut self,
        server: &str,
        uri: &str,
        now: Instant,
    ) -> Option<McpResourceContent> {
        let index = self.entries.iter().position(|((key_server, key_uri), _)| {
            key_server.as_str() == server && key_uri.as_str() == uri
        })?;
        let (key, entry) = self.entries.remove(index);
        if now.duration_since(entry.fetched_at) >= self.ttl {
            return None;
        }
        let content = entry.content.clone();
        self.entries.push((key, entry));
        Some(content)
    }

    pub(crate) fn put(&mut self, content: McpResourceContent, now: Instant) {
        let mut content = content;
        if content.text.len() > RESOURCE_CACHE_MAX_ITEM_BYTES {
            content.text = truncate_at_char_boundary(&content.text, RESOURCE_CACHE_MAX_ITEM_BYTES);
            content.truncated = true;
        }
        let key = (content.server.clone(), content.uri.clone());
        self.entries
            .retain(|((key_server, key_uri), _)| *key_server != key.0 || *key_uri != key.1);
        self.entries.push((
            key,
            ResourceCacheEntry {
                content,
                fetched_at: now,
            },
        ));
        while self.entries.len() > self.max_entries {
            self.entries.remove(0);
        }
    }

    pub(crate) fn invalidate(&mut self, server: &str, uri: &str) -> bool {
        let before = self.entries.len();
        self.entries
            .retain(|((key_server, key_uri), _)| key_server.as_str() != server || key_uri != uri);
        before != self.entries.len()
    }

    pub(crate) fn invalidate_server(&mut self, server: &str) -> usize {
        let before = self.entries.len();
        self.entries
            .retain(|((key_server, _), _)| key_server.as_str() != server);
        before - self.entries.len()
    }

    pub(crate) fn clear(&mut self) {
        self.entries.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(server: &str, uri: &str, text: &str) -> McpResourceContent {
        McpResourceContent {
            server: server.into(),
            uri: uri.into(),
            mime: Some("text/plain".into()),
            text: text.into(),
            truncated: false,
            binary: false,
        }
    }

    #[test]
    fn get_returns_fresh_value_with_mime() {
        let mut cache = ResourceCache::new();
        let now = Instant::now();
        cache.put(entry("fs", "memo://a", "body"), now);
        let hit = cache
            .get("fs", "memo://a", now + Duration::from_secs(60))
            .expect("fresh hit");
        assert_eq!(hit.text, "body");
        assert_eq!(hit.mime.as_deref(), Some("text/plain"));
        assert!(!hit.truncated);
    }

    #[test]
    fn ttl_expires_stale_entries() {
        let mut cache = ResourceCache::with_limits(8, Duration::from_millis(50));
        let now = Instant::now();
        cache.put(entry("fs", "memo://a", "body"), now);
        assert!(
            cache
                .get("fs", "memo://a", now + Duration::from_millis(49))
                .is_some()
        );
        assert!(
            cache
                .get("fs", "memo://a", now + Duration::from_millis(51))
                .is_none()
        );
        assert!(cache.is_empty(), "stale entry MUST be dropped on read");
    }

    #[test]
    fn lru_evicts_oldest_beyond_max_entries() {
        let mut cache = ResourceCache::with_limits(2, RESOURCE_CACHE_TTL);
        let now = Instant::now();
        cache.put(entry("fs", "memo://a", "a"), now);
        cache.put(entry("fs", "memo://b", "b"), now);
        cache.put(entry("fs", "memo://c", "c"), now);
        assert_eq!(cache.len(), 2);
        assert!(cache.get("fs", "memo://a", now).is_none());
        assert!(cache.get("fs", "memo://b", now).is_some());
        assert!(cache.get("fs", "memo://c", now).is_some());
    }

    #[test]
    fn lru_touch_updates_recency_order() {
        let mut cache = ResourceCache::with_limits(2, RESOURCE_CACHE_TTL);
        let now = Instant::now();
        cache.put(entry("fs", "memo://a", "a"), now);
        cache.put(entry("fs", "memo://b", "b"), now);
        assert!(cache.get("fs", "memo://a", now).is_some());
        cache.put(entry("fs", "memo://c", "c"), now);
        assert!(cache.get("fs", "memo://a", now).is_some());
        assert!(cache.get("fs", "memo://b", now).is_none());
    }

    #[test]
    fn oversized_ascii_text_is_truncated_with_flag() {
        let mut cache = ResourceCache::new();
        let text = "x".repeat(RESOURCE_CACHE_MAX_ITEM_BYTES + 10);
        cache.put(entry("fs", "memo://big", &text), Instant::now());
        let hit = cache.get("fs", "memo://big", Instant::now()).expect("hit");
        assert_eq!(hit.text.len(), RESOURCE_CACHE_MAX_ITEM_BYTES);
        assert!(hit.truncated);
    }

    #[test]
    fn oversized_multibyte_text_truncates_on_char_boundary() {
        let mut cache = ResourceCache::new();
        let text = "\u{e9}".repeat(RESOURCE_CACHE_MAX_ITEM_BYTES / 2 + 5);
        cache.put(entry("fs", "memo://bin", &text), Instant::now());
        let hit = cache.get("fs", "memo://bin", Instant::now()).expect("hit");
        assert!(hit.text.len() <= RESOURCE_CACHE_MAX_ITEM_BYTES);
        assert!(hit.text.is_char_boundary(hit.text.len()));
        assert!(hit.truncated);
    }

    #[test]
    fn invalidate_removes_single_uri() {
        let mut cache = ResourceCache::new();
        let now = Instant::now();
        cache.put(entry("fs", "memo://a", "a"), now);
        cache.put(entry("fs", "memo://b", "b"), now);
        assert!(cache.invalidate("fs", "memo://a"));
        assert!(cache.get("fs", "memo://a", now).is_none());
        assert!(cache.get("fs", "memo://b", now).is_some());
        assert!(!cache.invalidate("fs", "memo://a"));
    }

    #[test]
    fn invalidate_server_scopes_to_one_server() {
        let mut cache = ResourceCache::new();
        let now = Instant::now();
        cache.put(entry("fs", "memo://a", "a"), now);
        cache.put(entry("git", "memo://a", "a2"), now);
        cache.put(entry("git", "memo://b", "b"), now);
        assert_eq!(cache.invalidate_server("git"), 2);
        assert!(cache.get("fs", "memo://a", now).is_some());
        assert!(cache.get("git", "memo://a", now).is_none());
        assert!(cache.get("git", "memo://b", now).is_none());
    }

    #[test]
    fn put_replaces_existing_key_without_growing() {
        let mut cache = ResourceCache::new();
        let now = Instant::now();
        cache.put(entry("fs", "memo://a", "old"), now);
        cache.put(entry("fs", "memo://a", "new"), now);
        assert_eq!(cache.len(), 1);
        let hit = cache.get("fs", "memo://a", now).expect("hit");
        assert_eq!(hit.text, "new");
    }

    #[test]
    fn parse_resource_content_extracts_text_and_mime() {
        let value = serde_json::json!({
            "contents": [
                {"uri": "memo://a", "mimeType": "text/markdown", "text": "# hi"}
            ]
        });
        let parsed = parse_resource_content("fs", "memo://a", value);
        assert_eq!(parsed.text, "# hi");
        assert_eq!(parsed.mime.as_deref(), Some("text/markdown"));
        assert!(!parsed.binary);
    }

    #[test]
    fn parse_resource_content_marks_blob_binary() {
        let value = serde_json::json!({
            "contents": [{"uri": "memo://img", "mimeType": "image/png", "blob": "aGk="}]
        });
        let parsed = parse_resource_content("fs", "memo://img", value);
        assert!(parsed.binary);
        assert_eq!(parsed.mime.as_deref(), Some("image/png"));
        assert!(parsed.text.is_empty());
    }

    #[test]
    fn parse_resource_content_truncates_oversized_text() {
        let text = "y".repeat(RESOURCE_CACHE_MAX_ITEM_BYTES + 1);
        let value = serde_json::json!({
            "contents": [{"uri": "memo://big", "text": text}]
        });
        let parsed = parse_resource_content("fs", "memo://big", value);
        assert!(parsed.truncated);
        assert_eq!(parsed.text.len(), RESOURCE_CACHE_MAX_ITEM_BYTES);
    }
}
