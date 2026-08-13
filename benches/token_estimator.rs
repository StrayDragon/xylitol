//! Statistical micro-benchmarks for the context-token accounting path
//! (criterion wiring — complements `just qa` complexity gates with perf
//! regression data; not part of qa).
//!
//! Run: `cargo bench --bench token_estimator` (release profile; see Cargo.toml).

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use std::hint::black_box;

use xylitol::agent::compaction::token_estimator::{EstimateOpts, estimate_context_tokens_with};
use xylitol::protocol::message::{AgentMessage, AgentPart, LlmMessage, XyStopReason, XyUsage};
use xylitol::protocol::model::TokenProvenance;

fn sample_messages(turns: usize) -> Vec<AgentMessage> {
    (0..turns)
        .flat_map(|i| {
            let user_text = format!("please review file src/lib.rs chunk {i} and summarize");
            let assistant_text = format!("here is the summary for chunk {i}: all good");
            [
                AgentMessage::Llm(LlmMessage::UserMessage {
                    content: vec![AgentPart::Text { text: user_text }],
                    timestamp: 0,
                }),
                AgentMessage::Llm(LlmMessage::AssistantMessage {
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
                let est = estimate_context_tokens_with(black_box(msgs), None, None, &opts);
                // Keep the result observable so the optimizer can't elide the loop.
                black_box(est.tokens.max(1));
            });
        });
    }
    group.finish();

    // Provenance: touch the strum SSOT path so string derivations stay benchmarked.
    let provenance = [TokenProvenance::Api, TokenProvenance::LocalTokenizer];
    c.bench_function("provenance_as_str", |b| {
        b.iter(|| {
            for p in &provenance {
                black_box(p.as_str());
            }
        });
    });
}

criterion_group!(benches, bench_estimate_context_tokens);
criterion_main!(benches);
