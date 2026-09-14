# Design — c2810-update-compaction-floor-trigger

## 数值基线（32k 实测形态）

| 量 | 值 | 来源 |
|---|---|---|
| window / reserve / keepRecent | 32,768 / 16,384 / 20,000 | 用户配置 |
| 固定请求开销 | ≈ 9,000 | c16 同源折算（FixedRequestContext） |
| 有效保留尾 | min(20000, 16384−9000) = 7,384 | c8 `effective_keep_budget` clamp |
| 摘要占位（首枚常量） | 2,048 | `SUMMARY_PLACEHOLDER_TOKENS` 常量 |
| 压后地板 | ≈ 18,432（实测后 ≈ 18k，与 AfterCompaction settlement 吻合） | 本 change 新投影 |
| 旧阈值 | 16,384 → **低于地板 → 40/40 churn** | c2 旧公式 |
| 新阈值 | max(16384, 18432 + 4608) = **23,040** | 本 change |
| 迟滞带 | 地板 × 25% ≈ 4,608（每次 compact 买到 ≥ 此值 headroom） | 固定常量 |

稳态循环：tokens 涨到 23,040 → compact → ≈18,400 → 再涨 ≈4,600 tokens 才触发。
第二次 compact 起占位取上一枚 `CompactionEntry.summary` 实测 chars/4，地板估计收敛。

## 领域层（src/agent/compaction/）

### 纯函数（mod.rs，与 `effective_keep_budget` 相邻）

```rust
/// 压后地板投影：固定开销 + 有效保留尾（c8 clamp）+ 摘要占位。
pub(crate) fn projected_post_compact_tokens(
    settings: &CompactionSettings, context_window: u64,
    fixed_overhead: u64, summary_placeholder: u64,
) -> u64 {
    fixed_overhead
        + effective_keep_budget(settings, context_window, fixed_overhead)
        + summary_placeholder
}

/// 有效触发阈值：max(window − reserve, 地板 + 25% 地板)；window=0 → 0（禁触发，同现状）。
pub(crate) fn effective_trigger_threshold(
    context_window: u64, settings: &CompactionSettings, post_compact_floor: u64,
) -> u64 {
    let reserve_side = context_window.saturating_sub(settings.reserve_tokens);
    reserve_side.max(post_compact_floor + post_compact_floor / 4)
}

/// 摘要占位：leaf 上最新 CompactionEntry.summary 的 chars/4；无则 SUMMARY_PLACEHOLDER_TOKENS。
pub(crate) fn summary_placeholder_tokens(entries: &[SessionEntry]) -> u64;
```

- `should_compact(context_tokens, context_window, settings, post_compact_floor: u64)`：
  加第 4 参（地板投影，调用方算好）；`floor=0` 时阈值退化为 `window − reserve`（旧公式，
  与 c8 clamp 的退化纪律同构）。**不为兼容保留旧 3 参签名**（调用点仅 orchestrator +
  测试 + BDD 步骤，全部同步）。
- 现有 3 个 BDD 场景（need-compact / no-compact / disabled-no-compact）在 floor=0 下
  语义不变：threshold = max(window−reserve, 0) = window−reserve。

### orchestrator（orchestrator.rs）

- `maybe_auto_compact`：在 stale 守卫后计算
  `floor = projected_post_compact_tokens(settings, window, overhead, summary_placeholder_tokens(&entries))`，
  以 `should_compact(tokens, window, settings, floor)` 过闸。precomputed settlement 仍是
  tokens SSOT（c16/c26 不变），地板是**独立于 tokens 估计**的第二输入（由 settings + 窗口 +
  leaf 摘要实测决定，勿与 estimate 混算）。
- `run_auto_compaction`（threshold / overflow 共用收口）：
  1. compact 成功后先 quiet 计算 AfterCompaction settlement（复用 `emit_after_compaction_settlement`
     的取数逻辑，抽出 `compute_after_compaction_settlement(...)` 返回 settlement）；
  2. `tokens_after = settlement.estimate.tokens`；
  3. `notice = (tokens_after >= context_window && !manual && 本会话未发过).then(copy)`；
  4. emit `CompactionEnd { tokens_after, notice, .. }` → 再 emit `ContextTokenSettlement`
     事件（顺序不变：end 先于 settlement，c26「经 CompactionEnd 失效」语义不受扰动）。
- manual `compact` 路径：`tokens_after` 照带（呈现有益），notice 恒 None，不计 once 旗标。
- **overflow 路径不设闸**：`maybe_overflow_compact` 不消费地板阈值；其 `run_auto_compaction`
  内部可发 notice（ hopeless 形态常由 overflow 首次暴露，这正是诊断价值）。

