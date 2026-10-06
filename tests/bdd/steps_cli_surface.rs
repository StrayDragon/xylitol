//! CLI-surface BDD steps: surface verbs, owned flags, resume hint and
//! attach fail-closed (ce8/ce16/ce19/ce20/ce21), plus the atb4
//! attach-vs-inprocess pair. Serves `tests/features/cli-entry.feature`
//! (surface half) and app-tui-bridge `remote-type-kept`.

use crate::bdd::prelude::*;
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

use crate::bdd::fixtures::{AgentState, Workspace};

// ═══════════════════════════════════════════════════════════════════
// c2826 specs-compact — 新增裸规则场景步骤（r69/r70/r1378/r1386/r1387/r1389）
// ═══════════════════════════════════════════════════════════════════

pub struct CliEntryBdd {
    pub(crate) ok: Cell<bool>,
    pub(crate) detail: RefCell<String>,
}

impl CliEntryBdd {
    fn new() -> Self {
        Self {
            ok: Cell::new(false),
            detail: RefCell::new(String::new()),
        }
    }
}

#[fixture]
pub fn cli_entry_bdd() -> CliEntryBdd {
    CliEntryBdd::new()
}

#[when("在产品命令模块解析 {a:string} 与 {b:string}")]
fn w_c2826_parse_slash_pair(cli_entry_bdd: &CliEntryBdd, a: String, b: String) {
    use xylitol::app::tui::parse_slash_command;
    let first = parse_slash_command(a.trim_matches('"'));
    let second = parse_slash_command(b.trim_matches('"'));
    let summary = format!("first={:?} second={:?}", first.is_some(), second.is_some());
    cli_entry_bdd.detail.replace(summary);
}

#[then("仅 session-tree 得到待发命令且废弃短名不解析")]
fn t_c2826_legacy_names_rejected(cli_entry_bdd: &CliEntryBdd) {
    let detail = cli_entry_bdd.detail.borrow();
    assert_eq!(
        detail.as_str(),
        "first=false second=true",
        "c2826: 废弃短名 MUST NOT 解析，产品名必须解析：{detail}"
    );
}

#[then("分别得到模型与退出待发命令")]
fn t_c2826_model_exit_parsed(cli_entry_bdd: &CliEntryBdd) {
    let detail = cli_entry_bdd.detail.borrow();
    assert_eq!(
        detail.as_str(),
        "first=true second=true",
        "c2826: /model 与 /exit 必须经命令模块解析：{detail}"
    );
}

#[given("存在损坏的 YAML 配置文件")]
fn g_c2826_broken_config(ws: &Workspace, cli_entry_bdd: &CliEntryBdd) {
    ws.init();
    let path = ws.ws("broken-config.yaml");
    std::fs::write(&path, "models:\n  models: [unclosed\n").unwrap();
    cli_entry_bdd.detail.replace(path);
}

#[given("存在零显式模型的配置文件")]
fn g_c2826_zero_model_config(ws: &Workspace, cli_entry_bdd: &CliEntryBdd) {
    ws.init();
    let path = ws.ws("zero-models.yaml");
    std::fs::write(&path, "models:\n  models: {}\n").unwrap();
    cli_entry_bdd.detail.replace(path);
}

#[when("调用 load_app_config 加载该文件")]
fn w_c2826_load_config(ws: &Workspace, cli_entry_bdd: &CliEntryBdd) {
    let path = cli_entry_bdd.detail.borrow().clone();
    assert!(!path.is_empty(), "config path not prepared");
    let input = xylitol::app::core::bootstrap::BootstrapInput {
        config_path: Some(path.into()),
        session: None,
        model: None,
        trust_override: Some(true),
        interactive: false,
        caller: "bdd",
    };
    // 隔离全局配置层：XYLITOL_CONFIG_DIR 指向空目录，避免开发机真实配置注入模型。
    let isolated_global = ws.ws("isolated-global-config");
    std::fs::create_dir_all(&isolated_global).unwrap();
    let env_probe = move |k: &str| -> Option<String> {
        if k == "XYLITOL_CONFIG_DIR" {
            Some(isolated_global.clone())
        } else {
            None
        }
    };
    let cwd = ws.root();
    let result = xylitol::app::core::bootstrap::resolve_assembly_with(
        &input,
        env_probe,
        Some(std::path::Path::new(&cwd)),
    );
    let rendered = match result {
        Ok(_) => "ok".to_string(),
        Err(e) => format!("error: {e}"),
    };
    cli_entry_bdd.ok.set(rendered.starts_with("error"));
    cli_entry_bdd.detail.replace(rendered);
}

#[then("返回硬错误而非警告后继续")]
fn t_c2826_config_hard_fail(cli_entry_bdd: &CliEntryBdd) {
    assert!(
        cli_entry_bdd.ok.get(),
        "c2826: 损坏配置必须硬失败：{}",
        cli_entry_bdd.detail.borrow()
    );
}

#[then("返回硬错误且指向配置而非回退 env 默认模型")]
fn t_c2826_zero_models_hard_fail(cli_entry_bdd: &CliEntryBdd) {
    let detail = cli_entry_bdd.detail.borrow();
    assert!(
        cli_entry_bdd.ok.get(),
        "c2826: 零显式模型必须硬失败：{detail}"
    );
    assert!(
        detail.contains("model") || detail.contains("配置"),
        "c2826: 错误须指向模型配置：{detail}"
    );
}

#[when("产品面读取未选中模型的展示名")]
fn w_c2826_unset_model_display(cli_entry_bdd: &CliEntryBdd) {
    let shown = xylitol::app::core::bootstrap::UNSET_MODEL_DISPLAY.to_string();
    cli_entry_bdd.detail.replace(shown);
}

#[then("得到 NOT-SET 而非厂商默认模型名")]
fn t_c2826_unset_display(cli_entry_bdd: &CliEntryBdd) {
    let shown = cli_entry_bdd.detail.borrow();
    assert_eq!(shown.as_str(), "NOT-SET");
    assert_ne!(shown.as_str(), "gpt-4o");
}

#[when("对接好会话的驱动请求 file_browser 会话树")]
async fn w_c2826_tree_kind_unsupported(agent: &AgentState) {
    use xylitol::embed::XyInProcessDriver;
    use xylitol::protocol::session::SessionTreeKind;
    let (runtime, store) = crate::bdd::helpers::make_agent_with_store(agent);
    let mut driver = XyInProcessDriver::new(runtime, store);
    driver
        .new_session()
        .await
        .expect("c2826: new_session for tree-kind probe");
    let err = driver
        .session_tree(SessionTreeKind::FileBrowser)
        .await
        .expect_err("file_browser kind must be rejected explicitly");
    agent
        .last_result
        .replace(Some(Err(xylitol::XyDriverError::unsupported(
            err.to_string(),
        ))));
    agent.last_op_error.replace(Some(err.to_string()));
}

#[then("返回明确的不支持错误且提及 file_browser")]
fn t_c2826_tree_kind_error(agent: &AgentState) {
    let msg = agent
        .last_op_error
        .borrow()
        .clone()
        .expect("error captured");
    assert!(
        msg.contains("file_browser") && msg.contains("not implemented"),
        "c2826: 未实现 kind 必须明确报错：{msg}"
    );
}
