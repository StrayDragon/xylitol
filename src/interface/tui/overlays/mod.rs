//! Built-in overlay components (modals).

mod approval;
mod help;
mod history_search;
mod selector;

#[allow(unused_imports)]
pub(crate) use approval::ApprovalOverlay;
pub(crate) use help::HelpOverlay;
#[allow(unused_imports)]
pub(crate) use history_search::HistorySearchOverlay;
#[allow(unused_imports)]
pub(crate) use selector::{SelectorKind, SelectorOverlay};

#[cfg(feature = "ui-review")]
mod diff_preview;
#[cfg(feature = "ui-review")]
#[allow(unused_imports)]
pub(crate) use diff_preview::DiffPreviewOverlay;
