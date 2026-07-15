# packages/xylitol-ai-bridge

LLM **provider bridge** + multi-source **token accounting**。Workspace 库；**零依赖**主 crate `xylitol`。

| 是 | 不是 |
|---|---|
| OpenAI / Anthropic 方言 HTTP/SSE → `AiBridge*` DTO | ReAct / session / TUI |
| usage 归一化 + accounting（Api→RemoteCount→LocalTokenizer→Heuristic） | 产品 footer 文案（见 c1035） |
| OpenAI tiktoken builtin（`tiktoken-rs`）/ HF tokenizer.json（下载 **opt-in**）；Anthropic **无**本地 vocab；RemoteCount：Anthropic `count_tokens` + OpenAI Responses `input_tokens`（c1060） | 再抽 `xylitol-llm-types`；再套一层 `AgentMessage` newtype；已弃用的 `claude-tokenizer` |

> 类型方向（产品偏好）：主仓宜 **直接依赖** 本包 `AiBridge*` 作为可控契约，消灭双类型 JSON 往返；**不要**再增加 `AgentMessage` 嵌套。
> 计量优先级：Api → RemoteCount → LocalTokenizer → Heuristic；本地 count 不能替代 API。


## 包内模块边界（normative）

```text
accounting / tokenize / registry / usage / dto   ← 计量侧
provider / fake                                  ← 接线侧
```

| MUST | MUST NOT |
|---|---|
| `accounting` 只认 `AiBridge*` + 注入的 RemoteCount / tokenizer 回调 | `accounting` → `provider::*`（含 HTTP/SSE adapter） |
| 流末 / 响应里的厂商 usage = accounting 的 **Api** 最高优先输入 | 把 Heuristic 标成 Api；TextDelta 热路径全文 `tokenizer.encode` |
| 主仓 agent **可以**依赖本包的 `accounting`（+ DTO） | agent / domain 直接依赖 `provider` 的 vendor/HTTP 类型 |

主仓映射：`src/infra/provider/map.rs`（`AiBridge*` ↔ `AgentMessage` / `XyChunk` / `XyUsage`）。
Driver 只读缝：`Driver::estimate_context_tokens`（无 footer UI）。

验证：`cargo test -p xylitol-ai-bridge`；全仓 `just qa`。
