//! Four-quadrant RPC envelope (fory-v3). Does not know Host vocabulary or `XyEvent`.

pub mod codec;
pub mod command;
pub mod envelope;
pub mod event;
pub mod method;
pub mod registry;
pub mod session_tree;
pub mod todo;
pub mod v3;

pub use command::Command;
pub use envelope::{
    ApprovalRequestedPayload, HostDescribeValue, PROTOCOL_VERSION, QuestionRequestedPayload,
    RpcError, RpcMessage, RpcResult, SessionEventPayload, SessionResourcesPayload,
    SessionResyncRequiredPayload, SessionSubscribedPayload, WIRE_FORMAT_FORY_V3,
    WIRE_FORMAT_JSONRPC,
};
pub use event::Event;
pub use method::{DOWNLINK_METHODS, is_downlink_method, is_unary_method};
pub use registry::{Auth, Idem, MethodEntry, REGISTRY, Resp, lookup, names, parse_command};
pub use session_tree::SessionTreeKind;
pub use todo::{TodoItem, TodoList, TodoStatus};
