use std::sync::Arc;

use crate::bdd::prelude::*;
use futures::StreamExt;
use rstest_bdd_macros::{given, then, when};
use xylitol_plugin_sdk::message::XyLlmMessage;
use xylitol_plugin_sdk::message::XyStopReason;
use xylitol_plugin_sdk::model::{XyChunk, XyToolSchema};
use xylitol_plugin_sdk::{XyGenerateOptions, XyModel, XyStream};
use xylitol_runtime::agent::sampler::{
    LedgerState, SampleHandle, SampleKind, SamplerActor, aggregate,
};

struct GatedModel {
    text: &'static str,
    release: tokio::sync::Notify,
}

impl GatedModel {
    fn new(text: &'static str) -> Arc<Self> {
        Arc::new(Self {
            text,
            release: tokio::sync::Notify::new(),
        })
    }
}

#[async_trait::async_trait]
impl XyModel for GatedModel {
    fn name(&self) -> &str {
        "gated"
    }

    async fn generate_stream(
        &self,
        _messages: Vec<XyLlmMessage>,
        _tools: &[XyToolSchema],
        _stream: bool,
        _options: XyGenerateOptions,
    ) -> Result<XyStream, xylitol_plugin_sdk::error::XyError> {
        self.release.notified().await;
        let text = self.text;
        Ok(Box::pin(futures::stream::iter(vec![
            Ok(XyChunk::TextDelta(text.to_string())),
            Ok(XyChunk::Done {
                usage: None,
                finish_reason: XyStopReason::Stop,
            }),
        ])))
    }
}

pub struct SamplerBdd {
    actor: SamplerActor,
    handles: RefCell<Vec<SampleHandle>>,
    first: RefCell<Option<Arc<GatedModel>>>,
    second: RefCell<Option<Arc<GatedModel>>>,
}

#[rstest::fixture]
pub fn sampler_bdd() -> SamplerBdd {
    SamplerBdd {
        actor: SamplerActor::new(),
        handles: RefCell::new(Vec::new()),
        first: RefCell::new(None),
        second: RefCell::new(None),
    }
}

#[given("同一会话并行发起两个模型请求")]
pub(crate) async fn g_two_parallel_requests(sampler_bdd: &SamplerBdd) {
    let first = GatedModel::new("first-answer");
    let second = GatedModel::new("second-answer");
    let handle_a = sampler_bdd
        .actor
        .submit_stream(
            SampleKind::Turn,
            first.clone(),
            Vec::new(),
            Vec::new(),
            XyGenerateOptions::default(),
            None,
        )
        .await;
    let handle_b = sampler_bdd
        .actor
        .submit_stream(
            SampleKind::Turn,
            second.clone(),
            Vec::new(),
            Vec::new(),
            XyGenerateOptions::default(),
            None,
        )
        .await;
    assert_eq!(
        sampler_bdd.actor.active_count(),
        2,
        "both parallel requests must be registered as in flight"
    );
    *sampler_bdd.handles.borrow_mut() = vec![handle_a, handle_b];
    *sampler_bdd.first.borrow_mut() = Some(first);
    *sampler_bdd.second.borrow_mut() = Some(second);
}

#[when("两个请求先后完成")]
pub(crate) async fn w_requests_complete_in_turn(sampler_bdd: &SamplerBdd) {
    let second = sampler_bdd
        .second
        .borrow()
        .clone()
        .expect("second model gated");
    let first = sampler_bdd
        .first
        .borrow()
        .clone()
        .expect("first model gated");
    second.release.notify_one();
    tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    assert!(
        sampler_bdd
            .actor
            .is_active(sampler_bdd.handles.borrow()[0].id()),
        "the first-submitted request must still be in flight"
    );
    first.release.notify_one();
}

#[then("采样记账按发起序汇聚")]
pub(crate) async fn t_ledger_aggregates_in_submission_order(sampler_bdd: &SamplerBdd) {
    let handles = sampler_bdd
        .handles
        .borrow_mut()
        .drain(..)
        .collect::<Vec<_>>();
    let results = aggregate(handles).await;
    assert_eq!(results.len(), 2);
    let mut first_text = String::new();
    if let Ok(mut stream) = results.into_iter().next().unwrap() {
        while let Some(chunk) = stream.next().await {
            if let Ok(XyChunk::TextDelta(delta)) = chunk {
                first_text.push_str(&delta);
            }
        }
    }
    assert_eq!(
        first_text, "first-answer",
        "aggregation must return results in submission order even though the later request completed first"
    );
    let ledger = sampler_bdd.actor.ledger();
    assert_eq!(
        ledger.iter().map(|r| r.kind.as_str()).collect::<Vec<_>>(),
        vec!["turn", "turn"],
        "the ledger must keep submission order: {ledger:?}"
    );
    assert!(
        ledger.iter().all(|r| r.state == LedgerState::Completed),
        "both requests must be deregistered on completion: {ledger:?}"
    );
    assert_eq!(
        sampler_bdd.actor.active_count(),
        0,
        "the ledger must be empty of in-flight requests"
    );
}
