//! Wire protocol — client ↔ core `Command` / `Event` vocabulary.
//!
//! Envelope types live in `xylitol-protocol`. `XyEvent` mapping stays here
//! until the serve crate takes it (W2.4).

pub use xylitol_protocol::SessionTreeKind as WireSessionTreeKind;
pub use xylitol_protocol::{
    ApprovalRequestedPayload, Auth, Command, DOWNLINK_METHODS, Event, HostDescribeValue, Idem,
    MethodEntry, PROTOCOL_VERSION, QuestionRequestedPayload, REGISTRY, Resp, RpcError, RpcMessage,
    RpcResult, SessionEventPayload, SessionResourcesPayload, SessionResyncRequiredPayload,
    SessionSubscribedPayload, WIRE_FORMAT_FORY_V3, WIRE_FORMAT_JSONRPC, codec, command, envelope,
    is_downlink_method, is_unary_method, lookup, method, names, parse_command, registry,
};
pub use xylitol_protocol::{
    TodoItem as WireTodoItem, TodoList as WireTodoList, TodoStatus as WireTodoStatus,
};

pub mod event;
pub mod v3;

pub use event::{FromWireEvent, ToWireEvent, WireEventConvertError};

/// Envelope session-tree kind → Host vocabulary kind.
pub fn sdk_session_tree_kind(
    kind: xylitol_protocol::SessionTreeKind,
) -> crate::protocol::session::SessionTreeKind {
    match kind {
        xylitol_protocol::SessionTreeKind::MessageHistory => {
            crate::protocol::session::SessionTreeKind::MessageHistory
        }
        xylitol_protocol::SessionTreeKind::FileBrowser => {
            crate::protocol::session::SessionTreeKind::FileBrowser
        }
    }
}

/// Host vocabulary kind → envelope session-tree kind.
pub fn wire_session_tree_kind(
    kind: crate::protocol::session::SessionTreeKind,
) -> xylitol_protocol::SessionTreeKind {
    match kind {
        crate::protocol::session::SessionTreeKind::MessageHistory => {
            xylitol_protocol::SessionTreeKind::MessageHistory
        }
        crate::protocol::session::SessionTreeKind::FileBrowser => {
            xylitol_protocol::SessionTreeKind::FileBrowser
        }
    }
}
