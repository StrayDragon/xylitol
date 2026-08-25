//! Semantic scene dump for the product ActivityFold paint (c2200 scene slice).
//!
//! Scene = product path, not a test-only paint:
//! - entries are the same [`UiEntry`] shapes the product builds — via
//!   [`apply_xy_event`] (same seam as `live_tape.rs`) or, for sealed states,
//!   the same flushed/persisted entry fields that resume consumes;
//! - the frame comes from the product root: [`crate::app::tui::layout::UiRoot`]
//!   → `render_scrollback` → `root.render` (same call chain as
//!   `/debug activity-fold-live`);
//! - the dump annotates rows with the chord the product paint made:
//!   `L3 envelope` (`Worked for`) / `L2 cluster` (cluster head) /
//!   `L1 block` (foldable middle content, when kids are expanded).
//!
//! Chord truth comes from the product decision points:
//! `partition_segments` (segment.rs) · `count_cluster` / `format_cluster_body`
//! (summary.rs) · `paint_envelope_header_row` / `paint_cluster_header_row`
//! (widgets/scrollback.rs).

use super::summary::{count_cluster, format_cluster_body};
use crate::app::core::driver::XyEvent;
use crate::app::tui::activity_fold::{
    format_elapsed_secs, partition_segments, strip_ansi_live_window, thought_header_body,
};
use crate::app::tui::bridge::{UiEntry, UiModel, apply_xy_event};
use crate::app::tui::widgets::GlyphSet;
use xylitol_tui::Component;

/// One semantic row: line index in the plain frame + chord + cluster head body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SceneDumpRow {
    pub line_idx: usize,
    pub chord: &'static str,
    /// Cluster head body (e.g. `Used 4 tools`, `Thought 1s`); empty for
    /// envelope rows and L1 blocks.
    pub cluster_head: String,
}

/// Pure-text semantic dump over a plain (ANSI-stripped) product frame.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SemanticDump {
    pub line_rows: Vec<SceneDumpRow>,
}

impl SemanticDump {
    /// Build the dump from a plain frame + the entries the product painted.
    ///
    /// Anchors are the cluster head bodies the product formatter produces for
    /// sealed clusters (progressive/live adjustments are product paint
    /// behavior; the anchor only names the row). Rows are matched top-down so
    /// wrapped entries do not shift attribution.
    pub fn from_product_frame(plain: &str, entries: &[UiEntry]) -> Self {
        let segs = partition_segments(entries);
        let glyphs = GlyphSet::from_env();
        let markers = [glyphs.fold().to_string(), glyphs.unfold().to_string()];

        // Expected cluster head bodies, in paint order.
        let mut anchors: Vec<(&'static str, String)> = Vec::new();
        for seg in &segs {
            for cl in &seg.clusters {
                let counts = count_cluster(entries, cl);
                if counts.omits_cluster_header() {
                    continue;
                }
                let thought_dur = counts
                    .is_thought_only()
                    .then(|| thought_duration_label(entries, cl))
                    .flatten();
                let mut body = format_cluster_body(&counts, false);
                if body == "Thought" {
                    body = thought_header_body(thought_dur.as_deref());
                }
                anchors.push(("L2 cluster", body));
            }
        }

        let is_marker_row = |line: &str| -> bool {
            let t = line.trim_start();
            markers.iter().any(|m| t.starts_with(m.as_str()))
        };

        let mut line_rows = Vec::new();
        let mut anchor_at = 0usize;
        for (line_idx, raw) in plain.lines().enumerate() {
            let line = raw.trim_end_matches(char::is_whitespace);
            if line.is_empty() {
                continue;
            }
            if line.contains("Worked for") {
                line_rows.push(SceneDumpRow {
                    line_idx,
                    chord: "L3 envelope",
                    cluster_head: String::new(),
                });
                continue;
            }
            // Match the next cluster head anchor (top-down; skips content rows).
            if let Some((chord, body)) = anchors.get(anchor_at)
                && is_marker_row(line)
                && line.contains(body.as_str())
            {
                line_rows.push(SceneDumpRow {
                    line_idx,
                    chord,
                    cluster_head: body.clone(),
                });
                anchor_at += 1;
                continue;
            }
            // Inflight thinking stream paints its own live cluster head
            // (`paint_folded_streaming_thought`), which is not a sealed anchor.
            if is_marker_row(line) && line.contains("Thinking") {
                line_rows.push(SceneDumpRow {
                    line_idx,
                    chord: "L2 cluster",
                    cluster_head: "Thinking".into(),
                });
                continue;
            }
            // Foldable middle content (kids expanded) — L1 block rows.
            if is_marker_row(line) {
                line_rows.push(SceneDumpRow {
                    line_idx,
                    chord: "L1 block",
                    cluster_head: String::new(),
                });
            }
        }
        SemanticDump { line_rows }
    }

    pub fn to_text(&self) -> String {
        let mut out = String::new();
        for row in &self.line_rows {
            if row.cluster_head.is_empty() {
                out.push_str(&format!("[{}] {}\n", row.line_idx, row.chord));
            } else {
                out.push_str(&format!(
                    "[{}] {} {}\n",
                    row.line_idx, row.chord, row.cluster_head
                ));
            }
        }
        out
    }

    /// Rows of one chord.
    pub fn rows_with_chord(&self, chord: &'static str) -> impl Iterator<Item = &SceneDumpRow> {
        self.line_rows.iter().filter(move |r| r.chord == chord)
    }
}

/// Sum of persisted thinking wall-clock secs for a cluster (mirrors
/// `scrollback.rs::thought_duration_label`, the product duration source).
fn thought_duration_label(
    entries: &[UiEntry],
    cluster: &crate::app::tui::activity_fold::ActivityCluster,
) -> Option<String> {
    let mut secs = 0u64;
    for idx in crate::app::tui::activity_fold::cluster_middle_indices(entries, cluster) {
        if let Some(UiEntry::Thinking {
            elapsed_secs: Some(s),
            ..
        }) = entries.get(idx)
        {
            secs = secs.saturating_add(*s);
        }
    }
    (secs > 0).then(|| format_elapsed_secs(secs))
}

/// Build a product scene by replaying [`XyEvent`]s through `apply_xy_event` —
/// the exact seam the live tape and harness use, so tool intent, thinking
/// flush, todo projection and Ask shapes are the product's, not fixtures'.
pub struct SceneBuilder {
    model: UiModel,
    live: Vec<XyEvent>,
}

impl SceneBuilder {
    pub fn begin() -> Self {
        let mut model = UiModel::new();
        model.begin_run("scene");
        Self {
            model,
            live: Vec::new(),
        }
    }

