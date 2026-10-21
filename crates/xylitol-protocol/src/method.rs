use super::registry;

pub const SESSION_EVENT: &str = "session/event";
pub const SESSION_SUBSCRIBED: &str = "session/subscribed";
pub const SESSION_RESYNC_REQUIRED: &str = "session/resync_required";
pub const SESSION_RESOURCES: &str = "session/resources";
pub const APPROVAL_REQUESTED: &str = "approval/requested";
pub const QUESTION_REQUESTED: &str = "question/requested";
pub const BASH_OUTPUT: &str = "bash/output";

pub const APPROVE_TOOL: &str = "approve_tool";
pub const ANSWER_QUESTION: &str = "answer_question";

pub const RESERVED_PLUGIN_PREFIX: &str = "plugin/";

pub const DOWNLINK_METHODS: &[&str] = &[
    SESSION_EVENT,
    SESSION_SUBSCRIBED,
    SESSION_RESYNC_REQUIRED,
    SESSION_RESOURCES,
    APPROVAL_REQUESTED,
    QUESTION_REQUESTED,
    BASH_OUTPUT,
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
        assert!(!is_unary_method(APPROVE_TOOL));
        assert!(is_unary_method("prompt"));
        assert!(is_unary_method("host.describe"));
        assert!(is_unary_method("arm_tool_freeze"));
    }

    #[test]
    fn downlink_names_include_bash_output() {
        assert!(is_downlink_method(SESSION_EVENT));
        assert!(is_downlink_method(SESSION_RESOURCES));
        assert!(is_downlink_method(BASH_OUTPUT));
    }
}
