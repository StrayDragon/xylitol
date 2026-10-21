pub use xylitol_plugin_sdk::session::{
    BranchSummaryEntry, CompactionEntry, CustomEntry, CustomMessageEntry, EntryBase, LabelEntry,
    MessageEntry, ModelChangeEntry, SESSION_VERSION, SessionContext, SessionEntry, SessionHeader,
    SessionInfoEntry, ThinkingLevelChangeEntry,
};

#[derive(Debug, Clone)]
pub enum SessionBackend {
    Persisted,
    InMemory,
}
