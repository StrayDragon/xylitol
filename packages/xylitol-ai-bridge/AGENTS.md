# packages/xylitol-ai-bridge

LLM **provider 方言桥** + multi-source **token accounting**。Workspace 库；**零依赖**主 crate `xylitol`。

拆包理由：厂商差异极大，但交付收敛为 **OpenAI-like** / **Anthropic-like**；差异关在本包 adapter 族（**开闭**：新兼容端 = 新 adapter/配置，不改主仓 `AgentMessage` / ReAct）。

| 是 | 不是 |
|---|---|
| 厂商官方 SDK Client 接线（OpenAI：`async-openai`；Anthropic：**暂 reqwest**，官方 Rust SDK 未成熟）+ middleware / 可移植 hooks | ReAct / session / TUI |
| **LLM 投影 DTO**（只表达发给模型的形状）+ usage / accounting | 平行拷贝全量 `AgentMessage`（含 bash/compact/branch/custom） |
| RemoteCount（Anthropic count_tokens、OpenAI Responses input_tokens）优先于本地 tokenizer | 产品 footer 文案；抽第三个 `xylitol-llm-types` |

## 与主仓概念分层（normative）

```text
AgentMessage = Llm(AiBridgeMessage) | Env(…)   （domain 组合 bridge DTO）
        │
        │  project_for_llm → Vec<AiBridgeMessage>（Llm passthrough；Env 折叠）
        ▼
AiBridge* DTO（本包 = LLM 叶 SSOT）
        │
        │  dialect adapter（SDK Client）
        ▼
OpenAI-like / Anthropic-like upstream
```

| MUST | MUST NOT |
|---|---|
| 默认经 **官方 SDK Client** 发请求（OpenAI）；Anthropic 在官方 SDK 成熟前用 reqwest，DTO/hooks 形状不变 | 为每个网关手写第二套完整 HTTP/SSE 栈作默认路径（OpenAI 侧） |
| OpenAI：`async-openai` 仅启用 **`chat-completion` + `responses` + `byot` + `middleware` + `rustls`**（`default-features = false`；TLS 只选 rustls） | 默认拉 `full`；或同时启用 `native-tls` / `native-tls-vendored`；或无故用 `rustls-no-provider`（需自装 crypto provider） |
| OpenAI Responses **流式**：`create_stream_byot::<_, Value>`，按事件 `type` 宽松匹配（兼容端常缺字段） | 把 SSE 默认绑死在 typed `ResponseStreamEvent`（如缺 `created_at` 的 `response.created` 会炸） |
| hooks 经 SDK **middleware**（或文档化等价点）接到可移植 HeaderBag/JSON body | agent/protocol import 本包 `provider` / vendor SDK 类型 |
| DTO **不含** session 环境角色的平行 enum；主仓 MAY `pub use` DTO 组合 `AgentMessage::Llm` | 与 `AgentMessage` 全量孪生 + JSON 往返「对齐」 |
| accounting：Api → RemoteCount → LocalTokenizer → Heuristic | 把 Heuristic 标成 Api；TextDelta 热路径全文 encode |

## 包内模块边界

```text
accounting / tokenize / registry / usage / dto   ← 计量 + LLM DTO
provider / fake                                  ← SDK 接线侧
```

`accounting` / `tokenize` / `registry` **MUST NOT** import `provider::*`。

验证：`cargo test -p xylitol-ai-bridge`；全仓 `just qa`。设计史：`llmanspec/changes/archive/` 下 **c1070-refactor-ai-bridge-sdk-projection**。
