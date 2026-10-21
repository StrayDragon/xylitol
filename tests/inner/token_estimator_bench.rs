//! Mounted in-crate as `#[cfg(test)]` (bench targets are separate crates and
use criterion::{BenchmarkId, Criterion};
use std::hint::black_box;

use xylitol_plugin_sdk::message::{AgentMessage, AgentPart, XyLlmMessage, XyStopReason, XyUsage};
use xylitol_plugin_sdk::model::TokenProvenance;
use xylitol_plugin_sdk::token_estimator::{EstimateOpts, estimate_context_tokens_with};
use xylitol_runtime::agent::prompt::AgentStatusBar;

fn sample_messages(turns: usize) -> Vec<AgentMessage> {
    (0..turns)
        .flat_map(|i| {
            let user_text = format!("please review file src/lib.rs chunk {i} and summarize");
            let assistant_text = format!("here is the summary for chunk {i}: all good");
            [
                AgentMessage::Llm(XyLlmMessage::UserMessage {
                    content: vec![AgentPart::Text { text: user_text }],
                    timestamp: 0,
                }),
                AgentMessage::Llm(XyLlmMessage::AssistantMessage {
                    content: vec![AgentPart::Text {
                        text: assistant_text,
                    }],
                    stop_reason: Some(XyStopReason::Stop),
                    usage: Some(XyUsage {
                        input: 40,
                        output: 20,
                        cache_read: 0,
                        cache_write: 0,
                        cache_write_1h: 0,
                        total_tokens: 60,
                        cost: None,
                        prompt_cache_read: Default::default(),
                    }),
                    api: String::new(),
                    provider: String::new(),
                    model: String::new(),
                    response_id: None,
                    error_message: None,
                    timestamp: 0,
                    diagnostics: Vec::new(),
                }),
            ]
        })
        .collect()
}

fn bench_estimate_context_tokens(c: &mut Criterion) {
    let mut group = c.benchmark_group("context_token_estimate");
    let opts = EstimateOpts::default();
    for turns in [16usize, 64, 256] {
        let messages = sample_messages(turns);
        group.bench_with_input(BenchmarkId::from_parameter(turns), &messages, |b, msgs| {
            b.iter(|| {
                let est = estimate_context_tokens_with(
                    black_box(msgs),
                    None,
                    None,
                    &opts,
                    &AgentStatusBar::default(),
                );
                black_box(est.tokens.max(1));
            });
        });
    }
    group.finish();

    let provenance = [TokenProvenance::Api, TokenProvenance::Heuristic];
    c.bench_function("provenance_as_str", |b| {
        b.iter(|| {
            for p in &provenance {
                black_box(p.as_str());
            }
        });
    });
}

#[test]
#[ignore = "perf bench: run via `cargo test -r --lib token_estimator -- --ignored --nocapture`"]
fn token_estimator_perf() {
    let mut c = Criterion::default();
    bench_estimate_context_tokens(&mut c);
}
