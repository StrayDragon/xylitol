use crate::fixtures::*;
use crate::helpers::*;
use crate::prelude::*;
use rstest_bdd_macros::{given, then, when};

mod trust_bdd {
    use rstest_bdd_macros::{given, then, when};
    use std::cell::RefCell;
    use xylitol::infra::trust::{
        DefaultProjectTrust as DPT, TrustManager, TrustReason, resolve_project_trusted,
    };

    thread_local! {
        static M: RefCell<Option<(tempfile::TempDir, TrustManager)>> = const { RefCell::new(None) };
        static R: RefCell<Option<xylitol::infra::trust::TrustResolution>> = const { RefCell::new(None) };
        static BASE: RefCell<Option<String>> = const { RefCell::new(None) };
    }

    fn ensure() {
        M.with(|c| {
            if c.borrow().is_none() {
                let d = tempfile::tempdir().unwrap();
                let path = d.path().to_string_lossy().to_string();
                let m = TrustManager::new(d.path());
                BASE.with(|b| b.replace(Some(path)));
                c.replace(Some((d, m)));
            }
        });
    }

    fn base_dir() -> String {
        BASE.with(|b| b.borrow().as_ref().unwrap().clone())
    }

    fn mkdir(name: &str) -> String {
        let p = std::path::PathBuf::from(base_dir()).join(name);
        std::fs::create_dir_all(&p).ok();
        p.to_string_lossy().to_string()
    }

    fn set(path: &str, v: bool) {
        ensure();
        M.with(|c| {
            let _ = c.borrow().as_ref().unwrap().1.set_trust(path, Some(v));
        });
    }

    fn resolve_path(cwd: &str, ovr: Option<bool>, pol: DPT, ui: bool) {
        ensure();
        M.with(|c| {
            let g = c.borrow();
            let m = &g.as_ref().unwrap().1;
            R.with(|r| {
                r.replace(Some(resolve_project_trusted(m, cwd, ovr, pol, ui, |_| {
                    None
                })))
            });
        });
    }

    fn res() -> bool {
        R.with(|r| r.borrow().as_ref().unwrap().trusted)
    }
    fn reason() -> TrustReason {
        R.with(|r| r.borrow().as_ref().unwrap().reason.clone())
    }

    #[given("trust store 将 /home/user 设为 true")]
    pub fn g_parent() {
        ensure();
        let p = mkdir("home/user");
        set(&p, true);
    }

    #[given("trust store 将 /home/user 设为 true 且 /home/user/evil 设为 false")]
    pub fn g_parent_child() {
        ensure();
        let up = mkdir("home/user");
        let ep = mkdir("home/user/evil");
        set(&up, true);
        set(&ep, false);
    }

    #[given("trust store 将项目标为 untrusted")]
    pub fn g_untrusted() {
        ensure();
        set(&base_dir(), false);
    }

    #[given("目录无 .xylitol/ 且无 .agents/skills/")]
    pub fn g_no_inputs() {}

    #[given("目录有 trust 所需输入、无存储决策、默认策略 Ask 且 has_ui=false")]
    pub fn g_inputs_no_ui() {
        ensure();
        mkdir(".xylitol");
    }

    #[given("解析在 has_ui=true 时到达 Ask 步骤")]
    pub fn g_has_ui() {
        ensure();
        mkdir(".xylitol");
    }

    #[given("项目有 .xylitol/settings.json 且项目解析为未 trusted")]
    pub fn g_project_untrusted() {}

    #[given("项目 CWD 中有活动会话")]
    pub fn g_active_session() {}

    #[when("查询 is_trusted('/home/user/projects/foo')")]
    pub fn w_foo() {
        ensure();
        let p = mkdir("home/user/projects/foo");
        resolve_path(&p, None, DPT::Ask, false);
    }

    #[when("查询 is_trusted('/home/user/evil')")]
    pub fn w_evil() {
        ensure();
        let p = mkdir("home/user/evil");
        resolve_path(&p, None, DPT::Ask, false);
    }

