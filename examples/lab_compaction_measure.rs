//! c2848 T1 校准对拍 lab（只读）：对深会话 0664dab6 输出三口径 token 度量对照。
//! - A: 切点路径 chars/4 启发式（`estimate_tokens_entry_for_cut` 累计）
//! - B: 统一估算（`estimate_from_session_entries` · EstimateOpts::default，footer/触发同族）
//! - C: provider 实测 input（Langfuse llm.request，观测 17441）
//!
//! 运行：`cargo run --example lab_compaction_measure -- [session_id]`
use xylitol::agent::compaction::EstimateOpts;
use xylitol::agent::compaction::cut_detector::estimate_tokens_entry_for_cut;
use xylitol::agent::compaction::token_estimator::estimate_from_session_entries;
use xylitol::protocol::session::SessionEntry;

#[tokio::main]
async fn main() {
    let sid = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "0664dab6-d6b7-484e-8cd6-52691c91a2d3".into());
    let dir = format!(
        "{}/.xylitol/sessions/{sid}",
        std::env::var("HOME").unwrap_or_else(|_| ".".into())
    );
    let mut entries: Vec<SessionEntry> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let mut segment_files: Vec<_> = std::fs::read_dir(format!("{dir}/segments"))
        .map(|rd| rd.filter_map(|e| e.ok()).map(|e| e.path()).collect())
        .unwrap_or_default();
    segment_files.sort();
    segment_files.push(std::path::PathBuf::from(format!("{dir}/active-1.jsonl")));
    let mut parsed = 0usize;
    let mut skipped = 0usize;
    for path in &segment_files {
        let Ok(text) = std::fs::read_to_string(path) else {
            continue;
        };
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            if let Ok(e) = serde_json::from_str::<SessionEntry>(line) {
                let id = e.entry_id().unwrap_or_default().to_string();
                parsed += 1;
                if seen.insert(id) {
                    entries.push(e);
                }
            } else {
                skipped += 1;
            }
        }
    }
    // keep insertion order from store scan; sums are order-independent
    println!(
        "[measure] entries(parsed={parsed} skipped={skipped} unique={})",
        entries.len()
    );

    // A: 切点路径 chars/4 累计（find_cut_point 同度量）。
    let cut_sum: u64 = entries.iter().map(estimate_tokens_entry_for_cut).sum();
    println!("[A] cut chars/4 cumulative      = {cut_sum}");

    // B: 统一估算（与 footer EstimateContext 同函数族；默认 opts）。
    let est = estimate_from_session_entries(&entries, &EstimateOpts::default());
    println!("[B] unified estimate (default)  = {}", est.tokens);

    // C: provider 实测（Langfuse 观测 15:41Z）。
    println!("[C] provider llm.request input  = 17441 (reference)");

    // 参考 keep 预算：window=33792 reserve=16384 keep_recent=20000。
    let budget = 33_792u64.saturating_sub(16_384);
    println!("[D] effective_keep_budget       = {budget} (= window - reserve)");
    println!(
        "[judge] A({cut_sum}) vs budget({budget}): {} => prepare 'nothing to compact'",
        if cut_sum < budget { "below" } else { "at/over" }
    );
}
