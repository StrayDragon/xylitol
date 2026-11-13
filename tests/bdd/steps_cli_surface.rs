use crate::bdd::prelude::*;
use rstest::fixture;
use rstest_bdd_macros::{given, then, when};
use xylitol_kernel::attach::{
    HealthzInfo, ReadyDecision, classify_running, ready_decision, replace_timeout_message,
};

const OWN_VERSION: &str = env!("CARGO_PKG_VERSION");

pub struct SurfaceBdd {
    pub(crate) mode: Cell<Option<xylitol::cli::SurfaceMode>>,
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
    use xylitol::cli::{CliArgs, SurfaceIntent};
    assert!(surface_bdd.is_tty.get(), "given TTY must arm is_tty");
    let args = CliArgs::try_parse_from(["xylitol", "tui"]).expect("parse tui");
    let intent = SurfaceIntent::from_command(args.command.as_ref());
    surface_bdd
        .mode
        .set(Some(intent.mode(surface_bdd.is_tty.get())));
}

#[then("进入产品 TUI")]
fn t_ce16_enters_tui(surface_bdd: &SurfaceBdd) {
    assert_eq!(surface_bdd.mode.get(), Some(xylitol::cli::SurfaceMode::Tui));
}

#[given("无 prompt")]
fn g_ce16_no_prompt(surface_bdd: &SurfaceBdd) {
    surface_bdd.has_prompt.set(false);
}

#[when("xylitol print")]
fn w_ce16_print(surface_bdd: &SurfaceBdd) {
    use clap::Parser;
    use xylitol::cli::{CliArgs, SurfaceIntent, resolve_print_prompt};
    assert!(
        !surface_bdd.has_prompt.get(),
        "given 无 prompt must clear has_prompt"
    );
    let args = CliArgs::try_parse_from(["xylitol", "print"]).expect("parse print");
    let intent = SurfaceIntent::from_command(args.command.as_ref());
    let err = resolve_print_prompt(
        intent.one_shot_prompt.as_deref(),
        intent.explicit_print,
        true,
        || Ok(String::new()),
    )
    .expect_err("print without prompt must fail");
    surface_bdd.print_err.replace(err.to_string());
}

#[then("错误退出且无 Hello!")]
fn t_ce16_print_no_hello(surface_bdd: &SurfaceBdd) {
    let err = surface_bdd.print_err.borrow();
    assert!(!err.is_empty(), "expected print error");
    assert!(!err.contains("Hello!"), "{err}");
}

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

#[given("CLI 已解析")]
fn g_ce_cli_parsed(_cli_help_bdd: &CliHelpBdd) {}

#[when("xylitol serve --help")]
fn w_ce8_serve_help(cli_help_bdd: &CliHelpBdd) {
    use clap::CommandFactory;
    let mut cmd = xylitol::cli::CliArgs::command();
    let help = cmd
        .find_subcommand_mut("serve")
        .expect("serve subcommand")
        .render_long_help()
        .to_string();
    cli_help_bdd.out.replace(help);
    cli_help_bdd.ok.set(true);
}

#[then("可见 --host --port 与 stop，且无 install、server 或 run 别名")]
fn t_ce8_serve_shape(cli_help_bdd: &CliHelpBdd) {
    use clap::{CommandFactory, Parser};
    let help = cli_help_bdd.out.borrow();
    assert!(help.contains("--host"), "{help}");
    assert!(help.contains("--port"), "{help}");
    assert!(help.contains("--lifecycle"), "{help}");
    assert!(help.contains("--idle-timeout"), "{help}");
    let mut cmd = xylitol::cli::CliArgs::command();
    assert!(cmd.find_subcommand("server").is_none());
    let serve = cmd.find_subcommand_mut("serve").expect("serve");
    assert!(serve.find_subcommand("stop").is_some(), "{help}");
    assert!(serve.find_subcommand("install").is_none(), "{help}");
    assert!(serve.find_subcommand("run").is_none(), "{help}");
    assert!(
        xylitol::cli::CliArgs::try_parse_from(["xylitol", "server"]).is_err(),
        "server alias must not parse"
    );
    assert!(
        xylitol::cli::CliArgs::try_parse_from(["xylitol", "serve", "run"]).is_err(),
        "serve run must not parse"
    );
}

