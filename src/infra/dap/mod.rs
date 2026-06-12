#![allow(dead_code)]
/// DAP integration layer — placeholder (feature = "infra-dap").
/// Expanded in c85.
pub(crate) struct Dap;

impl Default for Dap {
    fn default() -> Self {
        Self::new()
    }
}

impl Dap {
    pub(crate) fn new() -> Self {
        Self
    }
}
