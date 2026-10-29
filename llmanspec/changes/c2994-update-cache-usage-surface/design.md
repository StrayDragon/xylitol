# Design

## Context

- SDK 已有 `PromptCacheRead::{Tokens(u64), NotReported, NotApplicable}`，OpenAI usage 解析与 provider trace 已消费。
- `ContextTokenSettlement` 目前只有 `tokens / provenance / usage_tokens / trailing_tokens / last_usage_index`；footer 与观测拿不到 cache 状态。
- protocol `Event::ContextTokenSettlement` 是跨进程唯一载荷；TUI attach 必须经它，不能直读 provider。

## Decision

### 1. SDK 载荷

`ContextTokenSettlement` 增加 `prompt_cache_read: PromptCacheRead`（`Default` 为 `NotApplicable`）。`XyEvent::ContextTokenSettlement` 同步增加该字段；settlement 生成点（turn_end / compact_ops）从最近一次 usage 取状态，无 usage 时保持 `NotApplicable`。

### 2. protocol / wire

protocol 不依赖 SDK，事件载荷用可序列化形状：

```
ContextTokenSettlement {
  ...,
  prompt_cache: Option<String>,      // "hit" | "unknown" | "not_applicable"
  prompt_cache_tokens: Option<u64>,  // 仅 hit 时非空
}
```

`xylitol-wire` 负责 SDK enum ↔ protocol 字符串映射；未知字符串按 `unknown` 处理，不得 panic。

### 3. footer

`r1224` 在 `used … tokens` 后追加：

| 状态 | footer 片段 |
|---|---|
| `Tokens(n)` | ` · cache n` |
| `NotReported` | ` · cache ?` |
| `NotApplicable` / 缺失 | 省略 |

字段序保持在 `used` 之后、占用比之前；放不下时仍按既有右侧截断。`NotApplicable` 不写 `cache —`，避免在无 prompt cache 的 provider 上制造噪声。

### 4. 观测

provider trace 已发 `xylitol.prompt_cache_read`（`tokens` / `not_reported` / `not_applicable`）与 `usage_details.cache_read`（仅 Tokens）；OpenAI / Anthropic 两族实现一致。本 change 只补两族一致性单测，不新增本地观测 CLI，也不让 footer 与 trace 各自解析 usage：二者都从 settlement / usage 的同一状态派生。

## Alternatives

- **B1. footer 直接读 provider usage**：attach 端拿不到 provider 内部状态，破坏端/宿主边界。
- **B2. 只加 `cache_read_tokens: Option<u64>`**：无法区分「未上报」与「不适用」，不满足 issue 的诚实三态。
- **B3. 在 status 区常驻 cache 状态**：噪声大；footer 已足够。

## Risks

- protocol 事件新增字段属于向后兼容（`#[serde(default)]`）；旧客户端忽略即可。
- 部分网关不返回 cache usage，footer 显示 `?` 是预期诚实行为，不得伪造成 0。
- footer 字段增加可能挤压 thinking/占用比；按既有截断规则处理。

## Test Seams

- protocol 事件 JSON 往返（含三态）。
- wire 映射 SDK enum ↔ protocol 字符串。
- footer 渲染三态与省略。
- `obs-summary` 读取属性（fixture）。
