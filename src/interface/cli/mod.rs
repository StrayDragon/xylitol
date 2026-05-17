//! CLI argument parsing and mode dispatch.
//!
//! Uses clap derive for arg parsing with three dispatch modes:
//! Print (always), Interactive (feature `ui-tui`), Acp (feature `infra-acp`).

use clap::Parser;

use crate::infra::config;

/// xylitol — LLM-Augmented Development Toolkit
#[derive(Parser, Debug)]
#[command(name = "xylitol", version, about)]
pub(crate) struct CliArgs {
    /// Prompt to process
    pub(crate) prompt: Option<String>,

    /// Run mode: print, interactive, or acp
    #[arg(long, default_value = "print")]
    pub(crate) mode: RunMode,

    /// Path to config file
    #[arg(long)]
    pub(crate) config: Option<String>,

    /// Project root directory
    #[arg(long)]
    pub(crate) project: Option<String>,

    /// Override default model
    #[arg(long)]
    pub(crate) model: Option<String>,

    /// Skip all confirmations
    #[arg(long)]
    pub(crate) yolo: bool,
}

/// Operating mode of the CLI.
#[derive(Clone, Debug, clap::ValueEnum)]
pub(crate) enum RunMode {
    /// Non-interactive, stream output to stdout
    Print,
    /// TUI mode (requires ui-tui feature)
    #[cfg(feature = "ui-tui")]
    Interactive,
    /// ACP over stdio (requires infra-acp feature)
    #[cfg(feature = "infra-acp")]
    Acp,
}

/// Entry point: parse args → load config → dispatch to mode.
pub(crate) fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args = CliArgs::parse();

    tracing::info!("starting xylitol in {:?} mode", args.mode);

    let config_path = args.config.as_deref().map(std::path::Path::new);
    let _config = config::load_app_config(config_path)?;

    // Override project root if --project was provided
    if let Some(project) = args.project {
        tracing::info!("project root: {project}");
    }

    // Override model if --model was provided
    if let Some(model) = args.model {
        tracing::info!("model override: {model}");
    }

    // Dispatch to mode-specific logic
    dispatch_mode(args.mode)?;

    Ok(())
}

/// Dispatch to the appropriate run mode handler.
fn dispatch_mode(mode: RunMode) -> Result<(), Box<dyn std::error::Error>> {
    match mode {
        RunMode::Print => {
            tracing::info!("Print mode — implemented in c30-add-print-mode");
            Ok(())
        }
        #[cfg(feature = "ui-tui")]
        RunMode::Interactive => {
            tracing::info!("Interactive/TUI mode — implemented in c80-add-tui");
            Ok(())
        }
        #[cfg(feature = "infra-acp")]
        RunMode::Acp => {
            tracing::info!("ACP mode — implemented in c87-add-acp-mode");
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_mode_is_print() {
        let args = CliArgs::parse_from(["xylitol"]);
        assert!(matches!(args.mode, RunMode::Print));
    }

    #[test]
    fn test_mode_print_explicit() {
        let args = CliArgs::parse_from(["xylitol", "--mode", "print"]);
        assert!(matches!(args.mode, RunMode::Print));
    }

    #[test]
    fn test_config_path() {
        let args = CliArgs::parse_from(["xylitol", "--config", "/tmp/test.yaml"]);
        assert_eq!(args.config.as_deref(), Some("/tmp/test.yaml"));
    }

    #[test]
    fn test_project_path() {
        let args = CliArgs::parse_from(["xylitol", "--project", "/my/project"]);
        assert_eq!(args.project.as_deref(), Some("/my/project"));
    }

    #[test]
    fn test_model_override() {
        let args = CliArgs::parse_from(["xylitol", "--model", "claude-opus-4"]);
        assert_eq!(args.model.as_deref(), Some("claude-opus-4"));
    }

    #[test]
    fn test_yolo_flag() {
        let args = CliArgs::parse_from(["xylitol", "--yolo"]);
        assert!(args.yolo);
    }

    #[test]
    fn test_yolo_default_false() {
        let args = CliArgs::parse_from(["xylitol"]);
        assert!(!args.yolo);
    }

    #[test]
    fn test_prompt_argument() {
        let args = CliArgs::parse_from(["xylitol", "my prompt here"]);
        assert_eq!(args.prompt.as_deref(), Some("my prompt here"));
    }

    #[test]
    fn test_all_options() {
        let args = CliArgs::parse_from([
            "xylitol",
            "--mode",
            "print",
            "--config",
            "/tmp/c.yaml",
            "--project",
            "/proj",
            "--model",
            "gpt-4o",
            "--yolo",
            "some prompt",
        ]);
        assert!(matches!(args.mode, RunMode::Print));
        assert_eq!(args.config.as_deref(), Some("/tmp/c.yaml"));
        assert_eq!(args.project.as_deref(), Some("/proj"));
        assert_eq!(args.model.as_deref(), Some("gpt-4o"));
        assert!(args.yolo);
        assert_eq!(args.prompt.as_deref(), Some("some prompt"));
    }
}
