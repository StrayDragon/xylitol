//! Stable `TypeKey` names for Host services.

use crate::ports::{XyCompaction, XyHookBus, XyModel, XyPermission, XySessionStore, XyTool};
use crate::rt::TypeKey;

pub fn session_store_key() -> TypeKey {
    TypeKey::of::<dyn XySessionStore>()
}

pub fn model_key() -> TypeKey {
    TypeKey::of::<dyn XyModel>()
}

pub fn compaction_key() -> TypeKey {
    TypeKey::of::<dyn XyCompaction>()
}

pub fn hook_bus_key() -> TypeKey {
    TypeKey::of::<dyn XyHookBus>()
}

pub fn permission_key() -> TypeKey {
    TypeKey::of::<dyn XyPermission>()
}

pub fn tool_vec_key() -> TypeKey {
    TypeKey::of::<Vec<std::sync::Arc<dyn XyTool>>>()
}

pub fn session_prepare_hub_key() -> TypeKey {
    TypeKey::of::<dyn crate::session_prepare::SessionPrepareHub>()
}