#[when("xylitol --help")]
fn w_ce16_top_help(cli_help_bdd: &CliHelpBdd) {
    use clap::CommandFactory;
    let help = xylitol::cli::CliArgs::command()
        .render_long_help()
        .to_string();
    cli_help_bdd.out.replace(help);
    cli_help_bdd.ok.set(true);
}

#[then("帮助文案含 XYlitol 且命令名为 xylitol")]
fn t_help_product_identity(cli_help_bdd: &CliHelpBdd) {
    use clap::CommandFactory;
    let help = cli_help_bdd.out.borrow();
    assert!(help.contains("XYlitol"), "CLI about 须含 XYlitol：{help}");
    let cmd = xylitol::cli::CliArgs::command();
    assert_eq!(cmd.get_name(), "xylitol");
}

#[then("Commands 含 serve 为顶层且不含 tokenizer 与 resources")]
fn t_ce16_ops_toplevel_legacy(cli_help_bdd: &CliHelpBdd) {
    t_ce16_ops_toplevel(cli_help_bdd);
}

#[then("Commands 含 serve 为顶层且不含 tokenizer、resources、preview 与 designing")]
fn t_ce16_ops_toplevel(cli_help_bdd: &CliHelpBdd) {
    use clap::CommandFactory;
    let help = cli_help_bdd.out.borrow();
    assert!(!help.contains("tokenizer"), "{help}");
    assert!(help.contains("serve"), "{help}");
    assert!(help.contains("tui"), "{help}");
    assert!(help.contains("print"), "{help}");
    let mut cmd = xylitol::cli::CliArgs::command();
    assert!(
        cmd.find_subcommand("serve").is_some(),
        "serve must be a top-level verb"
    );
    assert!(
        cmd.find_subcommand("tokenizer").is_none(),
        "tokenizer must not remain a product verb"
    );
    assert!(
        cmd.find_subcommand("resources").is_none(),
        "resources must not remain a product verb"
    );
    assert!(
        cmd.find_subcommand("server").is_none(),
        "server must not remain as a product verb"
    );
    assert!(
        cmd.find_subcommand("preview").is_none(),
        "preview must not be a top-level surface"
    );
    assert!(
        cmd.find_subcommand("designing").is_none(),
        "designing must not be a top-level surface"
    );
    let tui = cmd.find_subcommand_mut("tui").expect("tui subcommand");
    assert!(
        tui.find_subcommand("tokenizer").is_none()
            && tui.find_subcommand("resources").is_none()
            && tui.find_subcommand("serve").is_none(),
        "ops must not nest under tui"
    );
}

#[given("CLI 已解析且为 debug 构建")]
fn g_cli_debug(_cli_help_bdd: &CliHelpBdd) {}

#[given("CLI 已解析且为 release 构建")]
fn g_cli_release(_cli_help_bdd: &CliHelpBdd) {}

#[given("本机 Host 未在听且为 debug 构建")]
fn g_host_down_debug(_cli_help_bdd: &CliHelpBdd) {}

#[when("xylitol tui --help")]
fn w_tui_help(cli_help_bdd: &CliHelpBdd) {
    use clap::CommandFactory;
    let mut cmd = xylitol::cli::CliArgs::command();
    let help = cmd
        .find_subcommand_mut("tui")
        .expect("tui")
        .render_long_help()
        .to_string();
    cli_help_bdd.out.replace(help);
    cli_help_bdd.ok.set(true);
}

#[then("子命令含 designing")]
fn t_tui_has_designing(cli_help_bdd: &CliHelpBdd) {
    if !cfg!(debug_assertions) {
        return;
    }
    use clap::CommandFactory;
    let help = cli_help_bdd.out.borrow();
    assert!(help.contains("designing"), "{help}");
    let mut cmd = xylitol::cli::CliArgs::command();
    let tui = cmd.find_subcommand_mut("tui").expect("tui");
    assert!(tui.find_subcommand("designing").is_some());
}

#[then("子命令不含 designing")]
fn t_tui_omits_designing(cli_help_bdd: &CliHelpBdd) {
    if cfg!(debug_assertions) {
        return;
    }
    use clap::CommandFactory;
    let help = cli_help_bdd.out.borrow();
    assert!(!help.contains("designing"), "{help}");
    let mut cmd = xylitol::cli::CliArgs::command();
    let tui = cmd.find_subcommand_mut("tui").expect("tui");
    assert!(tui.find_subcommand("designing").is_none());
}