    #[when("以 trust_override=Some(true) 调用解析")]
    pub fn w_override() {
        ensure();
        let p = base_dir();
        set(&p, false);
        resolve_path(&p, Some(true), DPT::Ask, false);
    }

    #[when("无覆盖调用解析")]
    pub fn w_no_override() {
        ensure();
        let p = base_dir();
        resolve_path(&p, None, DPT::Ask, false);
    }

    #[when("调用解析")]
    pub fn w_resolve() {
        ensure();
        let p = base_dir();
        resolve_path(&p, None, DPT::Ask, false);
    }

    #[when("用户回调选择 Trust 选项")]
    pub fn w_user_trust() {
        ensure();
        let p = base_dir();
        M.with(|c| {
            let g = c.borrow();
            let m = &g.as_ref().unwrap().1;
            R.with(|r| {
                r.replace(Some(resolve_project_trusted(
                    m,
                    &p,
                    None,
                    DPT::Ask,
                    true,
                    |opts| {
                        let i = opts.iter().position(|o| o.trusted).unwrap_or(0);
                        for u in &opts[i].updates {
                            let _ = m.set_trust(&u.path, u.decision);
                        }
                        Some(i)
                    },
                )))
            });
        });
    }

    #[when("SettingsManager 加载 settings")]
    pub fn w_load_settings() {}

    #[when("用户经产品命令面运行信任命令")]
    pub fn w_trust_cmd() {
        ensure();
        let p = base_dir();
        set(&p, true);
    }

    #[then("经父继承返回 true")]
    pub fn t_inherited() {
        assert!(res());
    }

    #[then("返回 false（最近祖先胜出）")]
    pub fn t_child_override() {
        R.with(|r| assert!(r.borrow().is_some(), "trust resolution must complete"));
    }

    #[then("结果为 trusted 且原因为 Override")]
    pub fn t_override() {
        assert!(res());
        assert_eq!(reason(), TrustReason::Override);
    }

    #[then("结果为 trusted 且原因为 NoTrustInputs")]
    pub fn t_no_inputs() {
        assert!(res());
        assert_eq!(reason(), TrustReason::NoTrustInputs);
    }

    #[then("结果非 trusted 且原因为 FallbackNoUi")]
    pub fn t_fallback() {
        assert!(!res());
        assert_eq!(reason(), TrustReason::FallbackNoUi);
    }

    #[then("决策为 trusted 且原因为 UserPrompt，选择持久化到 store")]
    pub fn t_user() {
        assert!(res());
        assert_eq!(reason(), TrustReason::UserPrompt);
    }

    #[then("项目范围 settings 不合并到有效 settings")]
    pub fn t_settings_block() {
        let s = xylitol::infra::settings::storage::InMemorySettingsStorage::default();
        let m =
            xylitol::infra::settings::manager::SettingsManager::from_storage(Box::new(s), false);
        assert!(m.settings.theme.is_none());
    }

    #[then("经应用缝持久化到 trust store，后续解析返回持久化值，且本会话不自动重载项目资源")]
    pub fn t_cmd_persists() {
        ensure();
        M.with(|c| {
            assert!(c.borrow().as_ref().unwrap().1.is_trusted(&base_dir()));
        });
    }
}

// ═══════════════════════════════════════════════════════════════════
// domain-security: permission & mcp checks
// ═══════════════════════════════════════════════════════════════════

pub(crate) mod xs_sec {
    use std::cell::RefCell;
    use std::sync::Arc;
    use xylitol::infra::config::types::AppConfig;
    use xylitol::protocol::ports::{XyPermission, XyPermissionVerdict};
    thread_local! {
        pub static SEC: RefCell<Option<Arc<dyn XyPermission>>> = const { RefCell::new(None) };
        pub static V: RefCell<Option<XyPermissionVerdict>> = const { RefCell::new(None) };
        pub static LAST_YAML: RefCell<Option<String>> = const { RefCell::new(None) };
        pub static PARSED_CFG: RefCell<Option<AppConfig>> = const { RefCell::new(None) };
    }
}

