use xylitol_plugin_sdk::XyPermission;
use xylitol_runtime::permission::{
    AllowAllPermission, FilesystemRules, GlobPolicy, NetworkRules, PermissionRules, ProcessRules,
    allow_all_permission, build_permission, policy,
};

fn make_rules() -> PermissionRules {
    PermissionRules {
        enabled: true,
        filesystem: FilesystemRules {
            read_allowed: vec!["/home/user/project/**".into()],
            write_allowed: vec!["/home/user/project/**".into()],
            write_denied: vec!["**/.env".into(), "**/*.pem".into()],
        },
        network: NetworkRules {
            allowed_domains: vec!["github.com".into(), "*.github.com".into()],
            denied_domains: vec!["evil.com".into()],
        },
        process: ProcessRules {
            allowed_paths: vec![],
        },
    }
}

#[test]
fn glob_policy_allows_project_read() {
    let engine = GlobPolicy::new(&make_rules());
    assert!(
        engine
            .check_read("/home/user/project/src/main.rs")
            .is_allowed()
    );
}

#[test]
fn glob_policy_denies_outside_read() {
    let engine = GlobPolicy::new(&make_rules());
    let verdict = engine.check_read("/etc/passwd");
    assert!(!verdict.is_allowed());
    assert!(verdict.deny_reason().unwrap().contains("read_allowed"));
}

#[test]
fn glob_policy_denies_env_write() {
    let engine = GlobPolicy::new(&make_rules());
    let verdict = engine.check_write("/home/user/project/.env");
    assert!(!verdict.is_allowed());
    assert!(verdict.deny_reason().unwrap().contains("write_denied"));
}

#[test]
fn glob_policy_allows_project_write() {
    let engine = GlobPolicy::new(&make_rules());
    assert!(
        engine
            .check_write("/home/user/project/src/lib.rs")
            .is_allowed()
    );
}

#[test]
fn glob_policy_denies_evil_domain() {
    let engine = GlobPolicy::new(&make_rules());
    let verdict = engine.check_network("evil.com");
    assert!(!verdict.is_allowed());
    assert!(verdict.deny_reason().unwrap().contains("denied_domains"));
}

#[test]
fn glob_policy_allows_github_domain() {
    let engine = GlobPolicy::new(&make_rules());
    assert!(engine.check_network("github.com").is_allowed());
    assert!(engine.check_network("api.github.com").is_allowed());
}

#[test]
fn glob_policy_denies_unlisted_domain() {
    let engine = GlobPolicy::new(&make_rules());
    let verdict = engine.check_network("example.com");
    assert!(!verdict.is_allowed());
    assert!(verdict.deny_reason().unwrap().contains("allowed_domains"));
}

#[test]
fn allow_all_allows_everything() {
    let engine = AllowAllPermission;
    assert!(engine.check_read("/etc/shadow").is_allowed());
    assert!(engine.check_write("/etc/passwd").is_allowed());
    assert!(engine.check_network("evil.com").is_allowed());
    assert!(engine.check_process("/bin/rm").is_allowed());
}

#[test]
fn build_permission_disabled_returns_allow_all() {
    let mut rules = make_rules();
    rules.enabled = false;
    let engine = build_permission(&rules);
    assert!(engine.check_read("/etc/passwd").is_allowed());
}

#[test]
fn build_permission_enabled_returns_glob_policy() {
    let engine = build_permission(&make_rules());
    assert!(!engine.check_read("/etc/passwd").is_allowed());
}

#[test]
fn allow_all_helper_matches_type() {
    assert!(
        allow_all_permission()
            .check_read("/etc/passwd")
            .is_allowed()
    );
}

#[test]
fn path_glob_and_domain_matchers() {
    assert!(policy::path_matches_any(
        "/home/user/file.txt",
        &["/home/user/*.txt".into()]
    ));
    assert!(policy::path_matches_any(
        "/project/sub/.env",
        &["**/.env".into()]
    ));
    assert!(!policy::path_matches_any(
        "/etc/passwd",
        &["/home/**".into()]
    ));
    assert!(policy::domain_matches_any(
        "api.github.com",
        &["*.github.com".into()]
    ));
    assert!(!policy::domain_matches_any(
        "github.com",
        &["*.github.com".into()]
    ));
    assert!(policy::domain_matches_any("Evil.COM", &["evil.com".into()]));
}
