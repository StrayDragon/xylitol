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

#[tokio::test]
async fn occupancy_driver_run_queued_delegates_to_the_real_queue() {
    use futures::StreamExt;
    use xylitol_plugin_sdk::lifecycle::XyEvent;
    use xylitol_runtime::XyDriver;

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
    engine.bind_session("s0").await.expect("bind");
    for text in ["t1", "t2"] {
        engine
            .execute_command(xylitol_protocol::Command::FollowUp {
                message: text.to_string(),
            })
            .await
            .expect("enqueue through the writer command path");
    }

    let stream = XyDriver::run_queued(engine.as_mut()).await;
    let mut stream = stream;
    let mut events = Vec::new();
    while let Some(event) = stream.next().await {
        events.push(event);
    }
    assert!(
        events
            .iter()
            .any(|event| matches!(event, XyEvent::MessageStart { role, .. } if role == "user")),
        "the queued head must enter history through the queue path (user MessageStart): {events:?}"
    );
    assert!(
        XyDriver::queue_items(engine.as_mut())
            .iter()
            .all(
                |item| item.kind != xylitol_plugin_sdk::session::QueueItemKind::FollowUp
                    || item.preview != "t1"
            ),
        "run_queued must consume the head it started"
    );
}
