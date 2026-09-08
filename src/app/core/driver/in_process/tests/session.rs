use super::*;
use crate::protocol::session::{SessionTreeKind, ThinkingLevelChangeEntry};

#[tokio::test]

async fn switch_session_restores_sticky_thinking_without_rewriting() {
    let dir = tempfile::tempdir().unwrap();
    let store = Arc::new(SessionManager::new(dir.path().join("sessions")));
    let (mut driver, _obs) = build_test_driver(store.clone()).await;
    let target = "restored-thinking";
    store.create(target, Some("."), None).await.unwrap();
    store
        .append(
            target,
            &SessionEntry::ThinkingLevelChange(ThinkingLevelChangeEntry {
                base: EntryBase {
                    entry_type: "thinking_level_change".into(),
                    id: String::new(),
                    parent_id: None,
                    timestamp: 0,
                },
                thinking_level: "vendor-retired".into(),
            }),
        )
        .await
        .unwrap();
    store
        .append(target, &msg_entry("a1", None, "assistant", "flush"))
        .await
        .unwrap();
    let session_file = store.get_session_file(target).unwrap();
    let before = std::fs::read(&session_file).unwrap();

    driver.switch_session(target).await.unwrap();

    assert_eq!(driver.thinking_level(), "vendor-retired");
    assert_eq!(std::fs::read(session_file).unwrap(), before);
}

#[tokio::test]

async fn in_process_session_tree_ensures_missing_session() {
    let dir = tempfile::tempdir().unwrap();
    let store = Arc::new(SessionManager::new(dir.path().join("sessions")));
    let store_trait: Arc<dyn XySessionStore> = store.clone();
    let mut agent = AgentBuilder::new(
        crate::agent::model::registry::ModelRegistry::new(),
        Arc::new(crate::infra::provider::factory::build_provider),
        store_trait.clone(),
        Arc::new(EventBus::new()) as Arc<dyn XyEventSink>,
        permission::allow_all_permission(),
    )
    .cwd(".")
    .tools(ToolSet::from_iter(crate::infra::tools::default_tools()))
    .build();
    // Orphan id: set on agent but never created on disk (wipe / pre-persist).
    let orphan = uuid::Uuid::new_v4().to_string();
    agent.bind_session(orphan.clone()).expect("bind_session");
    let driver = XyInProcessDriver::new(agent, store);
    let tree = driver
        .session_tree(SessionTreeKind::MessageHistory)
        .await
        .expect("empty tree after ensure");
    assert!(tree.is_empty(), "expected empty tree, got {tree:?}");
    assert!(
        store_trait.exists(&orphan).await,
        "ensure_session must create orphan session"
    );
}

#[tokio::test]

async fn in_process_session_tree_returns_parent_child() {
    let dir = tempfile::tempdir().unwrap();
    let store = Arc::new(SessionManager::new(dir.path().join("sessions")));
    let (driver, _obs) = build_test_driver(store.clone()).await;
    let sid = driver.session_id().expect("session");

    store
        .append_with_id(&sid, &msg_entry("u1", None, "user", "hello"))
        .await
        .expect("append u1");
    store
        .append_with_id(&sid, &msg_entry("a1", Some("u1"), "assistant", "hi"))
        .await
        .expect("append a1");

    let loaded = store.load(&sid).await.expect("load after append");
    let ids: Vec<_> = loaded.iter().filter_map(|e| e.entry_id()).collect();
    assert_eq!(
        ids,
        vec!["u1", "a1"],
        "persisted entries before tree: {loaded:?}"
    );

    let tree = driver
        .session_tree(SessionTreeKind::MessageHistory)
        .await
        .expect("tree");
    assert_eq!(tree.len(), 1, "tree={tree:?} loaded={loaded:?}");
    assert_eq!(tree[0].entry.entry_id(), Some("u1"));
    assert_eq!(
        tree[0].children.len(),
        1,
        "expected a1 under u1; tree={tree:?} loaded={loaded:?}"
    );
    assert_eq!(tree[0].children[0].entry.entry_id(), Some("a1"));
}

#[tokio::test]

