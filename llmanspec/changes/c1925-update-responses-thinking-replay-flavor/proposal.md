---
depends_on:
  - c1880-update-responses-first-api-boundary
  - c1890-add-responses-context-policy-assembler
branch: sdd/c1925-update-responses-thinking-replay-flavor
base_sha: 04a201abb1499a71b67102fb4b2232d50cdb76b6
checkpointed: false
---

# Responses thinking / reasoning：Session JSONL → API body 保真重建

> **调研底稿**：[`docs/research/responses-context-layout-and-cache-2026.md`](../../../docs/research/responses-context-layout-and-cache-2026.md) §5.1（术语对照 §7）
> **书指针**：《深入理解 AI Agent》Ch2——Agent 场景下「思考是状态不是废料」；书语仅经 research §7 映射，**禁止**写入 live specs。
> **对照仓**：姊妹 `../pi`（`packages/ai` Responses 投影）、`../codex`（`codex-rs` `ResponseItem` history → `Prompt.input`）。
> **工程约定（本波次）**：策略默认 **code-first**（`defaults.rs`）；**不**新增 YAML/env 旋钮。用户面 YAML 仅既有字段（如 `api`）。

## Why

Coding agent 多轮 / **resume** 的正确性，取决于能否从导出的 **session JSONL** 重建发给 LLM provider 的 **Responses `input`（及必要 include）**：reasoning 项、assistant message、function_call / output 的类型、id、顺序与可回放材料不能丢，也不能 silently 改写稳定前缀。

回放错误会破坏工具环与轨迹，严重性高于 cache 未命中。今日路径已能「看着正常」resume；本 change **重置**为：**保真重建是主目标**；回放策略 **只有默认全量**（无 Strip/三态旋钮）。

## 对照：pi / Codex / xylitol（策略依据）

| | **pi** | **Codex** | **xylitol（现状）** |
|---|---|---|---|
| Session 真源形状 | pi messages；`thinking` + `thinkingSignature` | **`Vec<ResponseItem>`**（wire 项含 `Reasoning`） | session v5 JSONL → `AgentMessage` / `AiBridgePart`（**偏 pi**） |
| Reasoning 落盘 | `thinkingSignature = JSON.stringify(完整 reasoning item)` | 历史里直接存 `ResponseItem::Reasoning { encrypted_content, summary, content, id }` | 同 pi：signature = 上游 item 整包 JSON 字符串 |
| 重建到 API | `JSON.parse(signature)` → **原样 push** 进 Responses `input` | `history.for_prompt()` → `Prompt.input`（Reasoning 为一等 item） | `project_for_llm` 透传 → `convert_messages_to_input_items`：有 signature 则 parse 后 push |
| `store:false` / ZDR | 请求 `include: reasoning.encrypted_content`；Azure 可在 `response.completed` **回填**缺失的 encrypted | 默认 `include` 含 encrypted；Azure 才 `store:true` | 对齐 pi：thinking 开则 `include`；**无** completed 回填 |
| 规范化 | 省略无 signature 的空 thinking；跨模/abort 时注意 id 配对（`rs_`↔`fc_`） | `normalize_history`：补齐 call/output、剥不支持模态；截断保 encrypted | 无 signature → 省略 Thinking；Aborted 助手不进下一轮 LLM；无 Azure 回填 |

**策略选择（本 change 钉死）**：

1. **继续 pi 形 SSOT**（`AgentPart::Thinking` + 不透明 `thinkingSignature`），**不**把 session 史改成 Codex 式 `ResponseItem` 数组。后者是另一架构（更接近「provider view 即历史」）；与本仓 `AgentMessage`∪`Env` + 日后 `c1930` 分工一致。
2. **主不变量**：JSONL →（resume / 下一轮）→ Assembler 产出的 Responses `input` 中，凡落盘时有合法 signature 的 reasoning，**必须可原样重建**（类型/关键字段/相对顺序不漂）；展示用 `thinking` 文本 **不得**替代 signature 回放。
3. **向 pi 取经（本波应收）**：不透明往返；缺 signature 则省略；流结束材料尽量完整（含 **terminal 事件回填 encrypted**，对齐 pi Azure 坑）。
4. **向 Codex 取经（语义，非搬类型）**：把 reasoning 当 **工具环状态**；`store:false` 路径必须能请求并回放 encrypted；截断/压缩 **禁止**默默丢掉可回放材料；孤儿 call/output 规范化属相邻能力，本波只钉 thinking 缝。
5. **回放策略唯一默认：全量回放**；**不做** Strip/BestEffort；**禁止**伪造 encrypted。

