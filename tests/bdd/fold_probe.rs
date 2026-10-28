use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

static HOST_CALLS: AtomicUsize = AtomicUsize::new(0);
static STORE_EXPOSED: AtomicBool = AtomicBool::new(false);

pub fn assert_no_host_roundtrip() {
    HOST_CALLS.store(0, Ordering::SeqCst);
}

pub fn host_calls() -> usize {
    HOST_CALLS.load(Ordering::SeqCst)
}

pub fn record_session_store_exposed(exposed: bool) {
    STORE_EXPOSED.store(exposed, Ordering::SeqCst);
}

pub fn session_store_exposed() -> bool {
    STORE_EXPOSED.load(Ordering::SeqCst)
}
