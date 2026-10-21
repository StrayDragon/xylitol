use clap::{Args, Parser, Subcommand};

#[cfg(feature = "server")]
use xylitol_kernel::occupancy::subcommand::ServeAction;

#[cfg(feature = "tui")]
pub fn activity_fold_settings(
    c: xylitol_kernel::config::types::TuiActivityFoldConfig,
) -> xylitol_app_tui::ActivityFoldSettings {
    use xylitol_app_tui::SegmentLevel;
    use xylitol_kernel::config::types::ActivityFoldStreamCollapse;
    let collapse_floor = match c.stream_collapse {
        ActivityFoldStreamCollapse::Envelope => SegmentLevel::L3,
        ActivityFoldStreamCollapse::Clusters => SegmentLevel::L2,
    };
    xylitol_app_tui::ActivityFoldSettings {
        enabled: c.enabled,
        keep_recent_turns: c.keep_recent_turns,
        auto_on_rebuild: c.auto_on_rebuild,
        auto_on_turn_end: c.auto_on_turn_end,
        collapse_floor,
    }
}

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
    #[arg(long)]
    pub trust: bool,
    #[arg(long)]
    pub no_trust: bool,
    #[arg(long)]
    pub attach: Option<String>,
    #[arg(long)]
    pub port: Option<u16>,
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
    pub config: Option<String>,
    #[arg(long)]
    pub no_color: bool,
    #[arg(long)]
    pub trust: bool,
    #[arg(long)]
    pub no_trust: bool,
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
    #[cfg(feature = "server")]
    Serve {
        #[arg(long, default_value = "127.0.0.1")]
        host: String,
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
