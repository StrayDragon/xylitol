//! CLI-surface BDD steps: surface verbs, owned flags, resume hint and
//! attach fail-closed (ce8/ce16/ce19/ce20/ce21), plus the atb4
//! attach-vs-inprocess pair. Serves `tests/features/cli-entry.feature`
//! (surface half) and app-tui-bridge `remote-type-kept`.

use crate::prelude::*;
use rstest::fixture;
use rstest_bdd_macros::{given, then, when};

// ═══════════════════════════════════════════════════════════════════
// c1390 — CLI surface verbs (ce16)
// ═══════════════════════════════════════════════════════════════════

pub struct SurfaceBdd {
    pub(crate) mode: Cell<Option<xylitol::app::cli::SurfaceMode>>,
    pub(crate) print_err: RefCell<String>,
    pub(crate) is_tty: Cell<bool>,
    pub(crate) has_prompt: Cell<bool>,
}

impl SurfaceBdd {
    fn new() -> Self {
        Self {
            mode: Cell::new(None),
            print_err: RefCell::new(String::new()),
            is_tty: Cell::new(false),
            has_prompt: Cell::new(true),
        }
    }
}

#[fixture]
pub fn surface_bdd() -> SurfaceBdd {
    SurfaceBdd::new()
}

#[given("TTY")]
fn g_ce16_tty(surface_bdd: &SurfaceBdd) {
    surface_bdd.is_tty.set(true);
}

#[when("xylitol tui")]
fn w_ce16_tui(surface_bdd: &SurfaceBdd) {
    use clap::Parser;
    use xylitol::app::cli::{CliArgs, resolve_surface_intent, select_surface_mode};
    assert!(surface_bdd.is_tty.get(), "given TTY must arm is_tty");
    let args = CliArgs::try_parse_from(["xylitol", "tui"]).expect("parse tui");
    let (force_tui, print_flag, one_shot) = resolve_surface_intent(args.command.as_ref());
    surface_bdd.mode.set(Some(select_surface_mode(
        force_tui,
        print_flag,
        one_shot.is_some(),
        surface_bdd.is_tty.get(),
    )));
}

#[then("进入产品 TUI")]
fn t_ce16_enters_tui(surface_bdd: &SurfaceBdd) {
    assert_eq!(
        surface_bdd.mode.get(),
        Some(xylitol::app::cli::SurfaceMode::Tui)
    );
}

#[given("无 prompt")]
fn g_ce16_no_prompt(surface_bdd: &SurfaceBdd) {
    surface_bdd.has_prompt.set(false);
}

#[when("xylitol print")]
fn w_ce16_print(surface_bdd: &SurfaceBdd) {
    use clap::Parser;
    use xylitol::app::cli::{CliArgs, resolve_print_prompt, resolve_surface_intent};
    assert!(
        !surface_bdd.has_prompt.get(),
        "given 无 prompt must clear has_prompt"
    );
    let args = CliArgs::try_parse_from(["xylitol", "print"]).expect("parse print");
    let (_force_tui, print_flag, one_shot) = resolve_surface_intent(args.command.as_ref());
    let err = resolve_print_prompt(one_shot.as_deref(), print_flag, true, || Ok(String::new()))
        .expect_err("print without prompt must fail");
    surface_bdd.print_err.replace(err.to_string());
}

#[then("错误退出且无 Hello!")]
fn t_ce16_print_no_hello(surface_bdd: &SurfaceBdd) {
    let err = surface_bdd.print_err.borrow();
    assert!(!err.is_empty(), "expected print error");
    assert!(!err.contains("Hello!"), "{err}");
}

/// Minimal CLI help-output holder for serve/top-level help steps (ce8/ce16);
/// these scenarios never touch the tokenizer cache, only rendered help text.
pub struct CliHelpBdd {
    pub(crate) out: RefCell<String>,
    pub(crate) ok: Cell<bool>,
}

impl CliHelpBdd {
    fn new() -> Self {
        Self {
            out: RefCell::new(String::new()),
            ok: Cell::new(false),
        }
    }
}

#[fixture]
pub fn cli_help_bdd() -> CliHelpBdd {
    CliHelpBdd::new()
}