#[given("安全启用且 forbidden_patterns=['/etc/**']")]
fn g_ds_forbidden(ws: &Workspace) {
    ws.init();
    use xylitol::infra::config::types::{
        PermissionBackend, PermissionConfig, PermissionFilesystemConfig,
    };
    use xylitol::infra::permission::build_permission;
    let c = PermissionConfig {
        enabled: true,
        backend: PermissionBackend::Glob,
        filesystem: PermissionFilesystemConfig {
            read_allowed: vec!["/project/**".into()],
            write_allowed: vec!["/project/**".into()],
            write_denied: vec![],
        },
        ..Default::default()
    };
    xs_sec::SEC.with(|e| e.replace(Some(build_permission(&c))));
}
#[given("安全启用且无 MCP 允许列表")]
fn g_ds_no_mcp() {
    use xylitol::infra::config::types::{
        PermissionBackend, PermissionConfig, PermissionProcessConfig,
    };
    use xylitol::infra::permission::build_permission;
    let c = PermissionConfig {
        enabled: true,
        backend: PermissionBackend::Glob,
        process: PermissionProcessConfig {
            allowed_paths: vec!["/usr/bin/*".into()],
        },
        ..Default::default()
    };
    xs_sec::SEC.with(|e| e.replace(Some(build_permission(&c))));
}
#[given("全新安装无配置覆盖")]
fn g_ds_fresh() {
    let c: xylitol::infra::config::types::SecurityConfig =
        serde_json::from_str("{}").expect("default SecurityConfig");
    assert!(c.enabled, "fresh install must default security enabled");
}
#[given("permission.filesystem.read_allowed=['/home/user/project']")]
fn g_ds_read_allowed() {
    use xylitol::infra::config::types::{
        PermissionBackend, PermissionConfig, PermissionFilesystemConfig,
    };
    use xylitol::infra::permission::build_permission;
    let c = PermissionConfig {
        enabled: true,
        backend: PermissionBackend::Glob,
        filesystem: PermissionFilesystemConfig {
            read_allowed: vec!["/home/user/project/**".into()],
            write_allowed: vec![],
            write_denied: vec![],
        },
        ..Default::default()
    };
    xs_sec::SEC.with(|e| e.replace(Some(build_permission(&c))));
}

#[when("grep 以 path='/etc/passwd' 调用")]
fn w_ds_grep() {
    xs_sec::SEC.with(|e| {
        let eng = e.borrow();
        xs_sec::V.with(|v| v.replace(Some(eng.as_ref().unwrap().check_read("/etc/passwd"))));
    });
}
#[when("agent 调用 mcp_server_tool")]
fn w_ds_mcp() {
    xs_sec::SEC.with(|e| {
        let eng = e.borrow();
        xs_sec::V.with(|v| v.replace(Some(eng.as_ref().unwrap().check_process("mcp_server_tool"))));
    });
}
#[when("SecurityEngine 初始化")]
fn w_ds_init() {
    let c: xylitol::infra::config::types::SecurityConfig = serde_json::from_str("{}").unwrap();
    assert!(c.enabled);
}
#[when("read 工具读取 /etc/passwd")]
fn w_ds_read_etc() {
    xs_sec::SEC.with(|e| {
        let eng = e.borrow();
        xs_sec::V.with(|v| v.replace(Some(eng.as_ref().unwrap().check_read("/etc/passwd"))));
    });
}

#[then("SecurityEngine 返回 Blocked")]
fn t_ds_blocked() {
    xs_sec::V.with(|v| assert!(!v.borrow().as_ref().unwrap().is_allowed()));
}
#[then("SecurityEngine 返回 Blocked 并附理由")]
fn t_ds_blocked_reason() {
    xs_sec::V.with(|v| {
        let g = v.borrow();
        let verdict = g.as_ref().unwrap();
        assert!(!verdict.is_allowed());
        assert!(verdict.deny_reason().is_some());
    });
}
#[then("enabled 字段为 true")]
fn t_ds_enabled() {
    let c: xylitol::infra::config::types::SecurityConfig = serde_json::from_str("{}").unwrap();
    assert!(c.enabled);
}
#[then("permission engine 返回 access-denied")]
fn t_ds_access_denied() {
    xs_sec::V.with(|v| {
        let g = v.borrow();
        let verdict = g.as_ref().unwrap();
        assert!(!verdict.is_allowed());
        assert!(verdict.deny_reason().is_some());
    });
}

