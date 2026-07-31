//! `/mcp` readonly panel slot methods for [`UiRoot`] (c1210).

use super::super::slots::EditorSlot;
use super::UiRoot;
use crate::app::core::driver::{LoadedResourcesSnapshot, McpServerPhase};

impl UiRoot {
    pub fn mcp_open(&self) -> bool {
        self.slot == EditorSlot::Mcp
    }

    #[cfg(test)]
    pub fn status_next_turn_cue_for_test(&self) -> Option<String> {
        self.status_next_turn_cue.clone()
    }

    /// Mount `/mcp` readonly panel from a loaded-resources snapshot (c1210).
    pub fn mount_mcp_panel(&mut self, snap: &LoadedResourcesSnapshot) {
        let connected = snap
            .mcp_servers
            .iter()
            .filter(|s| s.phase == McpServerPhase::Connected)
            .count();
        let armed = snap.mcp_servers.iter().filter(|s| s.tools_armed).count();
        let mut lines = vec![format!(
            " configured {} · connected {} · armed {}",
            snap.mcp_configured, connected, armed
        )];
        if snap.mcp_servers.is_empty() {
            lines.push(" (no MCP servers configured)".into());
        } else {
            for s in &snap.mcp_servers {
                let phase = match s.phase {
                    McpServerPhase::Connecting => "connecting",
                    McpServerPhase::Connected => "connected",
                    McpServerPhase::Failed => "failed",
                };
                let armed = if s.tools_armed { "armed" } else { "not armed" };
                lines.push(format!(
                    " {}  {}  {}  tools={}",
                    s.id, phase, armed, s.tool_count
                ));
            }
        }
        if !snap.mcp_diag_short.is_empty() {
            lines.push(format!(" diag: {}", snap.mcp_diag_short.join("; ")));
        }
        lines.push(" Esc to close".into());
        self.mcp_panel_lines = lines;
        self.slot = EditorSlot::Mcp;
    }

    #[cfg(test)]
    pub fn mcp_panel_text_for_test(&self) -> String {
        self.mcp_panel_lines.join("\n")
    }
}