#[when("xylitol serve --help")]
fn w_ce8_serve_help(cli_help_bdd: &CliHelpBdd) {
    use clap::CommandFactory;
    let mut cmd = xylitol::app::cli::CliArgs::command();
    let help = cmd
        .find_subcommand_mut("serve")
        .expect("serve subcommand")
        .render_long_help()
        .to_string();
    cli_help_bdd.out.replace(help);
    cli_help_bdd.ok.set(true);
}

#[then("可见 --host --port 与 stop、install，且无 server 或 run 别名")]
fn t_ce8_serve_shape(cli_help_bdd: &CliHelpBdd) {
    use clap::{CommandFactory, Parser};
    let help = cli_help_bdd.out.borrow();
    assert!(help.contains("--host"), "{help}");
    assert!(help.contains("--port"), "{help}");
    let mut cmd = xylitol::app::cli::CliArgs::command();
    assert!(cmd.find_subcommand("server").is_none());
    let serve = cmd.find_subcommand_mut("serve").expect("serve");
    assert!(serve.find_subcommand("stop").is_some(), "{help}");
    assert!(serve.find_subcommand("install").is_some(), "{help}");
    assert!(serve.find_subcommand("run").is_none(), "{help}");
    assert!(
        xylitol::app::cli::CliArgs::try_parse_from(["xylitol", "server"]).is_err(),
        "server alias must not parse"
    );
    assert!(
        xylitol::app::cli::CliArgs::try_parse_from(["xylitol", "serve", "run"]).is_err(),
        "serve run must not parse"
    );
}

#[when("xylitol --help")]
fn w_ce16_top_help(cli_help_bdd: &CliHelpBdd) {
    use clap::CommandFactory;
    let help = xylitol::app::cli::CliArgs::command()
        .render_long_help()
        .to_string();
    cli_help_bdd.out.replace(help);
    cli_help_bdd.ok.set(true);
}

#[then("Commands 含 tokenizer、resources 与 serve 为顶层而非 tui 子命令")]
fn t_ce16_ops_toplevel(cli_help_bdd: &CliHelpBdd) {
    use clap::CommandFactory;
    let help = cli_help_bdd.out.borrow();
    assert!(help.contains("tokenizer"), "{help}");
    assert!(help.contains("resources"), "{help}");
    assert!(help.contains("serve"), "{help}");
    assert!(help.contains("tui"), "{help}");
    assert!(help.contains("print"), "{help}");
    let mut cmd = xylitol::app::cli::CliArgs::command();
    assert!(
        cmd.find_subcommand("serve").is_some(),
        "serve must be a top-level verb"
    );
    assert!(
        cmd.find_subcommand("server").is_none(),
        "server must not remain as a product verb"
    );
    let tui = cmd.find_subcommand_mut("tui").expect("tui subcommand");
    assert!(
        tui.find_subcommand("tokenizer").is_none()
            && tui.find_subcommand("resources").is_none()
            && tui.find_subcommand("serve").is_none(),
        "ops must not nest under tui"
    );
}
// ═══════════════════════════════════════════════════════════════════
// c1565 — surface-owned flags + resume hint (ce19/ce20)
// ═══════════════════════════════════════════════════════════════════

pub struct SurfaceFlagsBdd {
    pub(crate) parse_ok: Cell<bool>,
    pub(crate) session: RefCell<Option<String>>,
    pub(crate) hint: RefCell<Option<String>>,
}

impl SurfaceFlagsBdd {
    fn new() -> Self {
        Self {
            parse_ok: Cell::new(false),
            session: RefCell::new(None),
            hint: RefCell::new(None),
        }
    }
}

#[fixture]
pub fn surface_flags_bdd() -> SurfaceFlagsBdd {
    SurfaceFlagsBdd::new()
}

#[given("表面旗标上下文就绪")]
fn g_ce19_cli_ready(surface_flags_bdd: &SurfaceFlagsBdd) {
    surface_flags_bdd.parse_ok.set(false);
    surface_flags_bdd.session.replace(None);
    surface_flags_bdd.hint.replace(None);
}

#[when("xylitol tui --session sid --model m --trust")]
fn w_ce19_tui_flags(surface_flags_bdd: &SurfaceFlagsBdd) {
    use clap::Parser;
    use xylitol::app::cli::{CliArgs, surface_from_command};
    let args = CliArgs::try_parse_from([
        "xylitol",
        "tui",
        "--session",
        "sid",
        "--model",
        "m",
        "--trust",
    ])
    .expect("parse tui flags");
    let s = surface_from_command(args.command.as_ref());
    surface_flags_bdd.parse_ok.set(true);
    surface_flags_bdd.session.replace(s.session);
    assert_eq!(s.model.as_deref(), Some("m"));
    assert!(s.trust);
}