## What Changes

- **规范性（主）**：Session JSONL（v5）中 assistant `content` 的 `type:thinking` + 可选 `thinkingSignature`，经 `project_for_llm` + `ResponsesAssembler`，MUST 重建为正确的 Responses `input` 项序列（reasoning 在同轮 assistant text / function_call 之前；有合法 JSON signature 则 push 解析结果，不得只回放展示文）。
- **落盘（主）**：流式/非流式完整 reasoning item MUST 写入 `thinkingSignature`（整包 JSON）；若 `output_item.done` 缺 `encrypted_content` 而终端 `response.completed`/`incomplete` 带有，MUST 回填后再持久化（对齐 pi）。不支持时不假装写入。
- **本波继续允许**含 `encrypted_content` 的 signature 整包进 JSONL（工具环正确性优先）；文档标敏感；**不**改 `store:true`。
- **回放策略**：**唯一默认 = 全量回放（现状 Preserve）**；不增加模式枚举或 WirePolicy 位。
- **单测 / golden**：（1）有 signature 的 JSONL 形投影 → `input` 含等价 reasoning item；（2）缺 encrypted 时 completed/incomplete 回填；（3）compaction 摘要后不假装旧 signature。
- **与 `c1915` / `c1930`**：链式续跑与 Session↔provider view 仍依赖本保真合约；本 change **不**实现链式，**不**把历史 SSOT 改成 `ResponseItem`。

## Capabilities（意向）

- `package-ai-bridge`（落盘 signature、Assembler 保真重建、回填）
- `agent-runtime`（ReAct 持久化 Thinking；resume 不丢 signature；compaction 边界）

## Out of scope（追加）

## Impact

- resume / 多轮工具调用：从导出 JSONL 可预期重建 Responses body；前缀不因「丢 reasoning / 改顺序」无故断裂。
- 后续 Epoch / 链式以「回放保真合约已定」为前提更安全。

## Out of scope

- `reasoning_replay` / Strip / BestEffort 旋钮（明确不做）
- previous_response_id（→ `c1915`）
- Session SSOT 改为 Codex 式 `ResponseItem` 历史（若需要 → 另案 / 与 `c1930` 协调）
- 状态栏 / MCP search
- Anthropic thinking blocks 真实现（可后置；redacted+signature 直觉可参考）
- 训练数据收集模式下的「拒绝修补」策略

## Parallel / depends

- **硬依赖**：`c1880`、`c1890`（已归档）
- 可与 `c1930` 并行（provider view 叙事）；`c1920`/`c1935` delayed；注意 bridge thinking 文件所有权

## Decisions

1. **主目标**：JSONL → Responses `input` **保真重建**（不丢、不破坏前缀）。
2. **SSOT 形态**：保持 **pi 形**不透明 `thinkingSignature`；不迁 Codex wire-native history。
3. **回放策略（钉死）**：**唯一默认 = 全量回放**（有合法 signature 则原样回放）。**无** Strip/BestEffort/YAML 旋钮。
4. **非法 signature**：**omit + bridge trace/diagnostic**（不崩主路径）（Review C2）。
5. **回填事件**：流式路径处理 `response.completed` **与** `response.incomplete`（对齐 pi）（Review D2）。
6. **空 `encrypted_content`**：**仍整包回放**（不因空串丢弃）（Review E1）。
7. **Compaction**：本波加合约/单测句——摘要替换后 **不得假装**仍持有旧 reasoning signature（Review F2）。
8. **encrypted / JSONL**：本波继续允许整包落盘；不为本波改 `store:true`。
9. **`include`**：thinking 开时保持现状（含 `reasoning.encrypted_content`）。

## Open Questions（lab）

- ~~进程退出后全量回放 cache~~：**已测**（Ornith medium 761 / off 747；见 research §5.2）。隔天另含 system **date** 日界（→ `c1905`）。

## Ethics

- risk_level: medium
- prohibited_actions: 伪造 encrypted 回放；把官方 include 强加给所有 compat；引入 Strip 默默改前缀；用展示用 `thinking` 文本冒充可回放 signature
- required_evidence: JSONL→input 保真 golden；回填或缺材料时不崩主路径且有诊断；lab resume cache（research §5.2）
- refusal_contract: 不承诺一切兼容端可完整回放 thinking
- escalation_policy: 若要引入非全量回放模式或改 session SSOT 为 ResponseItem 史须另开确认波