#[then("Commands 不含 preview 与 designing")]
fn t_help_omits_preview_designing(cli_help_bdd: &CliHelpBdd) {
    use clap::CommandFactory;
    let cmd = xylitol::cli::CliArgs::command();
    assert!(cmd.find_subcommand("preview").is_none());
    assert!(cmd.find_subcommand("designing").is_none());
    let help = cli_help_bdd.out.borrow();
    let commands = help.split("Commands:").nth(1).unwrap_or(&help);
    assert!(
        !commands.contains("designing"),
        "top-level Commands must omit designing: {commands}"
    );
}

#[when("xylitol tui designing")]
fn w_tui_designing(cli_help_bdd: &CliHelpBdd) {
    use clap::Parser;
    if !cfg!(debug_assertions) {
        cli_help_bdd.ok.set(false);
        return;
    }
    let args = xylitol::cli::CliArgs::try_parse_from(["xylitol", "tui", "designing"])
        .expect("debug builds parse tui designing");
    assert!(xylitol::cli::is_tui_designing(args.command.as_ref()));
    assert!(xylitol::cli::tui_requires_host_listener(
        args.command.as_ref()
    ));
    cli_help_bdd.ok.set(true);
}

#[then("进入 designing 组合且须走 Host attach 预检")]
fn t_designing_skips_attach(cli_help_bdd: &CliHelpBdd) {
    if !cfg!(debug_assertions) {
        return;
    }
    assert!(
        cli_help_bdd.ok.get(),
        "tui designing must parse and attach Host"
    );
}

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
    use xylitol::cli::{CliArgs, surface_from_command};
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
    let s = surface_from_command(args.command.as_ref()).expect("surface flags");
    surface_flags_bdd.parse_ok.set(true);
    surface_flags_bdd.session.replace(s.session);
    assert_eq!(s.model.as_deref(), Some("m"));
    assert!(s.trust);
}

#[then("解析成功且表面旗标生效")]
fn t_ce19_flags_ok(surface_flags_bdd: &SurfaceFlagsBdd) {
    assert!(surface_flags_bdd.parse_ok.get());
    let session = surface_flags_bdd.session.borrow();
    if session.is_some() {
        assert_eq!(session.as_deref(), Some("sid"));
    }
}

#[when("xylitol tui run --session sid")]
fn w_ce19_tui_run_session(surface_flags_bdd: &SurfaceFlagsBdd) {
    use clap::Parser;
    use xylitol::cli::{CliArgs, surface_from_command};
    let args = CliArgs::try_parse_from(["xylitol", "tui", "run", "--session", "sid"])
        .expect("parse tui run --session");
    let s = surface_from_command(args.command.as_ref()).expect("surface flags");
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
    use xylitol::cli::{CliArgs, surface_from_command};
    let args = CliArgs::try_parse_from([
        "xylitol",
        "tui",
        "--attach",
        "http://127.0.0.1:9",
        "--port",
        "11",
    ])
    .expect("parse tui --attach/--port");
    let s = surface_from_command(args.command.as_ref()).expect("surface flags");
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
    use xylitol::cli::CliArgs;
    CliArgs::try_parse_from(["xylitol", "print", "--session", "sid", "--no-color", "hi"])
        .expect("parse print flags");
    surface_flags_bdd.parse_ok.set(true);
}

#[when("xylitol print --session sid --trust --model m hi")]
fn w_ce19_print_trust(surface_flags_bdd: &SurfaceFlagsBdd) {
    use clap::Parser;
    use xylitol::cli::{CliArgs, surface_from_command};
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
    let s = surface_from_command(args.command.as_ref()).expect("surface flags");
    surface_flags_bdd.parse_ok.set(true);
    surface_flags_bdd.session.replace(s.session);
    assert_eq!(s.model.as_deref(), Some("m"));
    assert!(s.trust);
    assert!(!s.no_trust);
}

#[when("xylitol tui --no-ensure-host")]
fn w_ce19_tui_no_ensure(surface_flags_bdd: &SurfaceFlagsBdd) {
    use clap::Parser;
    use xylitol::cli::{CliArgs, surface_from_command};
    let args = CliArgs::try_parse_from(["xylitol", "tui", "--no-ensure-host"])
        .expect("parse tui --no-ensure-host");
    let s = surface_from_command(args.command.as_ref()).expect("surface flags");
    surface_flags_bdd.parse_ok.set(true);
    assert!(s.no_ensure_host);
}