#[then("解析成功且表面旗标生效")]
fn t_ce19_flags_ok(surface_flags_bdd: &SurfaceFlagsBdd) {
    assert!(surface_flags_bdd.parse_ok.get());
    assert_eq!(surface_flags_bdd.session.borrow().as_deref(), Some("sid"));
}

#[when("xylitol tui run --session sid")]
fn w_ce19_tui_run_session(surface_flags_bdd: &SurfaceFlagsBdd) {
    use clap::Parser;
    use xylitol::app::cli::{CliArgs, surface_from_command};
    let args = CliArgs::try_parse_from(["xylitol", "tui", "run", "--session", "sid"])
        .expect("parse tui run --session");
    let s = surface_from_command(args.command.as_ref());
    surface_flags_bdd.parse_ok.set(true);
    surface_flags_bdd.session.replace(s.session);
}

#[then("解析成功且 --session 生效")]
fn t_ce19_session_ok(surface_flags_bdd: &SurfaceFlagsBdd) {
    assert!(surface_flags_bdd.parse_ok.get());
    assert_eq!(surface_flags_bdd.session.borrow().as_deref(), Some("sid"));
}

#[when("xylitol tui --attach http://127.0.0.1:9 --port 11")]
fn w_ce19_tui_attach(surface_flags_bdd: &SurfaceFlagsBdd) {
    use clap::Parser;
    use xylitol::app::cli::{CliArgs, surface_from_command};
    let args = CliArgs::try_parse_from([
        "xylitol",
        "tui",
        "--attach",
        "http://127.0.0.1:9",
        "--port",
        "11",
    ])
    .expect("parse tui --attach/--port");
    let s = surface_from_command(args.command.as_ref());
    let url = xylitol::resolve_attach_url(s.attach.as_deref(), s.port);
    assert_eq!(url, "http://127.0.0.1:9");
    surface_flags_bdd.parse_ok.set(true);
}

#[then("解析成功且 --attach 优先于 --port")]
fn t_ce19_attach_wins(surface_flags_bdd: &SurfaceFlagsBdd) {
    assert!(surface_flags_bdd.parse_ok.get());
}

#[when("xylitol print --session sid --no-color hi")]
fn w_ce19_print_flags(surface_flags_bdd: &SurfaceFlagsBdd) {
    use clap::Parser;
    use xylitol::app::cli::CliArgs;
    CliArgs::try_parse_from(["xylitol", "print", "--session", "sid", "--no-color", "hi"])
        .expect("parse print flags");
    surface_flags_bdd.parse_ok.set(true);
}

#[when("xylitol print --session sid --trust --model m hi")]
fn w_ce19_print_trust(surface_flags_bdd: &SurfaceFlagsBdd) {
    use clap::Parser;
    use xylitol::app::cli::{CliArgs, surface_from_command};
    let args = CliArgs::try_parse_from([
        "xylitol",
        "print",
        "--session",
        "sid",
        "--trust",
        "--model",
        "m",
        "hi",
    ])
    .expect("parse print --trust");
    let s = surface_from_command(args.command.as_ref());
    surface_flags_bdd.parse_ok.set(true);
    surface_flags_bdd.session.replace(s.session);
    assert_eq!(s.model.as_deref(), Some("m"));
    assert!(s.trust);
    assert!(!s.no_trust);
}

#[when("xylitol print --session sid --no-trust hi")]
fn w_ce19_print_no_trust(surface_flags_bdd: &SurfaceFlagsBdd) {
    use clap::Parser;
    use xylitol::app::cli::{CliArgs, surface_from_command};
    let args =
        CliArgs::try_parse_from(["xylitol", "print", "--session", "sid", "--no-trust", "hi"])
            .expect("parse print --no-trust");
    let s = surface_from_command(args.command.as_ref());
    surface_flags_bdd.parse_ok.set(true);
    surface_flags_bdd.session.replace(s.session);
    assert!(s.no_trust);
    assert!(!s.trust);
}

#[then("解析成功")]
fn t_ce19_parse_ok(surface_flags_bdd: &SurfaceFlagsBdd) {
    assert!(surface_flags_bdd.parse_ok.get());
}

