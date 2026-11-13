use clap::{Args, Parser, Subcommand};

#[cfg(feature = "server")]
use xylitol_kernel::occupancy::subcommand::ServeAction;

#[cfg(feature = "tui")]
pub fn activity_fold_settings(
    c: xylitol_kernel::config::types::TuiActivityFoldConfig,
) -> xylitol_app_tui::ActivityFoldSettings {
    use xylitol_app_tui::{DisplayLevel, SegmentLevel, VisibleItemKind};
    use xylitol_kernel::config::types::{ActivityDisplayLevel, ActivityFoldStreamCollapse};
    let collapse_floor = match c.stream_collapse {
        ActivityFoldStreamCollapse::Envelope => SegmentLevel::L3,
        ActivityFoldStreamCollapse::Clusters => SegmentLevel::L2,
    };
    let display_level = match c.display_level {
        ActivityDisplayLevel::Quiet => DisplayLevel::Quiet,
        ActivityDisplayLevel::Summary => DisplayLevel::Summary,
        ActivityDisplayLevel::Recent => DisplayLevel::Recent,
        ActivityDisplayLevel::Full => DisplayLevel::Full,
    };
    let keep_recent_turns = if display_level == DisplayLevel::Recent {
        c.keep_recent_turns.max(display_level.keep_recent_turns())
    } else {
        c.keep_recent_turns
    };
    let mut settings = xylitol_app_tui::ActivityFoldSettings {
        enabled: c.enabled,
        keep_recent_turns,
        auto_on_rebuild: c.auto_on_rebuild,
        auto_on_turn_end: c.auto_on_turn_end,
        collapse_floor,
        display_level,
        ..Default::default()
    };
    if let Some(v) = c.visible_items {
        settings.set_visible_item(VisibleItemKind::Thinking, v.thinking);
        settings.set_visible_item(VisibleItemKind::ToolHeader, v.tool_header);
        settings.set_visible_item(VisibleItemKind::ToolOutput, v.tool_output);
        settings.set_visible_item(VisibleItemKind::Diff, v.diff);
    }
    settings
}

#[derive(Args, Debug, Default, Clone, PartialEq, Eq)]
pub struct TuiSurfaceArgs {
    #[arg(long)]
    pub session: Option<String>,
    #[arg(long)]
    pub model: Option<String>,
    #[arg(long)]
    pub agent_profile: Option<String>,
    #[arg(long)]
    pub config: Option<String>,
    #[arg(long)]
    pub list_models: bool,
    #[arg(long)]
    pub no_color: bool,
    #[arg(long, conflicts_with = "no_trust")]
    pub trust: bool,
    #[arg(long, conflicts_with = "trust")]
    pub no_trust: bool,
    #[arg(long)]
    pub attach: Option<String>,
    #[arg(long)]
    pub port: Option<u16>,
    #[arg(long)]
    pub no_ensure_host: bool,
}

#[derive(Subcommand, Debug, Clone, PartialEq, Eq)]
pub enum TuiAction {
    Run {
        #[command(flatten)]
        surface: TuiSurfaceArgs,
    },
    #[cfg(debug_assertions)]
    Designing,
}

#[derive(Args, Debug, Default, Clone, PartialEq, Eq)]
pub struct PrintSurfaceArgs {
    #[arg(long)]
    pub session: Option<String>,
    #[arg(long)]
    pub model: Option<String>,
    #[arg(long)]
    pub agent_profile: Option<String>,
    #[arg(long)]
    pub config: Option<String>,
    #[arg(long)]
    pub no_color: bool,
    #[arg(long, conflicts_with = "no_trust")]
    pub trust: bool,
    #[arg(long, conflicts_with = "trust")]
    pub no_trust: bool,
}

#[derive(Subcommand, Debug, Clone, PartialEq, Eq)]
pub enum ConfigAction {
    Schema {
        #[arg(long)]
        domain: Option<String>,
        #[arg(long)]
        ts: bool,
    },
    Check {
        #[arg(value_name = "PATH")]
        path: Option<String>,
        #[arg(long)]
        warn: bool,
    },
    Show {
        #[arg(long)]
        resolved: bool,
    },
    Init {
        #[arg(long)]
        preset: Option<String>,
        #[arg(long)]
        force: bool,
    },
}

#[derive(Subcommand, Debug)]
pub enum SessionsAction {
    Maintain {
        #[arg(long)]
        dir: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
pub enum CliCommand {
    Tui {
        #[command(flatten)]
        surface: TuiSurfaceArgs,
        #[command(subcommand)]
        action: Option<TuiAction>,
    },
    Print {
        #[command(flatten)]
        surface: PrintSurfaceArgs,
        #[arg(value_name = "PROMPT")]
        prompt: Option<String>,
        #[arg(short = 'p', long = "prompt", value_name = "TEXT")]
        prompt_flag: Option<String>,
    },
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
    Sessions {
        #[command(subcommand)]
        action: SessionsAction,
    },
    #[cfg(feature = "server")]
    Serve {
        #[arg(long, default_value = "127.0.0.1")]
        host: String,
        #[arg(long, default_value_t = 18790)]
        port: u16,
        #[arg(long)]
        lifecycle: Option<String>,
        #[arg(long)]
        idle_timeout: Option<String>,
        #[command(subcommand)]
        action: Option<ServeAction>,
    },
}

#[derive(Parser, Debug)]
#[command(
    name = "xylitol",
    version,
    about = "XYlitol — personal coding agent; default entry is the interactive TUI"
)]
pub struct CliArgs {
    #[command(subcommand)]
    pub command: Option<CliCommand>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct SurfaceBootstrap {
    pub session: Option<String>,
    pub model: Option<String>,
    pub agent_profile: Option<String>,
    pub config: Option<String>,
    pub list_models: bool,
    pub no_color: bool,
    pub trust: bool,
    pub no_trust: bool,
    pub attach: Option<String>,
    pub port: Option<u16>,
    pub no_ensure_host: bool,
}
