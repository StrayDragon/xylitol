mod atom;
mod degrade;
mod live_tape;
mod segment;
mod settings;
mod state;
mod summary;

/// 故不再限 `#[cfg(test)]`。
pub mod scene;

#[cfg(test)]
pub(crate) use atom::{ToolActivityRole, is_path_placeholder, tool_activity_role};
pub use degrade::AutoTrigger;
#[cfg(test)]
pub use live_tape::LIVE_ASK_CLOSE_TEXT;
pub use live_tape::{live_ask_close_events, replay_live_window};
pub use segment::ActivityCluster;
pub use segment::{
    ActivitySegment, SegmentLevel, cluster_middle_indices, middle_entry_indices, partition_segments,
};
pub use settings::ActivityFoldSettings;
pub use state::{ActivityFoldState, SegmentClock};
pub use summary::{
    cluster_is_thought_only, cluster_omits_header, count_cluster, format_cluster_header,
    format_elapsed_secs, format_envelope_line, live_think_id_for_cluster, streaming_thought_counts,
    thought_header_body,
};
pub use xylitol_tui::utils::strip_ansi_codes as strip_ansi_live_window;

use time::OffsetDateTime;

use crate::bridge::UiEntry;
use xylitol_plugin_sdk::session::{SessionEntry, SessionTreeTravel};

#[cfg(test)]
pub(crate) use scene::SemanticDump;

pub fn parse_timestamp(raw: &str) -> Option<OffsetDateTime> {
    let raw = raw.trim();
    if raw.is_empty() {
        return None;
    }
    if let Ok(dt) = OffsetDateTime::parse(raw, &time::format_description::well_known::Rfc3339) {
        return Some(dt);
    }
    if let Ok(n) = raw.parse::<i64>() {
        let secs = if n > 10_000_000_000 { n / 1000 } else { n };
        return OffsetDateTime::from_unix_timestamp(secs).ok();
    }
    None
}

pub fn ingest_rebuild_clocks(
    state: &mut ActivityFoldState,
    ui_entries: &[UiEntry],
    session_entries: &[SessionEntry],
    travel: &SessionTreeTravel,
) {
    let segs = partition_segments(ui_entries);
    let path_ids = ancestry_path_ids(session_entries, travel.leaf_id.as_deref());
    let mut user_stamps: Vec<Option<OffsetDateTime>> = Vec::new();
    let mut asst_stamps: Vec<Option<OffsetDateTime>> = Vec::new();
    let mut cur_user: Option<OffsetDateTime> = None;
    let mut cur_asst: Option<OffsetDateTime> = None;
    let mut in_turn = false;

    for id in &path_ids {
        let Some(entry) = session_entries
            .iter()
            .find(|e| e.entry_id() == Some(id.as_str()))
        else {
            continue;
        };
        let Some(base) = entry.base() else {
            continue;
        };
        let ts = parse_timestamp(&base.timestamp.to_string());
        if let SessionEntry::Message(m) = entry {
            let role = xylitol_plugin_sdk::session::message_role(&m.message);
            match role {
                Some("user") => {
                    if in_turn {
                        user_stamps.push(cur_user);
                        asst_stamps.push(cur_asst);
                    }
                    cur_user = ts;
                    cur_asst = None;
                    in_turn = true;
                }
                Some("assistant") if ts.is_some() => {
                    cur_asst = ts;
                }
                _ => {}
            }
        }
    }
    if in_turn {
        user_stamps.push(cur_user);
        asst_stamps.push(cur_asst);
    }

    let mut user_ord = 0usize;
    for (idx, e) in ui_entries.iter().enumerate() {
        if !matches!(e, UiEntry::User { .. }) {
            continue;
        }
        if let Some(seg) = segs.iter().find(|s| s.user_idx == idx) {
            let start = user_stamps.get(user_ord).copied().flatten();
            let end = asst_stamps.get(user_ord).copied().flatten();
            state.set_clock(&seg.id, SegmentClock { start, end });
        }
        user_ord += 1;
    }
}

fn ancestry_path_ids(entries: &[SessionEntry], leaf_id: Option<&str>) -> Vec<String> {
    crate::bridge::session_tree::ancestry_path_ids(entries, leaf_id)
}
