# Tasks — c2810-update-compaction-floor-trigger

## 测试 seam（复用既有 harness，不新发明）

1. **compaction 领域**：`should_compact` / 阈值投影 / 占位自适应单测（`src/agent/compaction/*` 既有测试族，in-memory `SessionManager` + fake model）。
2. **wire 投影**：`protocol::wire::event` 往返单测（既有 `*_roundtrips_through_wire_event` 测试族 + 旧载荷缺省解码族）。
3. **BDD**：`tests/bdd/steps_compaction.rs` 步骤族 + `tests/bdd/bindings_domain_compaction.rs` 场景注册。
4. **TUI**：bridge 词形 / notice 尾插经 `app/tui` harness 与 insta 快照；`scrollback/tests.rs` 既有 `Compacted from` 断言族。
5. **lab**：`tests/lab_compaction_replay.rs`（`#[ignore]`，env 门控，不进 qa）。

## Tasks

### T1 领域：地板投影 + 有效阈值（含 32k 数值复现）

- `mod.rs`：`projected_post_compact_tokens` / `effective_trigger_threshold` / `summary_placeholder_tokens` 三纯函数（设计稿签名）；`SUMMARY_PLACEHOLDER_TOKENS = 2048`。
- `should_compact` 加 `post_compact_floor: u64` 第 4 参；floor=0 退化 `window − reserve`；同步全部调用点（orchestrator + 测试 + BDD 步骤）。
- 单测：32k 形态表（floor 18,432 → threshold 23,040；tokens 20,000 → false / 24,000 → true）；退化形态；占位自适应（有 / 无 CompactionEntry）。
- 撑 spec：`domain-compaction` c2 修订。

### T2 orchestrator：threshold 过闸 + tokens_after / notice 收口

- `maybe_auto_compact`：stale 守卫后算地板过闸（precomputed settlement 仍是 tokens SSOT）。
- `run_auto_compaction`：抽出 `compute_after_compaction_settlement`；CompactionEnd 先带 `tokens_after` / `notice` 发出，再发 settlement 事件（顺序不变）；once 判定消费调用方旗标。
- manual `compact`：tokens_after 照带、notice 恒 None；overflow 路径：不设闸、可发 notice。
- 单测：threshold 不再逐轮 churn（32k 形态 fake 模拟 N 轮，compact 次数上界断言）；manual / overflow 豁免；once 语义。
- 撑 spec：`domain-compaction` c2 / c26 / c28。

### T3 runtime：once 旗标接线

- `react/mod.rs`：`compaction_floor_notice_emitted: bool` 与 `overflow_recovery_attempted` 同位；`try_turn_end_compaction` `&mut` 下探。
- 单测：同会话二次满足条件不再发。
- 撑 spec：`domain-compaction` c28（once 频度）。

### T4 protocol：CompactionEnd 载荷 + wire 往返

- `XyEvent::CompactionEnd` + wire `Event::CompactionEnd` 增 `tokens_after: Option<u64>` / `notice: Option<String>`（serde 缺省）；`to_wire_event` / `TryFrom` 投影。
- 往返单测：全载荷形态 + 旧无字段 JSON 解码 None 不 panic。
- **不**扩 `CompactionEntry`（会话格式零变更）。
- 撑 spec：`protocol-app` pa-wire3、`domain-compaction` c26。

### T5 TUI：词形 N → M + notice 尾插

- `bridge/model.rs`：头行 `Compacted from N → M tokens`（缺省落回 N 形）；`notice` → `UiEntry::ScrollNotice` 尾插一行，不动折叠态。
- `widgets/scrollback/paint.rs` 头行同步；insta 快照与 `scrollback/tests.rs` 断言更新。
- harness 测试：两形态词形、notice 尾插、resume/rebuild 落回 N 形。
- 撑 spec：`app-tui-bridge` compaction 块条款修订。

### T6 BDD：地板阈值可执行场景

- 新 given 步骤：参数化窗口（`配置了上下文窗口为 {n:u64} 的模型`）、`compaction 固定请求开销为 {n:u64} token`。
- 场景：`floor-threshold-holds`（20000 < 23040 → false）、`floor-cross-triggers`（24000 > 23040 → true）；`reserve-trigger-shares-footer-estimate` 比较式措辞同步。
- `bindings_domain_compaction.rs` 注册新场景。

### T7 lab 回放 harness（#[ignore]，不进 qa）

- `tests/lab_compaction_replay.rs`：env `XYLITOL_COMPACTION_REPLAY_FIXTURE` 门控；真实 JSONL → in-memory 回放，fake model 摘要；断言单调下降 / 切点合法 / 摘要有界 / compact 次数上界；doc 注释给运行命令；仓库零 fixture。

### T8 门禁收口

- `just fmt` / `just lint` / `cargo test -p xylitol --lib --all-features` / `just test-tui` 全绿。
- `llman sdd validate c2810-update-compaction-floor-trigger --strict --no-interactive` 全绿。
