use super::*;

#[tokio::test]

async fn persist_project_trust_writes_store_under_agent_dir() {
    let home = tempfile::tempdir().unwrap();
    let cwd = home.path().join("proj");
    let agent_dir = home.path().join(".xylitol");
    std::fs::create_dir_all(cwd.join(".xylitol")).unwrap();
    let store = Arc::new(SessionManager::new(home.path().join("sessions")));
    let (mut driver, _obs) = build_test_driver(store).await;
    driver.enable_reload_state(cwd.clone(), agent_dir.clone(), false, Vec::new());
    let report = driver
        .persist_project_trust(ProjectTrustMode::TrustCwd)
        .expect("persist");
    assert!(report.trusted);
    assert!(report.message.contains("/reload") || report.message.contains("restart"));
    let mgr = crate::infra::trust::TrustManager::new(agent_dir);
    assert!(mgr.is_trusted(&cwd.display().to_string()));
}