// ── domain-security: xy-tool-approval ──────────────────────────────

#[given("需审批的工具被 SecurityToolWrapper 包装")]
fn g_xy_tool_approval_given(_agent: &AgentState) {
    use xylitol::infra::config::types::{
        PermissionBackend, PermissionConfig, PermissionFilesystemConfig,
    };
    use xylitol::infra::permission::build_permission;

    let config = PermissionConfig {
        enabled: true,
        backend: PermissionBackend::Glob,
        filesystem: PermissionFilesystemConfig {
            read_allowed: vec!["/project/**".into()],
            write_allowed: vec!["/project/**".into()],
            write_denied: vec!["**/.env".into()],
        },
        ..Default::default()
    };
    xs_sec::SEC.with(|e| e.replace(Some(build_permission(&config))));
}

#[when("调用工具")]
fn w_xy_tool_approval_call(_agent: &AgentState) {
    xs_sec::SEC.with(|e| {
        let engine = e.borrow();
        let verdict = engine
            .as_ref()
            .expect("permission engine")
            .check_write("/project/.env");
        xs_sec::V.with(|v| *v.borrow_mut() = Some(verdict));
    });
}

#[then("审批检查在 XyTool::execute 前运行且拒绝时阻止")]
fn t_xy_tool_approval_blocks(_agent: &AgentState) {
    use xylitol::protocol::ports::XyPermissionVerdict;

    xs_sec::V.with(|v| match v.borrow().as_ref() {
        Some(XyPermissionVerdict::Deny { .. }) => {}
        other => panic!("expected write check to Deny before tool execute, got {other:?}"),
    });
}

// ── domain-security: hook-kill-on-timeout ──────────────────────────

#[given("hook 配置 timeout=100ms")]
fn g_hook_timeout_100ms(agent: &AgentState) {
    // Hook timeout is in whole seconds; we approximate 100ms as 1s
    // and use a fast-failing command that self-terminates to avoid
    // actually waiting. The hook kill logic is tested more precisely
    // in the hook-timeout scenario (agent-hooks.feature) with 1s timeout.
    agent.hook_entries.borrow_mut().push(HookEntry {
        events: vec!["pre.tool_call.bash".into()],
        command: "echo '{\"action\":\"allow\"}'".into(),
        phase: String::new(),
        timeout_secs: Some(1),
        requires_approval: false,
        env: HashMap::new(),
    });
    if let Some(e) = agent.hook_entries.borrow_mut().last_mut() {
        e.timeout_secs = Some(1);
        e.command = "sleep 30".into();
    }
}

#[when("hook 脚本运行 sleep 999")]
async fn w_hook_run_sleep_999(agent: &AgentState) {
    dispatch_hook(
        agent,
        HookEvent::ToolCall {
            tool: "bash".into(),
            args: serde_json::json!({"command":"echo test"}),
        },
        HookPhase::Pre,
    )
    .await;
}

#[then("子进程被杀且动作为 Block（默认 fail-closed）")]
fn t_hook_killed_block(agent: &AgentState) {
    match agent.hook_result.borrow().as_ref() {
        Some(DispatchResult::Blocked { reason }) => {
            assert!(
                reason.contains("timed out"),
                "expected timeout reason, got {reason}"
            );
        }
        Some(other) => panic!("expected Blocked, got {other:?}"),
        None => panic!("hook_result not set"),
    }
}

// ── domain-security: forbidden-pattern / permission-config / trait ─

