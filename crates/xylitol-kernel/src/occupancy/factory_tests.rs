use super::*;

#[tokio::test]
async fn occupancy_driver_ensure_tool_table_frozen_freezes_tools() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store: Arc<dyn XySessionStore> = Arc::new(
        xylitol_official_plugin_session::LogSessionStore::new(dir.path().join("sessions")),
    );
    let ports = build_baseline_with_store(crate::composition::BuildAgentOptions::default(), store)
        .expect("baseline");
    let factory = ProductOccupancyFactory::new(
        ports,
        dir.path().to_path_buf(),
        dir.path().to_path_buf(),
        true,
        Vec::new(),
        None,
    );

    let mut engine = factory.new_driver_at(dir.path());
    engine.ensure_tool_table_frozen().await;

    assert!(
        engine.is_tools_frozen(),
        "occupancy driver must expose a frozen tool table after ensure"
    );
}
