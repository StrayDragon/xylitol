---
depends_on: []
needs_specs_change: true
rules_touched:
- c8
- c16
- c26
branch: fix/compact-overflow-reproject
base_sha: 62c22b7d83f750e52165ab6f8646df8a2d654f92
---

## Why

实测 session `92fa9adf-f834-410a-bcbe-9a01805e33fe`（窗口 32768 / reserve 16384 的本地模型）暴露 compaction 三处断裂，互为因果：

1. **切点预算失配 → compact 不瘦身**。`find_cut_point` 走查用 chars/4 启发式，全量历史只估出 15.7k < `keep_recent_tokens`(20000)，于是切点永远落在第一个合法切点 = 全保留（firstKept = 会话第 3 条）。而真实请求（provider 报 usage.input）33k→101k 一路涨：compact 每轮触发、每轮什么都不丢，上下文冲到窗口的 ~300%。
2. **估算口径缺固定开销 → 占位值失真**。settlement / footer / reserve 闸共用的 `estimate_from_session_entries` 只数 session 消息；system prompt + tools schema（实测 ~9k tokens）不在其内。Api 锚点存在时靠 usage.input 掩盖，锚点失效（compact 后、resume 时、Heuristic 路径）后占位值系统性偏低，用户看到 footer 很低而下一发请求溢出。
3. **wire 丢载荷 → TUI 显示 "Compacted from 0 tokens"**。`Event::CompactionEnd` 在 wire 上不带任何字段，反序列化侧重建为 `result:None / summary:None / tokens_before:None`，remote attach 客户端 bridge 归一成「0 tokens + 空 summary」完成块——展开无任何信息（用户截图缺失的症状）。

## What Changes

- **wire：`Event::CompactionEnd` 携带完整载荷**（`result` / `aborted` / `reason` / `will_retry` / `error_message` / `summary` / `tokens_before`，与 `XyEvent::CompactionEnd` 字段一一对应），`TryFrom<&Event> for XyEvent` 往返保留；remote attach 客户端从此显示真实 tokens 与可展开 summary。纯加字段，向后兼容（旧生产者不再发出——同仓库同版本发布）。
- **估算 SSOT 计入固定请求开销**：`EstimateOpts` 新增 `fixed_context`（system prompt 文本 + tool schemas）；`estimate_from_session_entries` 在 Heuristic / LocalTokenizer 路径把 system prompt + tools 折算进估计；**Api 锚点存在时 MUST NOT 重复计入**（usage.input 已含）。turn-end settlement（TurnSettled）、compact 后 settlement（AfterCompaction）、in-process driver 估计三处接同一输入。
- **remote footer/resume 与 host 同源**：remote driver `estimate_context_tokens` 不再用 GetMessages 拉条目在客户端本地估（既看不到 system prompt，也没有 host 侧 tokenizer 映射），改为新增 unary `Command::EstimateContext` 由 host 计算（含 fixed_context）返回；同时 host 在会话激活/resume 时发一次 `ContextTokenSettlement(reason=leaf_changed)`。
- **切点预算与窗口协调（clamp）**：`prepare_compaction` / `compact_session` 的 keep 预算取 `min(keep_recent_tokens, window − reserve − fixed_overhead)`（window 未知或 0 时退化回 `keep_recent_tokens`，行为不变），杜绝「预算 > 阈值 → 全保留 → 每轮重压」死循环。
- **占位值语义（AfterCompaction settlement）**：compact 成功后重载 leaf（含新 CompactionEntry 与回填的 session_env/agent_todo），以「summary 行 + 保留尾 + system prompt + tools」计算占位估计，经既有 `ContextTokenSettlement` 事件广播 footer / 下一轮 reserve 闸消费；**不主动发起任何模型请求**，重算上下文等下一个用户请求（或 resume 投影）按同一 `build_context_entries` 机制自然生效。

## Capabilities

- `protocol-app`（CompactionEnd wire 载荷往返；Command 枚举新增 EstimateContext）
- `domain-compaction`（c8 切点预算 clamp；c16 触发估计计入固定开销；c26 占位值含固定开销 + AfterCompaction 重算语义）
- `server-core`（EstimateContext unary 分发实现）

## Impact

- 代码：`src/protocol/wire/event.rs`、`src/protocol/lifecycle.rs`（不变，仅 wire 投影）、`src/protocol/wire/command.rs` + server 分发、`src/agent/compaction/{token_estimator,orchestrator,mod,settings}.rs`、`src/agent/runtime/react/{mod,turn_end}.rs`、`src/agent/capabilities/{stats,compact_ops}.rs`、`src/app/core/driver/{remote,in_process,proto,types}.rs`。
- 行为：remote attach 下 compact 块显示真实 tokens/摘要；footer 与 reserve 闸在小窗口模型上显著上修（更早触发 compact）；compact 后保留尾真正受窗口约束，不再出现 300% 溢出的无衰减增长。
- 兼容：wire Event 加字段为可选项式演进（serde Option/缺省），无跨版本迁移；`Command::EstimateContext` 为纯新增方法表项。
- 测试 seam：既有 rust 单测（wire round-trip、compaction 切点/orchestrator、token_estimator 表驱动）+ `tests/bdd/bindings_domain_compaction.rs` 既有 @executable 绑定，不发明新 seam。