async fn in_process_travel_user_sets_parent_leaf_and_editor_text() {
    let dir = tempfile::tempdir().unwrap();
    let store = Arc::new(SessionManager::new(dir.path().join("sessions")));
    let (driver, _obs) = build_test_driver(store.clone()).await;
    let sid = driver.session_id().expect("session");

    store
        .append_with_id(&sid, &msg_entry("u1", None, "user", "edit me"))
        .await
        .unwrap();
    store
        .append_with_id(&sid, &msg_entry("a1", Some("u1"), "assistant", "reply"))
        .await
        .unwrap();
    XySessionStore::set_leaf(store.as_ref(), &sid, Some("a1"));

    let travel = driver
        .travel_session_tree(SessionTreeKind::MessageHistory, "u1")
        .await
        .expect("travel");
    assert_eq!(travel.leaf_id, None);
    assert_eq!(travel.editor_text.as_deref(), Some("edit me"));
    assert_eq!(XySessionStore::leaf_id(store.as_ref(), &sid), None);
}

#[tokio::test]

async fn in_process_travel_non_user_sets_leaf_without_editor_text() {
    let dir = tempfile::tempdir().unwrap();
    let store = Arc::new(SessionManager::new(dir.path().join("sessions")));
    let (driver, _obs) = build_test_driver(store.clone()).await;
    let sid = driver.session_id().expect("session");

    store
        .append_with_id(&sid, &msg_entry("u1", None, "user", "hello"))
        .await
        .unwrap();
    store
        .append_with_id(&sid, &msg_entry("a1", Some("u1"), "assistant", "reply"))
        .await
        .unwrap();

    let travel = driver
        .travel_session_tree(SessionTreeKind::MessageHistory, "a1")
        .await
        .expect("travel");
    assert_eq!(travel.leaf_id.as_deref(), Some("a1"));
    assert!(travel.editor_text.is_none());
    assert_eq!(
        XySessionStore::leaf_id(store.as_ref(), &sid).as_deref(),
        Some("a1")
    );
}

#[tokio::test]

async fn unsupported_tree_kind_returns_err_without_changing_leaf() {
    let dir = tempfile::tempdir().unwrap();
    let store = Arc::new(SessionManager::new(dir.path().join("sessions")));
    let (driver, _obs) = build_test_driver(store.clone()).await;
    let sid = driver.session_id().expect("session");
    XySessionStore::set_leaf(store.as_ref(), &sid, Some("keep"));

    let err = driver
        .session_tree(SessionTreeKind::FileBrowser)
        .await
        .expect_err("file_browser");
    assert!(err.to_string().contains("file_browser"));
    assert_eq!(
        XySessionStore::leaf_id(store.as_ref(), &sid).as_deref(),
        Some("keep")
    );

    let err = driver
        .travel_session_tree(SessionTreeKind::FileBrowser, "x")
        .await
        .expect_err("travel file_browser");
    assert!(err.to_string().contains("file_browser"));
    assert_eq!(
        XySessionStore::leaf_id(store.as_ref(), &sid).as_deref(),
        Some("keep")
    );
}

#[tokio::test]

