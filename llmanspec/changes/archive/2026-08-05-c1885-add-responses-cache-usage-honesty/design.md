# Design: c1885 Responses Prompt Cache 诚实透出

## 目标

在 `c1880` WirePolicy 闸门之上，把 Responses usage 的 Prompt Cache 读数做成**可区分的三态语义**，并透出到 **trace / Langfuse**；**不做** TUI / `prompt_cache_key`。

```text
Responses usage JSON
        ×
WirePolicy.expects_prompt_cache_usage
        → PromptCacheRead { NotApplicable | NotReported | Tokens(n) }
        → 派生 cache_read:u64（仅 Tokens→n，其余 0）
        → ProviderRequestTrace / Langfuse usage_details + 三态属性
```

## 决策（深挖已钉）

| 项 | 选择 |
|---|---|
| 诚实模型 | 三态 provenance（禁止「没字段」=「没命中」） |
| 类型 | 一等枚举（意向名 `PromptCacheRead`）+ 派生 `cache_read` |
| 默认 | `PROMPT_CACHE_USAGE = true`（仅翻该位；`prompt_cache_key` / `previous_response_id` 仍 false） |
| 产品面 | bridge + 观测；TUI → draft `c1940` |
| `prompt_cache_key` | 整段后置 |

## 三态 × 闸门

| `expects_prompt_cache_usage` | JSON 有 `input_tokens_details.cached_tokens` | 无该细节 |
|---|---|---|
| false | `NotApplicable` | `NotApplicable` |
| true | `Tokens(n)`（含 0） | `NotReported` |

## 类型落点

- 放在 `xylitol-ai-bridge` DTO / usage（与 `AiBridgeUsage` 同层或嵌套字段）。
- `AiBridgeUsage.cache_read: u64` **保留**为派生兼容读数，供 accounting / compact 求和；权威语义在枚举。
- Completions / Anthropic 既有映射：本 change **不**强行改其三态（可保持 `Tokens`/`cache_read` 行为等价于今日数字语义）；焦点在 Responses。

## 观测

- `ProviderRequestTrace::attach_usage`：
  - `langfuse.observation.usage_details.cache_read`：**仅**在 `Tokens(n)` 且需要数字时写入（`n==0` 是否写入以实现为准，但 MUST NOT 在 `NotReported`/`NotApplicable` 时写 0 冒充）。
  - 另增三态属性（名以实现为准，如 `xylitol.prompt_cache_read` / `langfuse` 自定义 property）表达 `not_applicable` / `not_reported` / `tokens`。
- OTel 远程出口若已走同一 usage 通道，跟随同一规则；不新开 TUI。

## 合约修订

- 更新 `package-ai-bridge` **pab19**：`prompt_cache_usage` 默认 **true**（另两位仍 false）；同步 `feature: false` 场景期望。
- 新增 req：三态映射；观测诚实（禁止伪造）。
- **不**改 `runtime-config` / YAML。
- TUI chrome：**不**进本 change（`c1940`）。

## 测试 seam（已对齐深挖）

| Seam | 方式 |
|---|---|
| `from_responses_usage_with_policy` / 等价 | 包内单测 fixtures |
| `WirePolicy::default()` | 单测：`prompt_cache_usage==true` |
| `ProviderRequestTrace::attach_usage` | 包内单测：三态属性 / usage_details |
| BDD | **不扩**新 step；toon `feature: false` 文档场景即可 |

## 非目标

- TUI footer（`c1940`）
- `prompt_cache_key` 装配
- Assembler / 状态栏 / tool_search
- Anthropic cache 主路径
- Eval 闸
