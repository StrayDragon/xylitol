//! clap vocabulary for the product CLI.

use clap::{Args, Parser, Subcommand};

#[cfg(feature = "server")]
use crate::wiring::occupancy::subcommand::ServeAction;

/// Map `tui.activity_fold` config DTO → runtime knobs at the config-owning CLI
/// boundary (the TUI widget layer does not import wiring config types, c2277).
#[cfg(feature = "tui")]
impl From<crate::wiring::config::types::TuiActivityFoldConfig>
    for xylitol_app_tui::ActivityFoldSettings
{
    fn from(c: crate::wiring::config::types::TuiActivityFoldConfig) -> Self {
        use crate::wiring::config::types::ActivityFoldStreamCollapse;
        use xylitol_app_tui::SegmentLevel;
        let collapse_floor = match c.stream_collapse {
            ActivityFoldStreamCollapse::Envelope => SegmentLevel::L3,
            ActivityFoldStreamCollapse::Clusters => SegmentLevel::L2,
        };
        Self {
            enabled: c.enabled,
            keep_recent_turns: c.keep_recent_turns,
            auto_on_rebuild: c.auto_on_rebuild,
            auto_on_turn_end: c.auto_on_turn_end,
            collapse_floor,
        }
    }
}

/// Surface flags for `xylitol tui` / `xylitol tui run` (c1565).
#[derive(Args, Debug, Default, Clone, PartialEq, Eq)]
pub struct TuiSurfaceArgs {
    #[arg(long)]
    pub session: Option<String>,
    #[arg(long)]
    pub model: Option<String>,
    #[arg(long)]
    pub config: Option<String>,
    #[arg(long)]
    pub list_models: bool,
    #[arg(long)]
    pub no_color: bool,
    /// Trust the project directory and load its `.xylitol/` resources.
    #[arg(long)]
    pub trust: bool,
    /// Do not trust the project directory; skip its `.xylitol/` resources.
    #[arg(long)]
    pub no_trust: bool,
    /// Host origin to attach (`http://host:port`). Wins over `--port`.
    #[arg(long)]
    pub attach: Option<String>,
    /// Attach `http://127.0.0.1:<port>` when `--attach` is omitted.
    #[arg(long)]
    pub port: Option<u16>,
}

/// Optional leaf under `xylitol tui` (default = run when omitted).
/// Flags may sit on the parent (`tui --session …`) or on `run` (`tui run --session …`).
#[derive(Subcommand, Debug, Clone, PartialEq, Eq)]
pub enum TuiAction {
    /// Open the interactive TUI (same as bare `xylitol tui`).
    Run {
        #[command(flatten)]
        surface: TuiSurfaceArgs,
    },
    /// Preview composition: same TUI client, fake Host, scene slot. Debug builds only.
    #[cfg(debug_assertions)]
    Designing,
}

/// Surface flags for `xylitol print` (c1565 / c1620).
#[derive(Args, Debug, Default, Clone, PartialEq, Eq)]
pub struct PrintSurfaceArgs {
    #[arg(long)]
    pub session: Option<String>,
    #[arg(long)]
    pub model: Option<String>,
    #[arg(long)]
    pub config: Option<String>,
    #[arg(long)]
    pub no_color: bool,
    /// Trust the project directory and load its `.xylitol/` resources.
    #[arg(long)]
    pub trust: bool,
    /// Do not trust the project directory; skip its `.xylitol/` resources.
    #[arg(long)]
    pub no_trust: bool,
}

/// Top-level subcommand. When absent: TTY → TUI; non-TTY → print (stdin).
/// Surface verbs (`tui` / `print`) vs ops (`serve`). No flat surface aliases.
#[derive(Subcommand, Debug)]
pub enum CliCommand {
    /// Interactive TUI surface (default on a TTY).
    Tui {
        #[command(flatten)]
        surface: TuiSurfaceArgs,
        #[command(subcommand)]
        action: Option<TuiAction>,
    },
    /// One-shot print surface (requires a non-empty prompt).
    Print {
        #[command(flatten)]
        surface: PrintSurfaceArgs,
        /// Positional one-shot prompt.
        #[arg(value_name = "PROMPT")]
        prompt: Option<String>,
        /// Prompt via flag (`--prompt` / `-p`) under `print` only.
        #[arg(short = 'p', long = "prompt", value_name = "TEXT")]
        prompt_flag: Option<String>,
    },
    /// Host listener (default 127.0.0.1:18790). Closing the TUI does not stop it.
    #[cfg(feature = "server")]
    Serve {
        /// Bind address.
        #[arg(long, default_value = "127.0.0.1")]
        host: String,
        /// Bind port (`0` lets the OS assign and prints the actual port).
        #[arg(long, default_value_t = 18790)]
        port: u16,
        #[command(subcommand)]
        action: Option<ServeAction>,
    },
}

#[derive(Parser, Debug)]
#[command(
    name = "xylitol",
    version,
    about = "Personal coding agent — default entry is the interactive TUI"
)]
pub struct CliArgs {
    #[command(subcommand)]
    pub command: Option<CliCommand>,
}

/// Resolved surface bootstrap knobs (from `tui` / `print` flatten args).
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct SurfaceBootstrap {
    pub session: Option<String>,
    pub model: Option<String>,
    pub config: Option<String>,
    pub list_models: bool,
    pub no_color: bool,
    pub trust: bool,
    pub no_trust: bool,
    pub attach: Option<String>,
    pub port: Option<u16>,
}
