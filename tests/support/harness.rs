//! Test harness — stub for c05-rebuild-core.
//! Full implementation will be restored in a future change.
//!
//! The previous InMemorySession/XySession types were removed as part of
//! the SessionManager migration.

#[allow(dead_code)]
pub(crate) struct HarnessBuilder {
    _placeholder: bool,
}

#[allow(dead_code)]
pub(crate) struct TestHarness {
    _placeholder: bool,
}

impl TestHarness {
    pub(crate) fn builder() -> HarnessBuilder {
        HarnessBuilder { _placeholder: true }
    }
}
