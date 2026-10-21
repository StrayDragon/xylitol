use super::summary::{count_cluster, format_cluster_body};
use crate::activity_fold::{
    format_elapsed_secs, partition_segments, strip_ansi_live_window, thought_header_body,
};
use crate::bridge::{UiEntry, UiModel, apply_xy_event};
use crate::driver::XyEvent;
use crate::widgets::GlyphSet;
use xylitol_tui::Component;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SceneDumpRow {
    pub line_idx: usize,
    pub chord: &'static str,
    pub cluster_head: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SemanticDump {
    pub line_rows: Vec<SceneDumpRow>,
}

impl SemanticDump {
    pub fn from_product_frame(plain: &str, entries: &[UiEntry]) -> Self {
        let segs = partition_segments(entries);
        let glyphs = GlyphSet::from_env();
        let markers = [glyphs.fold().to_string(), glyphs.unfold().to_string()];

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
            if is_marker_row(line) && line.contains("Thinking") {
                let chord = if line.contains("Ctrl+T") {
                    "L1 block"
                } else {
                    "L2 cluster"
                };
                line_rows.push(SceneDumpRow {
                    line_idx,
                    chord,
                    cluster_head: "Thinking".into(),
                });
                continue;
            }
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

    pub fn rows_with_chord(&self, chord: &'static str) -> impl Iterator<Item = &SceneDumpRow> {
        self.line_rows.iter().filter(move |r| r.chord == chord)
    }
}

fn thought_duration_label(
    entries: &[UiEntry],
    cluster: &crate::activity_fold::ActivityCluster,
) -> Option<String> {
    let mut secs = 0u64;
    for idx in crate::activity_fold::cluster_middle_indices(entries, cluster) {
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

    pub fn tool_end(&mut self, id: &str, name: &str) -> &mut Self {
        self.push_xy(XyEvent::ToolExecutionEnd {
            id: id.into(),
            name: name.into(),
            result: "ok".into(),
            is_error: false,
        })
    }

    pub fn todo_result(&mut self, id: &str, name: &str, result: &str) -> &mut Self {
        if let Ok(list) = serde_json::from_str::<xylitol_plugin_sdk::session::TodoList>(result) {
            self.push_xy(XyEvent::TodoUpdated { list });
        }
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

    pub fn compaction(&mut self, summary: &str, tokens_before: u64) -> &mut Self {
        self.push_xy(XyEvent::CompactionStart {
            reason: "threshold".into(),
        });
        self.push_xy(XyEvent::CompactionEnd {
            result: Some("ok".into()),
            aborted: false,
            reason: "threshold".into(),
            will_retry: false,
            error_message: None,
            summary: Some(summary.into()),
            tokens_before: Some(tokens_before),
            tokens_after: None,
            notice: None,
        })
    }

    pub fn message_end(&mut self) -> &mut Self {
        self.push_xy(XyEvent::MessageEnd {
            role: "assistant".into(),
            message: None,
        })
    }

    pub fn thinking_flushed(&mut self, text: &str, elapsed_secs: u64) -> &mut Self {
        apply_xy_event(&mut self.model, &XyEvent::ThinkingDelta(text.into()));
        self.model.flush_streaming_elapsed(Some(elapsed_secs));
        self
    }

    pub fn live_thinking(&mut self, text: &str) -> &mut Self {
        self.push_xy(XyEvent::ThinkingDelta(text.into()))
    }

    pub fn live_events(&self) -> &[XyEvent] {
        &self.live
    }

    pub fn entries(&self) -> &[UiEntry] {
        &self.model.entries
    }

    pub fn into_model(self) -> UiModel {
        self.model
    }

    pub fn live_think_idle(&self) -> bool {
        self.model.streaming_think_id.is_none() && self.model.streaming_thinking.is_empty()
    }

    pub fn render(&self, width: usize) -> (String, SemanticDump) {
        let mut root = crate::layout::UiRoot::new();
        root.apply_ui_model(&self.model);
        root.touch_activity();
        let plain = strip_ansi_live_window(&root.render(width).join("\n"));
        let dump = SemanticDump::from_product_frame(&plain, &self.model.entries);
        (plain, dump)
    }
}
