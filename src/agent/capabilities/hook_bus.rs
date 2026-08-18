//! Script-hook bus helpers for capability lifecycle (cancel / observe).

use std::sync::Arc;

use crate::protocol::ports::XyHookBus;

/// Hook dispatched `Blocked { reason }` (product copy; not `Xy*`).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{0}")]
pub(crate) struct HookBlockedError(pub String);

pub(crate) async fn cancel_hook(
    bus: &Arc<dyn XyHookBus>,
    event_type: &str,
    phase: &str,
    context: serde_json::Value,
) -> Result<(), HookBlockedError> {
    match bus.dispatch(event_type, phase, context).await {
        crate::protocol::ports::XyHookOutcome::Blocked { reason } => Err(HookBlockedError(reason)),
        _ => Ok(()),
    }
}

pub(crate) async fn observe_hook(
    bus: &Arc<dyn XyHookBus>,
    event_type: &str,
    phase: &str,
    context: serde_json::Value,
) {
    if let crate::protocol::ports::XyHookOutcome::Blocked { reason } =
        bus.dispatch(event_type, phase, context).await
    {
        log::warn!(
            "Script hook blocked observe-only lifecycle event (fail-open) event={} phase={} reason={}",
            event_type,
            phase,
            reason
        );
    }
}

/// Process-wide runtime for sync observe hooks (model/thinking select).
///
/// Replaces per-call `thread::spawn` + ad-hoc runtime (c996 hot path).
fn hook_observe_runtime() -> &'static tokio::runtime::Runtime {
    use std::sync::OnceLock;
    static RT: OnceLock<tokio::runtime::Runtime> = OnceLock::new();
    RT.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .thread_name("xy-hook-obs")
            .build()
            .expect("observe_hook_sync runtime")
    })
}

/// Sync observe for XyDriver/agent APIs that are not async (c996).
///
/// Always schedules on the dedicated shared runtime and waits via channel so
/// callers on `current_thread` test runtimes never hit `block_in_place` /
/// nested `block_on` panics.
pub(crate) fn observe_hook_sync(
    bus: &Arc<dyn XyHookBus>,
    event_type: &str,
    phase: &str,
    context: serde_json::Value,
) {
    let bus = bus.clone();
    let event_type = event_type.to_string();
    let phase = phase.to_string();
    let (tx, rx) = std::sync::mpsc::channel();
    hook_observe_runtime().spawn(async move {
        observe_hook(&bus, &event_type, &phase, context).await;
        let _ = tx.send(());
    });
    if rx.recv().is_err() {
        log::warn!("observe_hook_sync worker disconnected");
    }
}