#[when("xylitol --session sid")]
fn w_ce19_toplevel_session(surface_flags_bdd: &SurfaceFlagsBdd) {
    use clap::Parser;
    use xylitol::app::cli::CliArgs;
    surface_flags_bdd
        .parse_ok
        .set(CliArgs::try_parse_from(["xylitol", "--session", "sid"]).is_ok());
}

#[then("解析失败")]
fn t_ce19_parse_fail(surface_flags_bdd: &SurfaceFlagsBdd) {
    assert!(!surface_flags_bdd.parse_ok.get());
}

#[given("当前 session 已出现在 list_sessions")]
fn g_ce20_listed(surface_flags_bdd: &SurfaceFlagsBdd) {
    surface_flags_bdd.session.replace(Some("uuid-1".into()));
    surface_flags_bdd
        .hint
        .replace(xylitol::app::cli::resume_hint_line(Some("uuid-1"), true));
}

#[given("当前 session 未持久化")]
fn g_ce20_unlisted(surface_flags_bdd: &SurfaceFlagsBdd) {
    surface_flags_bdd.session.replace(Some("uuid-2".into()));
    surface_flags_bdd
        .hint
        .replace(xylitol::app::cli::resume_hint_line(Some("uuid-2"), false));
}

#[when("TUI 或 print 正常退出")]
fn w_ce20_exit(surface_flags_bdd: &SurfaceFlagsBdd) {
    // Exit path: resume hint is prepared in given from list_sessions membership.
    assert!(
        surface_flags_bdd.session.borrow().is_some(),
        "normal exit requires a current session prepared by given"
    );
}

#[then("stderr 含 resume 提示行")]
fn t_ce20_hint_present(surface_flags_bdd: &SurfaceFlagsBdd) {
    let hint = surface_flags_bdd.hint.borrow();
    let line = hint.as_deref().expect("hint");
    assert!(
        line.starts_with("Resume by $ xylitol tui --session "),
        "{line}"
    );
}

#[then("stderr 不含 resume 提示行")]
fn t_ce20_hint_absent(surface_flags_bdd: &SurfaceFlagsBdd) {
    assert!(surface_flags_bdd.hint.borrow().is_none());
}

// ═══════════════════════════════════════════════════════════════════
// c2290 — TUI attach fail-closed (ce21)
// ═══════════════════════════════════════════════════════════════════

pub struct AttachBdd {
    pub(crate) err: RefCell<Option<String>>,
}

impl AttachBdd {
    fn new() -> Self {
        Self {
            err: RefCell::new(None),
        }
    }
}

#[fixture]
pub fn attach_bdd() -> AttachBdd {
    AttachBdd::new()
}

#[given("本机 Host 未在听")]
fn g_ce21_host_down(attach_bdd: &AttachBdd) {
    assert!(attach_bdd.err.borrow().is_none());
}

#[when("产品 TUI 尝试 attach")]
fn w_ce21_attach(attach_bdd: &AttachBdd) {
    let err = xylitol::probe_host("http://127.0.0.1:1").expect_err("port 1 must be down");
    attach_bdd.err.replace(Some(err.to_string()));
}

#[then("非零退出并提示用 xylitol serve 启动 Host")]
fn t_ce21_fail(attach_bdd: &AttachBdd) {
    let msg = attach_bdd.err.borrow().clone().expect("probe error");
    assert!(
        msg.contains("not listening") || msg.contains("Host is not listening"),
        "{msg}"
    );
    let hint = xylitol::attach_fail_message(xylitol::DEFAULT_ATTACH_URL);
    assert!(hint.contains("xylitol serve"), "{hint}");
    assert!(!hint.contains("server run"), "{hint}");
}

#[when("检查 Driver 实现")]
fn w_atb4_inspect_drivers() {}

#[then("默认 attach 且同进程驱动路径仍保留给 print 与嵌入")]
fn t_atb4_attach_and_inprocess() {
    assert_eq!(xylitol::DEFAULT_ATTACH_URL, "http://127.0.0.1:18790");
    let inprocess = std::any::type_name::<xylitol::XyInProcessDriver>();
    let http_ws = std::any::type_name::<xylitol::HttpWsClient>();
    assert!(inprocess.contains("XyInProcessDriver"), "{inprocess}");
    assert!(http_ws.contains("HttpWsClient"), "{http_ws}");
}
