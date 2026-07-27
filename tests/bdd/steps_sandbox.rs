mod sandbox_bdd {
    use rstest_bdd_macros::{given, then, when};
    use std::sync::Arc;
    use xylitol::infra::permission::{XyPermission, XyPermissionVerdict};

    thread_local! {
        static SANDBOX_ENGINE: std::cell::RefCell<Option<Arc<dyn XyPermission>>> =
            const { std::cell::RefCell::new(None) };
        static LAST_VERDICT: std::cell::RefCell<Option<XyPermissionVerdict>> =
            const { std::cell::RefCell::new(None) };
    }

    use xylitol::infra::config::types::{
        PermissionBackend, PermissionConfig, PermissionFilesystemConfig, PermissionNetworkConfig,
        PermissionProcessConfig,
    };

    fn default_sandbox() -> PermissionConfig {
        PermissionConfig {
            enabled: true,
            backend: PermissionBackend::Glob,
            filesystem: PermissionFilesystemConfig {
                read_allowed: vec!["/project/**".into()],
                write_allowed: vec!["/project/**".into()],
                write_denied: vec!["**/.env".into()],
            },
            network: PermissionNetworkConfig {
                allowed_domains: vec!["github.com".into()],
                denied_domains: vec!["evil.com".into()],
            },
            process: PermissionProcessConfig {
                allowed_paths: vec![],
            },
        }
    }

    #[given("沙箱引擎已初始化")]
    fn sandbox_engine_init() {
        SANDBOX_ENGINE.with(|e| {
            *e.borrow_mut() = Some(xylitol::infra::permission::build_permission(
                &default_sandbox(),
            ));
        });
    }

    #[when("检查写入路径 {path:string}")]
    fn sandbox_check_write(path: String) {
        SANDBOX_ENGINE.with(|e| {
            let engine = e.borrow();
            let verdict = engine.as_ref().unwrap().check_write(&path);
            LAST_VERDICT.with(|v| *v.borrow_mut() = Some(verdict));
        });
    }

    #[when("检查网络域名 {domain:string}")]
    fn sandbox_check_domain(domain: String) {
        SANDBOX_ENGINE.with(|e| {
            let engine = e.borrow();
            let verdict = engine.as_ref().unwrap().check_network(&domain);
            LAST_VERDICT.with(|v| *v.borrow_mut() = Some(verdict));
        });
    }

    #[then("结果应为拒绝")]
    fn sandbox_assert_denied() {
        LAST_VERDICT.with(|v| {
            let verdict = v.borrow();
            assert!(
                !verdict.as_ref().unwrap().is_allowed(),
                "Expected sandbox verdict to be Deny, but got Allow"
            );
        });
    }

    #[then("结果应为允许")]
    fn sandbox_assert_allowed() {
        LAST_VERDICT.with(|v| {
            let guard = v.borrow();
            let verdict = guard.as_ref().unwrap();
            assert!(
                verdict.is_allowed(),
                "Expected sandbox verdict to be Allow, but got {:?}",
                verdict
            );
        });
    }

    #[then("拒绝原因包含 {text:string}")]
    fn sandbox_assert_deny_reason(text: String) {
        LAST_VERDICT.with(|v| {
            let verdict = v.borrow();
            let reason = verdict.as_ref().unwrap().deny_reason().unwrap_or("");
            assert!(
                reason.contains(&text),
                "Expected deny reason to contain '{text}', got '{reason}'"
            );
        });
    }

    #[then("结果应为拒绝且原因含 write_denied")]
    fn sandbox_denied_write_reason() {
        sandbox_assert_denied();
        sandbox_assert_deny_reason("write_denied".into());
    }

    #[then("结果应为拒绝且原因含 denied_domains")]
    fn sandbox_denied_domain_reason() {
        sandbox_assert_denied();
        sandbox_assert_deny_reason("denied_domains".into());
    }
}