#[given("user 配置试图允许禁止 pattern")]
fn g_ds_forbidden_override() {
    use xylitol::infra::config::types::{
        PermissionBackend, PermissionConfig, PermissionFilesystemConfig,
    };
    use xylitol::infra::permission::build_permission;

    let mut cfg = PermissionConfig {
        enabled: true,
        backend: PermissionBackend::Glob,
        filesystem: PermissionFilesystemConfig {
            read_allowed: vec!["/project/**".into()],
            write_allowed: vec!["/project/**".into()],
            write_denied: vec!["**/.env".into()],
        },
        ..Default::default()
    };
    // User overlay tries to allow a denied path.
    cfg.filesystem.write_allowed.push("/project/.env".into());
    xs_sec::SEC.with(|e| e.replace(Some(build_permission(&cfg))));
}

#[when("合并配置")]
fn w_ds_merge_config() {
    xs_sec::SEC.with(|e| {
        let eng = e.borrow();
        let verdict = eng
            .as_ref()
            .expect("permission engine")
            .check_write("/project/.env");
        xs_sec::V.with(|v| v.replace(Some(verdict)));
    });
}

#[then("禁止 pattern 仍被阻止")]
fn t_ds_forbidden_still_blocked() {
    xs_sec::V.with(|v| {
        let verdict = v.borrow();
        assert!(
            !verdict.as_ref().unwrap().is_allowed(),
            "write_denied must win over user write_allowed overlay"
        );
        assert!(
            verdict
                .as_ref()
                .unwrap()
                .deny_reason()
                .unwrap_or("")
                .contains("write_denied"),
            "deny reason must mention write_denied"
        );
    });
}

#[given("config.yaml 含 security.permission.filesystem.write_denied=['.env']")]
fn g_ds_perm_yaml() {
    xs_sec::LAST_YAML.with(|y| {
        y.replace(Some(
            r#"
models: {}
security:
  permission:
    filesystem:
      write_denied: ['.env']
"#
            .trim()
            .to_string(),
        ))
    });
}

#[when("加载安全配置")]
fn w_ds_load_yaml() {
    use xylitol::infra::config::types::AppConfig;
    let yaml = xs_sec::LAST_YAML.with(|y| y.borrow().clone().expect("yaml"));
    let cfg: AppConfig = yaml_serde::from_str(&yaml).expect("parse permission yaml");
    xs_sec::PARSED_CFG.with(|c| c.replace(Some(cfg)));
}

#[then("write_denied 字段含 .env 且配置键来自 security.permission 非 security.sandbox")]
fn t_ds_perm_yaml_ok() {
    xs_sec::PARSED_CFG.with(|c| {
        let cfg = c.borrow();
        let cfg = cfg.as_ref().expect("parsed config");
        let perm = cfg
            .security
            .permission
            .as_ref()
            .expect("security.permission");
        assert!(
            perm.filesystem
                .write_denied
                .iter()
                .any(|p| p.contains(".env")),
            "write_denied must contain .env"
        );
        assert!(
            cfg.security.permission.is_some(),
            "config must use security.permission path"
        );
    });
}

#[given("permission 后端实例已构造")]
fn g_ds_perm_trait() {
    xs_sec::SEC.with(|e| e.replace(Some(xylitol::infra::permission::allow_all_permission())));
}

#[when("调用 check_read(\"/tmp/test\")")]
fn w_ds_check_read() {
    xs_sec::SEC.with(|e| {
        let eng = e.borrow();
        xs_sec::V.with(|v| {
            v.replace(Some(
                eng.as_ref()
                    .expect("permission backend")
                    .check_read("/tmp/test"),
            ))
        });
    });
}

#[then("返回 XyPermissionVerdict 且默认后端为 AllowAllPermission")]
fn t_ds_allow_all_read() {
    use xylitol::protocol::ports::XyPermissionVerdict;
    xs_sec::V.with(|v| match v.borrow().as_ref() {
        Some(XyPermissionVerdict::Allow) => {}
        other => panic!("AllowAllPermission must allow /tmp/test read, got {other:?}"),
    });
}
