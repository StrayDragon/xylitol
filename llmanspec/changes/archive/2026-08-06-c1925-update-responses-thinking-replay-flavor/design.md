# Design: c1925 Session JSONL → Responses input 保真重建

## 目标

把「从导出 session JSONL 重建正确的 Responses `input`（及必要 `include`）」立为可验证合约：不丢 reasoning 可回放材料、不破坏稳定前缀。

**回放策略只有一个默认：全量回放**——有合法 `thinkingSignature` 则原样 push；**无** Strip/BestEffort 旋钮。本波补齐 pi 已修过的 **terminal encrypted 回填**，并加 golden。

## 书 / 对照仓边界

| 来源 | 用什么 | 不用什么 |
|---|---|---|
| 书 Ch2 | 「思考是状态」直觉 | 书语进 live specs |
| **pi** `openai-responses-shared.ts` | 不透明 `thinkingSignature`；`output_item.done` 落签；`response.completed` **回填** `encrypted_content` | 搬 TS 类型 / Completions 多厂商 marker 全家桶 |
| **Codex** `ResponseItem::Reasoning` + `for_prompt` | reasoning 为一等 API 项；`store:false` 必能要回 encrypted；截断勿丢 | 把 session SSOT 改成 `Vec<ResponseItem>`（→ 另案 / `c1930`） |

## 架构选择（钉死）

```text
Session JSONL v5
  message.content[]  type=thinking + thinkingSignature?<opaque JSON string>
        │  load / resume
        ▼
AgentMessage / AiBridgePart   （pi 形 SSOT；Llm 透传）
        │  project_for_llm
        ▼
ResponsesAssembler → input[]
  Thinking+合法 signature → parse 后原样 push（排在同轮 text / function_call 前）
  无 signature / 非法 JSON → 省略该 Thinking（不并入 output_text）
```

**不**迁 Codex wire-native history。Assembler 仍是唯一 body 布局入口（c1890）。

## 落点

| 能力 | 层 | 说明 |
|---|---|---|
| signature 落盘 + **completed/incomplete 回填** | `xylitol-ai-bridge` `openai_responses` | SSE 状态机 |
| `input[]` 全量保真重建 | 同包 `convert_messages_to_input_items` + Assembler | 无回放模式枚举 |
| 持久化 Thinking 部分 | `agent` ReAct | 已支持二次 `ThinkingEnd` 覆盖 signature |

禁止：`agent` 手拼 reasoning item；用展示文冒充 signature；YAML/env 新旋钮；`ExtraPolicy.reasoning_replay` 分叉。

## SSE 回填挂点（相对 pi）

pi 时序：

```text
output_item.done(reasoning)  → thinkingSignature = JSON.stringify(item)
                               reasoningBlocksById.set(id, block)
response.completed|incomplete → backfillReasoningSignatures(response.output)
  若 item.encrypted_content 有值且 stored 缺 → 合并写回 signature
```

xylitol **现状**：

```text
output_item.done(reasoning)  → ThinkingEnd { thinking, thinking_signature: item.to_string() }
response.completed           → 仅解析 usage → Done   ← 无回填、无按 id 索引
```

ReAct 已是：`ThinkingEnd` 若 `sig.is_some()` 则 **覆盖** `thinking_signature`（可二次到达）。因此回填 **不必**新 chunk 变体。

### 推荐实现（本波）

1. 扩展 `ResponsesStreamState`：`reasoning_items_by_id: HashMap<String, Value>`（或等价）。
2. `output_item.done` + `type=reasoning`：发 `ThinkingEnd`；按 `item.id` 存入 map。
3. `response.completed` + `response.incomplete`：合并非空 `encrypted_content` → 再发 `ThinkingEnd` → 再 `Done`。
4. 无终端事件 / 无 id / 无可补 encrypted：保持 done 材料，**不**伪造。
5. **非流式**：最终 `output` 一次写全 → 无需回填。

### 挂点文件（意向）

- 主改：`packages/xylitol-ai-bridge/src/provider/openai_responses.rs`
- 消费确认：`src/agent/runtime/react.rs`
- 重建：`convert_messages_to_input_items`（全量回放，无模式闸）
- 合约：`llmanspec/specs/package-ai-bridge`；compaction 边界见 agent-runtime / domain-compaction

## 回放策略（唯一默认）

| | 行为 |
|---|---|
| **全量回放（默认=唯一）** | 合法 `thinkingSignature` → 原样 push；非法 → omit + diagnostic |
| Strip / BestEffort / YAML 旋钮 | **不做** |

`include: reasoning.encrypted_content`：thinking 开时保持现状。空 `encrypted_content` 仍整包回放。

### 用户痛点

> 今天建 session、聊几轮；退出 / 隔天 resume，**不应无故破坏**已构筑的前缀缓存。

本波用 **全量回放保真** 守住 reasoning 前缀。date 日界 → `c1905`。

### Lab（维护脚本，不进 qa）

`cargo run -p xylitol-ai-bridge --example lab_resume_prompt_cache`
可选：`XYLITOL_LAB_THINKING=off|medium`

**2026-08-06 Ornith/llama.cpp**（`cache_read`）：

| 条件 | warm1 | warm3 | resume 全量回放 |
|---|---|---|---|
| thinking=medium | 0 | 727 | **761** |
| thinking=off | 0 | 710 | **747** |

早期 Strip 对照曾到 404（否决分叉证据，已不进脚本）。详见 research §5.2。

## 保真 / 前缀纪律

- 同轮顺序：**reasoning → assistant message(text) → function_call**。
- 不得把 Thinking 文本并入 `output_text`。
- Compaction：摘要替换后 **不得假装**仍有旧 signature（合约/单测）。

## 测试缝

| 缝 | 方式 |
|---|---|
| JSONL 形 parts → `input` | 包内 golden |
| SSE 回填 | 合成：done 无 encrypted → completed 有 → 第二次 ThinkingEnd |
| 非法 signature | omit + diagnostic，不 panic |
| Compaction | 摘要后无旧 signature 可回放 |

## Out of scope

- 回放模式枚举 / Strip / BestEffort
- `previous_response_id` / `store:true`
- Session SSOT → `ResponseItem` 史
- Anthropic thinking 真实现

## Ethics 映射

- 禁止伪造 encrypted；禁止展示文冒充 signature。
- 证据：保真 golden + 回填合成 SSE + lab resume cache。
