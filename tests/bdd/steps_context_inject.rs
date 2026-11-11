use std::path::PathBuf;
use std::sync::Arc;

use rstest::fixture;
use rstest_bdd_macros::{given, then, when};
use xylitol_plugin_sdk::resource::SkillInfo;
use xylitol_plugin_sdk::source_info::{SourceInfo, SourceOrigin, SourceScope};
use xylitol_plugin_sdk::{BeforeStepCtx, XySessionStore};

use crate::bdd::prelude::*;

pub struct ContextInjectBdd {
    pub(crate) store: RefCell<Option<Arc<dyn XySessionStore>>>,
    pub(crate) skill_path: RefCell<Option<PathBuf>>,
    pub(crate) output: RefCell<Option<String>>,
    pub(crate) session_id: RefCell<String>,
}

impl ContextInjectBdd {
    fn new() -> Self {
        Self {
            store: RefCell::new(None),
            skill_path: RefCell::new(None),
            output: RefCell::new(None),
            session_id: RefCell::new(String::new()),
        }
    }
}

#[fixture]
pub fn context_inject_bdd() -> ContextInjectBdd {
    ContextInjectBdd::new()
}

async fn produce(context_inject_bdd: &ContextInjectBdd) -> Option<String> {
    use xylitol_official_plugin_context_inject::drift::DriftInjector;
    use xylitol_official_plugin_context_inject::registry::{InjectorContext, NativeInjector};

    let store = context_inject_bdd
        .store
        .borrow()
        .clone()
        .expect("session store");
    let path = context_inject_bdd
        .skill_path
        .borrow()
        .clone()
        .expect("skill path");
    let session_id = context_inject_bdd.session_id.borrow().clone();
    let before = BeforeStepCtx {
        skills: vec![SkillInfo {
            name: "B".into(),
            description: None,
            source_info: SourceInfo {
                path,
                source: "bdd".into(),
                scope: SourceScope::Project,
                origin: SourceOrigin::TopLevel,
                base_dir: None,
            },
            disable_model_invocation: false,
        }],
        session_id: Some(session_id.clone()),
        ..Default::default()
    };
    let injector = DriftInjector::default();
    let injector_ctx = InjectorContext {
        before: &before,
        store: Some(store.as_ref()),
        session_id: Some(session_id.as_str()),
    };
    injector.produce(&injector_ctx).await
}

#[given("会话快照记录技能 B 的内容指纹")]
async fn g_snapshot_baseline(context_inject_bdd: &ContextInjectBdd) {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("B.md");
    std::fs::write(&path, "B").expect("skill content");
    let manager = xylitol_official_plugin_session::SessionManager::new(dir.keep());
    *context_inject_bdd.store.borrow_mut() = Some(Arc::new(manager));
    *context_inject_bdd.skill_path.borrow_mut() = Some(path);
    *context_inject_bdd.session_id.borrow_mut() = "ctx-bdd".into();
    assert!(
        produce(context_inject_bdd).await.is_none(),
        "first evaluation must establish the baseline without a reminder"
    );
}

#[when("技能 B 内容变为 B1 且进入下一 turn_pre")]
async fn w_skill_changed(context_inject_bdd: &ContextInjectBdd) {
    let path = context_inject_bdd
        .skill_path
        .borrow()
        .clone()
        .expect("skill path");
    std::fs::write(path, "B1").expect("skill content");
    *context_inject_bdd.output.borrow_mut() = produce(context_inject_bdd).await;
}

#[when("会话离开期间技能 B 内容变更后 resume")]
async fn w_resume(context_inject_bdd: &ContextInjectBdd) {
    let path = context_inject_bdd
        .skill_path
        .borrow()
        .clone()
        .expect("skill path");
    std::fs::write(path, "B1").expect("skill content");
    *context_inject_bdd.output.borrow_mut() = produce(context_inject_bdd).await;
}

#[then("模型上下文含技能 B 变更提醒")]
fn t_reminder(context_inject_bdd: &ContextInjectBdd) {
    assert_reminder(context_inject_bdd);
}

#[then("下一 turn_pre 注入技能 B 变更提醒")]
fn t_resume_reminder(context_inject_bdd: &ContextInjectBdd) {
    assert_reminder(context_inject_bdd);
}

fn assert_reminder(context_inject_bdd: &ContextInjectBdd) {
    let output = context_inject_bdd.output.borrow();
    let output = output.as_ref().expect("drift must emit a reminder");
    assert!(output.contains("[skills] skill \"B\""), "{output}");
    assert!(output.contains("已变更"), "{output}");
}