## 运行时层（src/agent/runtime/react/）

- once 旗标：`react/mod.rs` 现有 `overflow_recovery_attempted: bool` 同位新增
  `compaction_floor_notice_emitted: bool`（会话生命周期内由 `try_turn_end_compaction`
  以 `&mut` 传递；orchestrator 请求发 notice 时置 true）。orchestrator 本身每次调用新建
  （turn_end.rs:227），无状态可寄居，旗标必须住在 ReAct 运行时。
- notice 文案（产品英文一行，对齐既有滚动提示语域）：
  `Context still ~{M} tokens after compaction — lower keepRecentTokens, raise contextWindow, or trim tool surface`

## protocol 与 wire（src/protocol/）

- `XyEvent::CompactionEnd` 增 `tokens_after: Option<u64>`、`notice: Option<String>`
  （serde default + skip_serializing_if，与 tokens_before 同款）。
- wire `Event::CompactionEnd` 同步 + `to_wire_event` / `TryFrom` 投影；旧无字段 JSON
  解码为 None（顺延 pa-wire3 既有纪律与 c25 测试族形态）。
- **不**扩 `CompactionEntry`（c11 / 会话格式不动）；resume/rebuild 从 CompactionEntry
  重建块时无 M，词形落回 N 形。

## TUI（packages 之外，src/app/tui/）

- `bridge/model.rs`（≈:410）：`tokens_after` 存在 → 头行
  `Compacted from {N} → {M} tokens`；缺省 → 现 `N` 形。
- `widgets/scrollback/paint.rs`（≈:744）头行格式化同步；insta 快照与
  `scrollback/tests.rs` 断言更新（`Compacted from 186,842 tokens` 等 → 依载荷）。
- `notice` 非空 → bridge 尾插一条 `UiEntry::ScrollNotice`（一次性诊断行），MUST NOT
  改动 compaction 块折叠态 / 不叠第二条提示。
- 数字千分位：沿用既有 `186,842` 逗号分隔形。

## 规约落点（Specs landing 清单）

| 文件 | 变更 |
|---|---|
| `domain-compaction.feature` | c2 公式修订 + scope 限定句；c26 追加 tokens_after 同源句；新增 c28 诊断规则；`reserve-trigger-shares-footer-estimate` 可执行场景比较式同步；新增 2 个地板阈值可执行场景 |
| `protocol-app.feature` | pa-wire3 载荷清单 + 旧载荷解码句扩 tokens_after / notice |
| `app-tui-bridge.feature` | compaction 块条款：词形 N → M（缺省落回）+ notice → ScrollNotice 尾插一句 |
| `agent-runtime.feature` | turn-end 触发义务条款括注的公式引用从「c1630 reserve 公式」改为「domain-compaction c2 地板感知阈值」（义务不变） |

## 测试设计

- 单测（mod.rs / orchestrator.rs）：32k 数值复现表（floor=18,432 → threshold=23,040；
  20,000 → false；24,000 → true；floor=0 退化旧公式）；占位自适应
  （有/无 CompactionEntry）；tokens_after / notice（发、不发、once）；manual/overflow 豁免。
- BDD：`floor-threshold-holds` / `floor-cross-triggers` 两个可执行场景 + 新 given 步骤
  （参数化窗口、固定开销）；`bindings_domain_compaction.rs` 同步注册。
- wire 往返：tokens_after / notice 成功形态 + 旧载荷缺省解码不 panic。
- TUI harness：块头词形两形态、notice 尾插、resume 落回 N 形。
- `#[ignore]` lab：`tests/lab_compaction_replay.rs`，env
  `XYLITOL_COMPACTION_REPLAY_FIXTURE=<session.jsonl>`；读真实 JSONL → in-memory store
  回放：fake model 承担摘要调用；断言单调下降 / 切点合法 / 摘要有界 /
  追加 40 轮固定内容后 compact 次数 ≤ 预期上界。doc 注释给运行命令。

## 风险与回退

- 阈值升高意味着超过 window−reserve 后才 compact（32k 场景 16.4k→23k），单响应可用
  headroom 变薄 → 由 overflow Case1 兜底（既有一次 compact-and-retry），且这正是
  「保留频繁、买真 headroom」的取舍；常规大窗口配置零变化。
- 地板低估（首枚摘要超常量）：最坏一次额外 compact 后自适应收敛；压后 ≥ window 的
  hopeless 形态由一次性诊断显式暴露，不再静默 churn。