    fn push_xy(&mut self, event: XyEvent) -> &mut Self {
        apply_xy_event(&mut self.model, &event);
        self.live.push(event);
        self
    }

    pub fn thinking(&mut self, text: &str) -> &mut Self {
        self.push_xy(XyEvent::ThinkingDelta(text.into()))
    }

    pub fn tool_start(&mut self, id: &str, name: &str, path: &str) -> &mut Self {
        self.push_xy(XyEvent::ToolExecutionStart {
            id: id.into(),
            name: name.into(),
            args: serde_json::json!({ "path": path }),
        })
    }

    /// Plain tool end (no todo projection).
    pub fn tool_end(&mut self, id: &str, name: &str) -> &mut Self {
        self.push_xy(XyEvent::ToolExecutionEnd {
            id: id.into(),
            name: name.into(),
            result: "ok".into(),
            is_error: false,
        })
    }

    /// todo_* result through the product projection
    /// (`sync_todo_checklist_from_tool_result` inside `apply_tools_family`):
    /// the checklist row is a projection, not a Used call (lesson 1).
    pub fn todo_result(&mut self, id: &str, name: &str, result: &str) -> &mut Self {
        self.push_xy(XyEvent::ToolExecutionEnd {
            id: id.into(),
            name: name.into(),
            result: result.into(),
            is_error: false,
        })
    }

    pub fn assistant(&mut self, text: &str) -> &mut Self {
        self.push_xy(XyEvent::TextDelta(text.into()))
    }

    /// Flush streaming buffers the way MessageEnd does (product handler).
    pub fn message_end(&mut self) -> &mut Self {
        self.push_xy(XyEvent::MessageEnd {
            role: "assistant".into(),
            message: None,
        })
    }

    /// Seal a thinking burst the product way: [`XyEvent::ThinkingDelta`] then
    /// [`UiModel::flush_streaming_elapsed`] (same flush `MessageEnd` uses).
    ///
    /// Elapsed is pinned because a scene cannot steer
    /// [`std::time::Instant`]. This is **not** stuffing `UiEntry::Thinking`.
    pub fn thinking_flushed(&mut self, text: &str, elapsed_secs: u64) -> &mut Self {
        apply_xy_event(&mut self.model, &XyEvent::ThinkingDelta(text.into()));
        self.model.flush_streaming_elapsed(Some(elapsed_secs));
        self
    }

    /// Inflight stream for the live window. Identity is
    /// [`crate::app::tui::bridge::STREAMING_THINK_ID`], not a nonempty buffer.
    pub fn live_thinking(&mut self, text: &str) -> &mut Self {
        self.push_xy(XyEvent::ThinkingDelta(text.into()))
    }

    /// Live Xy events for [`HostSession::step(HostEvent::Xy)`]. Does not include
    /// [`Self::thinking_flushed`] (pinned elapsed is Resume-shaped).
    pub fn live_events(&self) -> &[XyEvent] {
        &self.live
    }

    pub fn entries(&self) -> &[UiEntry] {
        &self.model.entries
    }

    /// Live thinking buffers must be empty after a product flush.
    pub fn live_think_idle(&self) -> bool {
        self.model.streaming_think_id.is_none() && self.model.streaming_thinking.is_empty()
    }

    /// Product render of the current scene state (entries + inflight streams).
    pub fn render(&self, width: usize) -> (String, SemanticDump) {
        let mut root = crate::app::tui::layout::UiRoot::new();
        root.apply_ui_model(&self.model);
        root.touch_activity();
        let plain = strip_ansi_live_window(&root.render(width).join("\n"));
        let dump = SemanticDump::from_product_frame(&plain, &self.model.entries);
        (plain, dump)
    }
}
