//! Product method vocabulary. Unary names live on [`super::registry`] (c2530);
//! this module keeps the downlink closure and re-exports the unary gate.

use super::registry;

/// Downlink `ServerRequest.method` values (not unary).
pub const DOWNLINK_METHODS: &[&str] = &[
    "session/event",
    "session/subscribed",
    "session/resync_required",
    "session/resources",
    "approval/requested",
    "question/requested",
    "host/hello",
];

pub fn is_unary_method(name: &str) -> bool {
    registry::lookup(name).is_some()
}

pub fn is_downlink_method(name: &str) -> bool {
    DOWNLINK_METHODS.contains(&name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quit_is_not_unary() {
        assert!(!is_unary_method("quit"));
        assert!(!is_unary_method("approve_tool"));
        assert!(is_unary_method("prompt"));
        assert!(is_unary_method("host.describe"));
        assert!(is_unary_method("arm_tool_freeze"));
        assert!(is_downlink_method("session/event"));
        assert!(is_downlink_method("session/resources"));
    }
}