async fn after_run_session_tree_reflects_persisted_turn() {
    use std::pin::Pin;

    use async_trait::async_trait;
    use futures::StreamExt;

    use crate::protocol::error::XyError;
    use crate::protocol::message::XyStopReason;
    use crate::protocol::model::XyModelConfig;
    use crate::protocol::model::{XyChunk, XyModelMeta, XyToolSchema};
    use crate::protocol::ports::{XyModel, XyStream};

    struct TextMockModel;
    #[async_trait]
    impl XyModel for TextMockModel {
        fn name(&self) -> &str {
            "text-mock"
        }

        async fn generate_stream(
            &self,
            _messages: Vec<crate::protocol::message::LlmMessage>,
            _tools: &[XyToolSchema],
            _stream: bool,
            _options: crate::protocol::ports::XyGenerateOptions,
        ) -> Result<XyStream, XyError> {
            let chunks = vec![
                Ok(XyChunk::TextDelta("reply".into())),
                Ok(XyChunk::Done {
                    finish_reason: XyStopReason::Stop,
                    usage: None,
                }),
            ];
            Ok(Box::pin(futures::stream::iter(chunks))
                as Pin<
                    Box<dyn futures::Stream<Item = Result<XyChunk, XyError>> + Send>,
                >)
        }
    }

    let dir = tempfile::tempdir().unwrap();
    let store = Arc::new(SessionManager::new(dir.path().join("sessions")));
    let store_trait: Arc<dyn XySessionStore> = store.clone();
    let mut reg = crate::agent::model::registry::ModelRegistry::new();
    reg.register(XyModelMeta {
        id: "mock".into(),
        config: XyModelConfig {
            kind: crate::protocol::model::XyModelKind::Fake,
            api_key: String::new(),
            model: "mock".into(),
            base_url: None,
            api: None,
            compat: None,
        },
        display_name: "Mock".into(),
        thinking: false,
        context_window: 128000,
        api: String::new(),
        provider: String::new(),
        cost_input: 0.0,
        cost_output: 0.0,
        cost_cache_read: 0.0,
        cost_cache_write: 0.0,
        max_tokens: 0,
        thinking_levels: Vec::new(),
        thinking_level_map: Default::default(),
    });
    let builder: ModelBuilderFn = Arc::new(|_| Arc::new(TextMockModel) as Arc<dyn XyModel>);
    let mut agent = AgentBuilder::new(
        reg,
        builder,
        store_trait.clone(),
        Arc::new(EventBus::new()) as Arc<dyn crate::protocol::ports::XyEventSink>,
        permission::allow_all_permission(),
    )
    .cwd(".")
    .tools(ToolSet::empty())
    .build();
    agent.select_model("mock").await.expect("select mock");
    let sid = uuid::Uuid::new_v4().to_string();
    agent.bind_session(sid).expect("bind_session");
    let mut driver = XyInProcessDriver::new(agent, store_trait);

    let mut stream = driver.run("hello tree").await;
    while stream.next().await.is_some() {}

    let tree = driver
        .session_tree(SessionTreeKind::MessageHistory)
        .await
        .expect("tree");
    assert!(
        !tree.is_empty(),
        "session tree should reflect persisted messages"
    );
}

#[tokio::test]

async fn fork_rejects_unflushed_session_via_driver() {
    // TUI cannot hit this while assistant is streaming (steer takes over); cover via XyDriver.
    let dir = tempfile::tempdir().unwrap();
    let store = Arc::new(SessionManager::new(dir.path().join("sessions")));
    let (mut driver, _obs) = build_test_driver(store.clone()).await;
    let sid = driver.session_id().expect("session");
    let path = store.get_session_file(&sid).expect("persisted path");
    assert!(
        !path.exists(),
        "build_test_driver leaves session pending-only"
    );

    store
        .append(
            &sid,
            &SessionEntry::Message(MessageEntry {
                base: EntryBase {
                    entry_type: "message".into(),
                    id: String::new(),
                    parent_id: None,
                    timestamp: 0,
                },
                message: crate::protocol::session::fixture_message_json("user", "only user"),
            }),
        )
        .await
        .expect("pending user");
    assert!(!path.exists());

    let uid = store
        .load(&sid)
        .await
        .expect("load")
        .iter()
        .find_map(|e| e.entry_id())
        .expect("user id")
        .to_string();

    let err = driver
        .fork_session(&uid, crate::protocol::session::ForkPosition::At)
        .await
        .expect_err("unflushed fork");
    assert!(
        err.to_string().contains("not been saved yet"),
        "pi unflushed guard via XyDriver: {err}"
    );
}

/// Default `XyDriver::run` path is Reject: concurrent root while live → Busy, one provider stream.
#[tokio::test]