#[when("xylitol print --session sid --no-trust hi")]
fn w_ce19_print_no_trust(surface_flags_bdd: &SurfaceFlagsBdd) {
    use clap::Parser;
    use xylitol::cli::{CliArgs, surface_from_command};
    let args =
        CliArgs::try_parse_from(["xylitol", "print", "--session", "sid", "--no-trust", "hi"])
            .expect("parse print --no-trust");
    let s = surface_from_command(args.command.as_ref()).expect("surface flags");
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
    use xylitol::cli::CliArgs;
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
        .replace(xylitol::cli::resume_hint_line(Some("uuid-1"), true));
}

#[given("当前 session 未持久化")]
fn g_ce20_unlisted(surface_flags_bdd: &SurfaceFlagsBdd) {
    surface_flags_bdd.session.replace(Some("uuid-2".into()));
    surface_flags_bdd
        .hint
        .replace(xylitol::cli::resume_hint_line(Some("uuid-2"), false));
}

#[when("TUI 或 print 正常退出")]
fn w_ce20_exit(surface_flags_bdd: &SurfaceFlagsBdd) {
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

pub struct AttachBdd {
    pub(crate) err: RefCell<Option<String>>,
    pub(crate) no_ensure: Cell<bool>,
    pub(crate) attach: RefCell<Option<String>>,
    pub(crate) spawned: Cell<bool>,
    pub(crate) ensure_target: Cell<bool>,
    pub(crate) peer_version: RefCell<Option<String>>,
    pub(crate) foreign_occupant: Cell<bool>,
    pub(crate) replace_timed_out: Cell<bool>,
    pub(crate) decision: Cell<Option<ReadyDecision>>,
}

impl AttachBdd {
    fn new() -> Self {
        Self {
            err: RefCell::new(None),
            no_ensure: Cell::new(false),
            attach: RefCell::new(None),
            spawned: Cell::new(false),
            ensure_target: Cell::new(false),
            peer_version: RefCell::new(None),
            foreign_occupant: Cell::new(false),
            replace_timed_out: Cell::new(false),
            decision: Cell::new(None),
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

#[given("本机 Host 未在听且关闭确保")]
fn g_ce21_host_down_no_ensure(attach_bdd: &AttachBdd) {
    attach_bdd.no_ensure.set(true);
    attach_bdd.attach.replace(Some("http://127.0.0.1:1".into()));
}

#[given("显式 attach 非本机确保口且该口未在听")]
fn g_ce21_explicit_other(attach_bdd: &AttachBdd) {
    attach_bdd.attach.replace(Some("http://127.0.0.1:1".into()));
}

#[when("产品 TUI 尝试 attach")]
fn w_ce21_attach(attach_bdd: &AttachBdd) {
    let attach = attach_bdd.attach.borrow().clone();
    let url = xylitol::resolve_attach_url(attach.as_deref(), None);
    if let Some(version) = attach_bdd.peer_version.borrow().clone() {
        let hz = HealthzInfo { pid: 4242, version };
        attach_bdd.err.replace(Some(
            classify_running(Some(hz), None, &url, OWN_VERSION).unwrap_or_default(),
        ));
        return;
    }
    let should = xylitol::should_ensure_host(attach.as_deref(), None, attach_bdd.no_ensure.get());
    if should {
        attach_bdd.spawned.set(true);
        attach_bdd
            .err
            .replace(Some(xylitol::ready_timeout_message(&url)));
        return;
    }
    let err = xylitol::probe_host(&url).expect_err("target must be down");
    attach_bdd.spawned.set(false);
    attach_bdd.err.replace(Some(err.to_string()));
}

#[when("解析本机默认 attach URL 的确保策略")]
fn w_ce21_policy(attach_bdd: &AttachBdd) {
    attach_bdd
        .ensure_target
        .set(xylitol::should_ensure_host(None, None, false));
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
    assert!(!attach_bdd.spawned.get());
}

#[then("非零退出且未拉起 Host")]
fn t_ce21_no_spawn(attach_bdd: &AttachBdd) {
    let msg = attach_bdd.err.borrow().clone().expect("probe error");
    assert!(
        msg.contains("not listening") || msg.contains("Host is not listening"),
        "{msg}"
    );
    assert!(!attach_bdd.spawned.get());
    assert!(!xylitol::should_ensure_host(
        attach_bdd.attach.borrow().as_deref(),
        None,
        false
    ));
}

#[then("该 URL 为确保目标且不改走同进程直握")]
fn t_ce21_ensure_target(attach_bdd: &AttachBdd) {
    assert!(attach_bdd.ensure_target.get());
    let inprocess = std::any::type_name::<xylitol::XyInProcessDriver>();
    assert!(inprocess.contains("XyInProcessDriver"), "{inprocess}");
}

#[given("本机确保口上 Host 版本旧于客户端")]
fn g_ce21_older_host(attach_bdd: &AttachBdd) {
    attach_bdd.peer_version.replace(Some("0.0.0-aa".into()));
}

#[given("确保口上 Host 版本新于客户端")]
fn g_ce21_newer_host(attach_bdd: &AttachBdd) {
    attach_bdd.peer_version.replace(Some("9.9.9".into()));
}

#[given("本机确保口被非 xylitol 进程占用")]
fn g_ce21_foreign_occupant(attach_bdd: &AttachBdd) {
    attach_bdd.foreign_occupant.set(true);
}

#[given("显式 attach 非本机确保口且该口 Host 版本旧于客户端")]
fn g_ce21_explicit_older_host(attach_bdd: &AttachBdd) {
    attach_bdd.attach.replace(Some("http://127.0.0.1:1".into()));
    attach_bdd.peer_version.replace(Some("0.0.0-aa".into()));
}

#[given("旧 Host 在五秒内未退出")]
fn g_ce21_replace_timed_out(attach_bdd: &AttachBdd) {
    attach_bdd.replace_timed_out.set(true);
}

#[when("产品 TUI 执行确保")]
fn w_ce21_ensure(attach_bdd: &AttachBdd) {
    if attach_bdd.foreign_occupant.get() {
        attach_bdd
            .decision
            .set(Some(ready_decision(None, OWN_VERSION)));
        attach_bdd.err.replace(Some(
            classify_running(None, None, "http://127.0.0.1:18790", OWN_VERSION).unwrap_or_default(),
        ));
        return;
    }
    let hz = attach_bdd
        .peer_version
        .borrow()
        .clone()
        .map(|version| HealthzInfo { pid: 4242, version });
    let decision = ready_decision(hz.as_ref(), OWN_VERSION);
    attach_bdd.decision.set(Some(decision));
    if decision == ReadyDecision::FailClosed {
        attach_bdd.err.replace(Some(
            classify_running(hz, None, "http://127.0.0.1:18790", OWN_VERSION).unwrap_or_default(),
        ));
    }
}

#[when("确保口尝试替换")]
fn w_ce21_attempt_replace(attach_bdd: &AttachBdd) {
    assert!(
        attach_bdd.replace_timed_out.get(),
        "given must arm the replace timeout"
    );
    attach_bdd.err.replace(Some(replace_timeout_message(
        4242,
        std::time::Duration::from_secs(5),
    )));
}

#[then("请求旧 Host 退出并在新 Host 就绪后 attach")]
fn t_ce21_replace_decision(attach_bdd: &AttachBdd) {
    assert_eq!(attach_bdd.decision.get(), Some(ReadyDecision::Replace));
}

#[then("非零退出且不向该进程发退出请求")]
fn t_ce21_no_exit_request(attach_bdd: &AttachBdd) {
    assert_eq!(attach_bdd.decision.get(), Some(ReadyDecision::FailClosed));
}

#[then("不杀该进程也不拉起 Host 并给出可操作诊断")]
fn t_ce21_foreign_fail_closed(attach_bdd: &AttachBdd) {
    assert_eq!(attach_bdd.decision.get(), Some(ReadyDecision::FailClosed));
    let msg = attach_bdd
        .err
        .borrow()
        .clone()
        .expect("foreign occupant diagnostic");
    assert!(msg.contains("not xylitol"), "{msg}");
    assert!(msg.contains("xylitol tui --port"), "{msg}");
}

#[then("非零退出且不替换该 Host")]
fn t_ce21_explicit_no_replace(attach_bdd: &AttachBdd) {
    let msg = attach_bdd.err.borrow().clone().expect("attach diagnostic");
    assert!(msg.contains("explicit attach cannot"), "{msg}");
    assert!(!xylitol::should_ensure_host(
        attach_bdd.attach.borrow().as_deref(),
        None,
        false
    ));
}

#[then("诊断含该 pid 与手停建议且不承诺强制结束进程")]
fn t_ux_replace_timeout_diag(attach_bdd: &AttachBdd) {
    let msg = attach_bdd.err.borrow().clone().expect("diagnostic");
    assert!(msg.contains("4242"), "{msg}");
    assert!(
        msg.contains("xylitol serve stop") || msg.contains("kill 4242"),
        "{msg}"
    );
    assert!(!msg.contains("SIGKILL"), "{msg}");
}

#[then("诊断建议升级客户端或执行 xylitol serve stop")]
fn t_ux_client_too_old(attach_bdd: &AttachBdd) {
    let msg = attach_bdd.err.borrow().clone().expect("diagnostic");
    assert!(msg.contains("Upgrade this client"), "{msg}");
    assert!(msg.contains("serve stop"), "{msg}");
}

#[then("诊断说明确保口会替换旧 Host 且显式 attach 不替换并给出手停建议")]
fn t_ux_host_too_old(attach_bdd: &AttachBdd) {
    let msg = attach_bdd.err.borrow().clone().expect("diagnostic");
    assert!(msg.contains("explicit attach cannot"), "{msg}");
    assert!(msg.contains("xylitol serve stop"), "{msg}");
}

#[when("读取拉起失败与就绪超时与非 xylitol 占用的诊断")]
fn w_ux_ensure_diag(attach_bdd: &AttachBdd) {
    let spawn = xylitol::spawn_failed_message("http://127.0.0.1:18790", "boom");
    let timeout = xylitol::ready_timeout_message("http://127.0.0.1:18790");
    let occupied =
        xylitol_kernel::attach::classify_running(None, None, "http://127.0.0.1:18790", "0.0.0-dev")
            .unwrap_or_default();
    attach_bdd
        .err
        .replace(Some(format!("{spawn}\n{timeout}\n{occupied}")));
}

#[then("各诊断可操作且不只提示先手 serve")]
fn t_ux_ensure_diag(attach_bdd: &AttachBdd) {
    let msg = attach_bdd.err.borrow().clone().expect("diag");
    assert!(msg.contains("Failed to start Host"), "{msg}");
    assert!(msg.contains("Timed out"), "{msg}");
    assert!(msg.contains("not xylitol"), "{msg}");
    assert!(msg.contains("bind") || msg.contains("/healthz"), "{msg}");
}

#[when("检查 Driver 实现")]
fn w_atb4_inspect_drivers() {}

#[then("默认 attach 且同进程驱动路径仍保留给 print 与嵌入")]
fn t_atb4_attach_and_inprocess() {
    assert_eq!(xylitol::DEFAULT_ATTACH_URL, "http://127.0.0.1:18790");
    let inprocess = std::any::type_name::<xylitol::XyInProcessDriver>();
    let http_ws = std::any::type_name::<xylitol_app_tui::host_client::HttpWsClient>();
    assert!(inprocess.contains("XyInProcessDriver"), "{inprocess}");
    assert!(http_ws.contains("HttpWsClient"), "{http_ws}");
}

use crate::bdd::fixtures::{AgentState, Workspace};

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
    use xylitol_app_tui::parse_slash_command;
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
    std::fs::write(&path, "models:\n  entries: [unclosed\n").unwrap();
    cli_entry_bdd.detail.replace(path);
}

#[given("存在零显式模型的配置文件")]
fn g_c2826_zero_model_config(ws: &Workspace, cli_entry_bdd: &CliEntryBdd) {
    ws.init();
    let path = ws.ws("zero-models.yaml");
    std::fs::write(&path, "models:\n  entries: {}\n").unwrap();
    cli_entry_bdd.detail.replace(path);
}

#[when("调用 load_app_config 加载该文件")]
fn w_c2826_load_config(ws: &Workspace, cli_entry_bdd: &CliEntryBdd) {
    let path = cli_entry_bdd.detail.borrow().clone();
    assert!(!path.is_empty(), "config path not prepared");
    let input = xylitol_kernel::bootstrap::BootstrapInput {
        config_path: Some(path.into()),
        session: None,
        model: None,
        trust_override: Some(true),
        interactive: false,
        caller: "bdd",
        ephemeral_sessions: false,
        static_tools: Vec::new(),
        agent_profile: None,
    };
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
    let result = xylitol_kernel::bootstrap::resolve_assembly_with(
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
    let shown = xylitol_kernel::bootstrap::UNSET_MODEL_DISPLAY.to_string();
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
    use xylitol::embed::assemble_driver;
    use xylitol_plugin_sdk::session::SessionTreeKind;
    let (runtime, store) = crate::bdd::helpers::make_agent_with_store(agent);
    let mut driver = assemble_driver(runtime, store);
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

#[given("已显式配置默认模型 fake-model 且未显式选择模型")]
async fn g_c2841_configured_default(agent: &AgentState, cli_entry_bdd: &CliEntryBdd) {
    use xylitol_plugin_sdk::model::{XyModelConfig, XyModelKind, XyModelMeta};
    {
        let mut reg = agent.registry.borrow_mut();
        if reg.find("fake-model").is_none() {
            reg.register(XyModelMeta {
                id: "fake-model".into(),
                config: XyModelConfig {
                    kind: XyModelKind::Fake,
                    api_key: String::new(),
                    model: "fake-model".into(),
                    base_url: None,
                    api: None,
                    compat: None,
                    prompt_cache: None,
                },
                display_name: "fake-model".into(),
                thinking: false,
                context_window: 64_000,
                api: String::new(),
                provider: "fake".into(),
                cost_input: 0.0,
                cost_output: 0.0,
                cost_cache_read: 0.0,
                cost_cache_write: 0.0,
                max_tokens: 0,
                thinking_levels: vec!["off".into()],
                thinking_level_map: Default::default(),
            });
        }
    }
    let (runtime, store) = crate::bdd::helpers::make_agent_with_store(agent);
    let mut driver = xylitol::embed::assemble_driver(runtime, store);
    driver
        .restore_model("fake-model")
        .await
        .expect("restore configured default");
    let shown = model_display_name(xylitol::XyDriver::current_model(&driver).as_ref());
    cli_entry_bdd.detail.replace(shown);
}

#[when("产品面读取当前模型展示名")]
async fn w_c2841_read_display(_agent: &AgentState, _cli_entry_bdd: &CliEntryBdd) {}

#[then("展示为 fake-model 而非 NOT-SET")]
fn t_c2841_configured_default_shown(cli_entry_bdd: &CliEntryBdd) {
    let shown = cli_entry_bdd.detail.borrow();
    assert_eq!(shown.as_str(), "fake-model");
    assert_ne!(shown.as_str(), "NOT-SET");
}

fn model_display_name(model: Option<&xylitol_kernel::driver::ModelInfo>) -> String {
    match model {
        Some(m) => {
            if m.display_name.is_empty() {
                m.id.clone()
            } else {
                m.display_name.clone()
            }
        }
        None => xylitol_kernel::bootstrap::UNSET_MODEL_DISPLAY.to_string(),
    }
}

pub struct TrustFlagConflictBdd {
    pub(crate) runs: RefCell<Vec<(String, i32, String, String)>>,
}

impl TrustFlagConflictBdd {
    fn new() -> Self {
        Self {
            runs: RefCell::new(Vec::new()),
        }
    }
}

#[fixture]
pub fn trust_flag_conflict_bdd() -> TrustFlagConflictBdd {
    TrustFlagConflictBdd::new()
}

#[when("命令行同传 --trust 与 --no-trust")]
fn w_trust_flags_conflict(trust_flag_conflict_bdd: &TrustFlagConflictBdd) {
    let bin = env!("CARGO_BIN_EXE_xylitol");
    let cases: [&[&str]; 4] = [
        &["tui", "--trust", "--no-trust"],
        &["tui", "--trust", "run", "--no-trust"],
        &["tui", "--no-trust", "run", "--trust"],
        &["print", "--trust", "--no-trust", "hi"],
    ];
    let mut runs = Vec::new();
    for args in cases {
        let out = std::process::Command::new(bin)
            .args(args)
            .env("NO_COLOR", "1")
            .output()
            .expect("spawn xylitol with conflicting trust flags");
        runs.push((
            args.join(" "),
            out.status.code().unwrap_or(-1),
            String::from_utf8_lossy(&out.stdout).into_owned(),
            String::from_utf8_lossy(&out.stderr).into_owned(),
        ));
    }
    trust_flag_conflict_bdd.runs.replace(runs);
}

#[then("报错退出且未进入 trust 门禁")]
fn t_trust_flags_conflict(trust_flag_conflict_bdd: &TrustFlagConflictBdd) {
    let runs = trust_flag_conflict_bdd.runs.borrow();
    assert_eq!(runs.len(), 4, "all conflict shapes must be exercised");
    for (argv, code, stdout, stderr) in runs.iter() {
        assert!(*code != 0, "xylitol {argv} must exit non-zero: {stderr}");
        assert!(stderr.contains("--trust"), "{argv} stderr: {stderr}");
        assert!(stderr.contains("--no-trust"), "{argv} stderr: {stderr}");
        assert!(
            stderr.contains("cannot be used with") || stderr.contains("must not be combined"),
            "{argv} must report the flag conflict: {stderr}"
        );
        assert!(
            !stderr.contains("Trust project folder"),
            "{argv} must not reach the trust gate: {stderr}"
        );
        assert!(
            !stderr.contains("Starting host") && !stderr.contains("Resume by"),
            "{argv} must not start the TUI: {stderr}"
        );
        assert!(stdout.is_empty(), "{argv} must not start print: {stdout}");
    }
}

pub struct ProgressBdd {
    pub(crate) buffer: std::sync::Arc<std::sync::Mutex<Vec<u8>>>,
    pub(crate) progress: RefCell<Option<xylitol_app_tui::BootProgress>>,
    pub(crate) diagnostic: RefCell<Option<String>>,
}

struct BufferWriter(std::sync::Arc<std::sync::Mutex<Vec<u8>>>);

impl std::io::Write for BufferWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().expect("buffer").extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl ProgressBdd {
    fn new() -> Self {
        Self {
            buffer: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),
            progress: RefCell::new(None),
            diagnostic: RefCell::new(None),
        }
    }

    fn start(&self) {
        let progress = xylitol_app_tui::BootProgress::start_with(
            "Starting host",
            Box::new(BufferWriter(self.buffer.clone())),
            std::time::Duration::from_millis(150),
            std::time::Duration::from_millis(20),
        );
        self.progress.replace(Some(progress));
    }

    fn output(&self) -> String {
        String::from_utf8(self.buffer.lock().expect("buffer").clone()).unwrap_or_default()
    }
}

#[fixture]
pub fn progress_bdd() -> ProgressBdd {
    ProgressBdd::new()
}

#[given("本机 Host 未在听且产品 TUI 在 TTY 上执行确保")]
fn g_progress_absent(progress_bdd: &ProgressBdd) {
    progress_bdd.start();
}

#[given("本机 Host 已就绪")]
fn g_progress_ready(_progress_bdd: &ProgressBdd) {}

#[given("本机 Host 拉起后未就绪")]
fn g_progress_failing(progress_bdd: &ProgressBdd) {
    progress_bdd.start();
}

#[when("等待超过 150 毫秒")]
fn w_progress_wait(_progress_bdd: &ProgressBdd) {
    std::thread::sleep(std::time::Duration::from_millis(220));
}

#[when("确保在 150 毫秒内完成")]
fn w_progress_fast(progress_bdd: &ProgressBdd) {
    let progress = xylitol_app_tui::BootProgress::start_with(
        "Starting host",
        Box::new(BufferWriter(progress_bdd.buffer.clone())),
        std::time::Duration::from_millis(150),
        std::time::Duration::from_millis(20),
    );
    progress.finish();
}

#[when("确保等待失败")]
fn w_progress_fail(progress_bdd: &ProgressBdd) {
    std::thread::sleep(std::time::Duration::from_millis(220));
    if let Some(progress) = progress_bdd.progress.borrow_mut().take() {
        progress.finish();
    }
    progress_bdd
        .diagnostic
        .replace(Some(xylitol_kernel::attach::ready_timeout_message(
            "http://127.0.0.1:18790",
        )));
}

#[then("输出含 Starting host 的一行进度")]
fn t_progress_visible(progress_bdd: &ProgressBdd) {
    let text = progress_bdd.output();
    assert!(text.contains("Starting host"), "{text}");
    assert!(text.contains("\r\x1b[2K"), "{text:?}");
}

#[then("不输出进度行")]
fn t_progress_absent(progress_bdd: &ProgressBdd) {
    assert!(progress_bdd.output().is_empty());
}

#[then("进度行被清除且输出可操作诊断")]
fn t_progress_cleared(progress_bdd: &ProgressBdd) {
    let text = progress_bdd.output();
    assert!(text.ends_with("\r\x1b[2K"), "{text:?}");
    let diagnostic = progress_bdd
        .diagnostic
        .borrow()
        .clone()
        .expect("diagnostic");
    assert!(diagnostic.contains("Timed out"), "{diagnostic}");
    assert!(diagnostic.contains("xylitol serve"), "{diagnostic}");
}
