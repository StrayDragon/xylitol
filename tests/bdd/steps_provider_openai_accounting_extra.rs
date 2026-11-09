use crate::bdd::prelude::*;
use rstest::fixture;
use rstest_bdd_macros::{given, then, when};

pub struct T2EstBdd {
    pub entries: RefCell<Vec<SessionEntry>>,
    pub est: RefCell<Option<xylitol_plugin_sdk::model::ContextTokenEstimate>>,
}

#[fixture]
pub fn t2_est_bdd() -> T2EstBdd {
    T2EstBdd {
        entries: RefCell::new(Vec::new()),
        est: RefCell::new(None),
    }
}

fn t2_assistant_entry(
    id: &str,
    usage: xylitol_plugin_sdk::message::XyUsage,
    stop: xylitol_plugin_sdk::message::XyStopReason,
) -> SessionEntry {
    use xylitol_plugin_sdk::message::{AgentMessage, AgentPart, XyLlmMessage};
    let asst = AgentMessage::Llm(XyLlmMessage::AssistantMessage {
        content: vec![AgentPart::text("ok")],
        stop_reason: Some(stop),
        usage: Some(usage),
        api: String::new(),
        provider: String::new(),
        model: String::new(),
        response_id: None,
        error_message: None,
        timestamp: 0,
        diagnostics: Vec::new(),
    });
    SessionEntry::Message(MessageEntry {
        base: EntryBase {
            entry_type: "message".into(),
            id: id.into(),
            parent_id: None,
            timestamp: 0,
        },
        message: serde_json::to_value(asst).unwrap(),
    })
}

#[given("会话条目含 stop 回合的 usage 锚点")]
pub(crate) fn g_t2_anchor_stop(t2_est_bdd: &T2EstBdd) {
    use xylitol_plugin_sdk::message::{AgentMessage, XyStopReason, XyUsage};
    *t2_est_bdd.entries.borrow_mut() = vec![
        SessionEntry::Message(MessageEntry {
            base: EntryBase {
                entry_type: "message".into(),
                id: "u1".into(),
                parent_id: None,
                timestamp: 0,
            },
            message: serde_json::to_value(AgentMessage::user("hi")).unwrap(),
        }),
        t2_assistant_entry(
            "a1",
            XyUsage {
                input: 100,
                output: 20,
                total_tokens: 120,
                ..Default::default()
            },
            XyStopReason::Stop,
        ),
    ];
}

#[given("会话条目含缓存命中的 stop 回合 usage 锚点")]
pub(crate) fn g_c2994_anchor_cache_hit(t2_est_bdd: &T2EstBdd) {
    use xylitol_plugin_sdk::dto::PromptCacheRead;
    use xylitol_plugin_sdk::message::{AgentMessage, XyStopReason, XyUsage};
    *t2_est_bdd.entries.borrow_mut() = vec![
        SessionEntry::Message(MessageEntry {
            base: EntryBase {
                entry_type: "message".into(),
                id: "u1".into(),
                parent_id: None,
                timestamp: 0,
            },
            message: serde_json::to_value(AgentMessage::user("hi")).unwrap(),
        }),
        t2_assistant_entry(
            "a1",
            XyUsage {
                input: 100,
                output: 20,
                total_tokens: 120,
                ..Default::default()
            }
            .with_prompt_cache_read(PromptCacheRead::Tokens(50)),
            XyStopReason::Stop,
        ),
    ];
}

#[then("估计缓存状态为命中且 token 数一致")]
pub(crate) fn t_c2994_estimate_cache_hit(t2_est_bdd: &T2EstBdd) {
    use xylitol_plugin_sdk::dto::PromptCacheRead;
    let est = t2_est_bdd.est.borrow().as_ref().expect("estimate").clone();
    assert_eq!(
        est.prompt_cache_read,
        PromptCacheRead::Tokens(50),
        "c2994: estimate must carry the usage anchor cache read"
    );
}

#[given("会话条目仅含 abort 回合的 usage 锚点")]
pub(crate) fn g_t2_anchor_aborted(t2_est_bdd: &T2EstBdd) {
    use xylitol_plugin_sdk::message::{AgentMessage, XyStopReason, XyUsage};
    *t2_est_bdd.entries.borrow_mut() = vec![
        SessionEntry::Message(MessageEntry {
            base: EntryBase {
                entry_type: "message".into(),
                id: "u1".into(),
                parent_id: None,
                timestamp: 0,
            },
            message: serde_json::to_value(AgentMessage::user("hi")).unwrap(),
        }),
        t2_assistant_entry(
            "a1",
            XyUsage {
                input: 100,
                output: 20,
                total_tokens: 120,
                ..Default::default()
            },
            XyStopReason::Aborted,
        ),
    ];
}

#[when("以同源估计器估计上下文")]
pub(crate) fn w_t2_estimate(t2_est_bdd: &T2EstBdd) {
    let entries = t2_est_bdd.entries.borrow().clone();
    let est = xylitol_official_plugin_compaction::token_estimator::estimate_from_session_entries(
        &entries,
        &xylitol_official_plugin_compaction::token_estimator::EstimateOpts::default(),
    );
    *t2_est_bdd.est.borrow_mut() = Some(est);
}

#[then("估计来源为 Api")]
pub(crate) fn t_t2_prov_api(t2_est_bdd: &T2EstBdd) {
    let est = t2_est_bdd.est.borrow().as_ref().expect("estimate").clone();
    assert_eq!(
        est.provenance,
        xylitol_plugin_sdk::model::TokenProvenance::Api,
        "c2827: stop 回合 usage 应为 Api 锚点"
    );
}

#[then("估计来源降级而非 Api")]
pub(crate) fn t_t2_prov_not_api(t2_est_bdd: &T2EstBdd) {
    let est = t2_est_bdd.est.borrow().as_ref().expect("estimate").clone();
    assert_ne!(
        est.provenance,
        xylitol_plugin_sdk::model::TokenProvenance::Api,
        "c2827: abort 回合 usage MUST NOT 作 Api 锚点"
    );
}
