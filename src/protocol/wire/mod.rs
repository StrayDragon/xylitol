//! Wire protocol — client ↔ core `Command` / `Event` vocabulary.
//!
//! Transport-agnostic: the same types are spoken over stdio RPC, WebSocket, and REST.
//! MUST NOT depend on [`crate::protocol::ports`].

pub mod codec;
pub mod command;
pub mod envelope;
pub mod event;
pub mod method;
pub mod registry;

pub use command::Command;
pub use envelope::{
    ApprovalRequestedPayload, HostDescribeValue, PROTOCOL_VERSION, QuestionRequestedPayload,
    RpcError, RpcMessage, RpcResult, SessionEventPayload, SessionResourcesPayload,
    SessionResyncRequiredPayload, SessionSubscribedPayload,
};
pub use event::Event;
pub use method::{DOWNLINK_METHODS, is_downlink_method, is_unary_method};
pub use registry::{Auth, Idem, MethodEntry, REGISTRY, Resp, lookup, names, parse_command};