async fn concurrent_run_rejects_second_with_busy() {
    use std::pin::Pin;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use async_trait::async_trait;
    use futures::StreamExt;

    use crate::protocol::error::XyError;
    use crate::protocol::message::XyStopReason;
    use crate::protocol::model::{XyChunk, XyModelConfig, XyModelMeta, XyToolSchema};
    use crate::protocol::ports::{XyModel, XyStream};

    struct SlowMock {
        calls: Arc<AtomicUsize>,
    }
    #[async_trait]
    impl XyModel for SlowMock {
        fn name(&self) -> &str {
            "slow-mock"
        }
        async fn generate_stream(
            &self,
            _messages: Vec<crate::protocol::message::LlmMessage>,
            _tools: &[XyToolSchema],
            _stream: bool,
            _options: crate::protocol::ports::XyGenerateOptions,
        ) -> Result<XyStream, XyError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(Box::pin(async_stream::stream! {
                for i in 0..40u32 {
                    tokio::time::sleep(std::time::Duration::from_millis(20)).await;
                    yield Ok(XyChunk::TextDelta(format!("c{i}")));
                }
                yield Ok(XyChunk::Done {
                    finish_reason: XyStopReason::Stop,
                    usage: None,
                });
            })
                as Pin<
                    Box<dyn futures::Stream<Item = Result<XyChunk, XyError>> + Send>,
                >)
        }
    }

    let calls = Arc::new(AtomicUsize::new(0));
    let calls_b = calls.clone();
    let dir = tempfile::tempdir().unwrap();
    let store = Arc::new(SessionManager::new(dir.path().join("sessions")));
    let store_trait: Arc<dyn XySessionStore> = store.clone();
    let mut reg = crate::agent::model::registry::ModelRegistry::new();
    reg.register(XyModelMeta {
        id: "mock".into(),
        config: XyModelConfig {
            kind: crate::protocol::model::XyModelKind::Fake,
            api_key: String::new(),
            model: "mock".into(),
            base_url: None,
            api: None,
            compat: None,
        },
        display_name: "Mock".into(),
        thinking: false,
        context_window: 128000,
        api: String::new(),
        provider: String::new(),
        cost_input: 0.0,
        cost_output: 0.0,
        cost_cache_read: 0.0,
        cost_cache_write: 0.0,
        max_tokens: 0,
        thinking_levels: Vec::new(),
        thinking_level_map: Default::default(),
    });
    let builder: ModelBuilderFn = Arc::new(move |_| {
        Arc::new(SlowMock {
            calls: calls_b.clone(),
        }) as Arc<dyn XyModel>
    });
    let mut agent = AgentBuilder::new(
        reg,
        builder,
        store_trait.clone(),
        Arc::new(EventBus::new()) as Arc<dyn crate::protocol::ports::XyEventSink>,
        permission::allow_all_permission(),
    )
    .cwd(".")
    .tools(ToolSet::empty())
    .build();
    agent.select_model("mock").await.expect("select mock");
    let sid = uuid::Uuid::new_v4().to_string();
    agent.bind_session(sid).expect("bind_session");
    let mut driver = XyInProcessDriver::new(agent, store_trait);

    let mut first = driver.run("one").await;
    let mut saw = false;
    while let Some(ev) = first.next().await {
        if matches!(ev, crate::protocol::lifecycle::XyEvent::TextDelta(_)) {
            saw = true;
            break;
        }
    }
    assert!(saw, "first driver run must emit text");

    let mut second = driver.run("two").await;
    let mut busy = false;
    let mut second_text = false;
    while let Some(ev) = second.next().await {
        match ev {
            crate::protocol::lifecycle::XyEvent::Error(err)
                if err.kind == crate::agent::runtime::state::RuntimeControlError::Busy.kind() =>
            {
                busy = true
            }
            crate::protocol::lifecycle::XyEvent::TextDelta(_) => second_text = true,
            _ => {}
        }
    }
    assert!(busy, "second XyDriver::run must Busy");
    assert!(!second_text, "rejected run must not stream model text");
    assert_eq!(calls.load(Ordering::SeqCst), 1, "only one provider stream");

    while first.next().await.is_some() {}
}

#[tokio::test]
async fn reader_driver_switch_session_does_not_stomp_obs_slot() {
    // otel25 / c2610: host reader materialization binds the target session on its
    // own runtime; the active obs identity must stay the writer's session.
    let dir = tempfile::tempdir().unwrap();
    let store = Arc::new(SessionManager::new(dir.path().join("sessions")));
    let target = "reader-target";
    store.create(target, Some("."), None).await.unwrap();

    let (mut reader, _scope) = build_test_driver(store.clone()).await;
    let writer_identity = xylitol_ai_bridge::provider::obs_session_context();
    reader.agent.set_obs_slot_writes(false);

    reader.switch_session(target).await.unwrap();

    assert_eq!(reader.session_id(), Some(target.to_string()));
    assert_eq!(
        xylitol_ai_bridge::provider::obs_session_context(),
        writer_identity,
        "reader materialization must not stomp the obs identity"
    );
}
