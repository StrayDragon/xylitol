# Design: c1880 Responses-first + code-first WirePolicy

## 目标

```text
YAML api（既有）
        ×
代码 WirePolicy { compat, extra_policy }  ← 集中默认板（调试改这里）
        → bridge 适配 /（后继）Assembler、usage
```

交付：**术语 + `api` 边界 + WirePolicy 默认板 + 只读契约**。不实现 Assembler；**不**扩 YAML。

## 术语：第一语言 vs 方言

| 概念 | 定义 | 例 |
|---|---|---|
| **第一语言** | 厂商原生 API | OpenAI / Anthropic / Kimi 官方 |
| **方言** | 他方实现该协议形状 | DeepSeek 实现 `openai-responses` |
| **`api`** | YAML 协议族全称 | 本波唯一相关用户旋钮 |
| **`compat` / `extra_policy`** | 代码内 wire 策略 | 非 agent capabilities |

## 默认板落点：bridge vs infra（已钉 bridge）

| | **A. `packages/xylitol-ai-bridge`（已选）** | **B. 主仓 `infra/provider`（未选）** |
|---|---|---|
| 职责贴合 | wire req/resp、usage、adapter 已在此；策略与实现同仓 | 装配 / ModelMeta / bootstrap 在此，像「产品默认」 |
| 分层 | `agent` 可依赖 bridge **DTO**；Assembler（多半 agent/bridge）能直接读类型 | `agent` **↛** `infra` → Assembler 无法 import infra 默认板，类型仍得外置到 bridge/protocol，infra 只剩重复包装 |
| 测试 | `cargo test -p xylitol-ai-bridge` 即可拧旋钮 | 单测易拖进主仓装配 |
| 透传 | infra 装配：`WirePolicy::default()`（或 `for_api`）注入 adapter | 默认板在 infra，bridge 仍要接收参数 → **两处真相风险** |
| 产品升格 YAML 时 | 主仓解析后 **写入** bridge 的 `WirePolicy` 即可 | 看似近配置，但 wire 语义仍回流 bridge |

**已钉：A（`xylitol-ai-bridge`）**。SSOT 在 bridge；infra **只接线不拥有**默认值。

## WirePolicy（代码，非 YAML）

模块落点（已钉）：`packages/xylitol-ai-bridge`，建议拆：

```text
wire_policy/
  defaults.rs   ← 纯常量 / 字面量；未暴露策略默认值的唯一真源
  mod.rs        ← WirePolicy / ExtraPolicy 类型 + Default（只读 defaults）
```

```rust
// defaults.rs — 调试未暴露默认：只改这一文件（示意）
pub const COMPAT_DEFAULT: Compat = Compat::Generic;
pub const PROMPT_CACHE_USAGE: bool = false;
pub const PROMPT_CACHE_KEY: bool = false;
pub const PREVIOUS_RESPONSE_ID: bool = false;

// mod.rs
impl Default for WirePolicy {
    fn default() -> Self { /* 只引用 defaults::* */ }
}
```

### 约定（本波硬约束）

| MUST | MUST NOT |
|---|---|
| 未暴露 wire 策略默认值只写在 `defaults.rs` | 业务路径散落魔法数 |
| `WirePolicy::default()` 只组合 `defaults` | 本波读 `std::env` / 正式 env 配置面 |
| 单测用结构体字面量覆盖 | 用改全局 env 当单测夹具（本波） |
| 不进 `ModelEntry` / schemars / example 新键 | 把 env 当成第二套 YAML |

**日后**若需要「不重编拧一下」：另开确认，在 **`WirePolicy::default()` 单点**加 debug overlay（明确前缀、非正式、可删）——**禁止**在调用点各自 `env::var`。

**不进 ExtraPolicy**：`tool_search`、`defer_loading`、状态栏等（agent / ContextPolicy；同样各自 `defaults.rs`）。

## 与 `api`（YAML）关系

| 来源 | 字段 |
|---|---|
| YAML / manifest（既有） | `api` → AdapterKind |
| 代码默认板 | `compat`、`extra_policy` |

Adapter **选择**只看 `api`；字段子集 / usage 期望看 `WirePolicy`。

## 运行时流

```text
ModelEntry.api（YAML）
  → AdapterKind
  → OpenAiResponsesAdapter + WirePolicy::default()
  → （c1890）Assembler 读同一 WirePolicy
```

观测可记 `api` + `compat`（从 WirePolicy）；不进 `XyEvent` 闭集。

## 本波工程约定

c1885–c1935：

- 策略默认 **code-first**：集中 **`defaults.rs` 纯常量**，改文件调试。
- **不**新增 YAML 旋钮；用户 YAML 仅既有字段（`api` 等）。
- **不**用环境变量当未暴露配置面（本波）。
- 升格 YAML / 正式 env / debug overlay = 未来独立确认 + change。

## Specs 关系（收窄）

| Spec | 动作 |
|---|---|
| `package-ai-bridge` | WirePolicy + 适配只读 / 降级契约 |
| `infra-provider` | 装配注入默认板；adapter 选择仍按 `api`（既有） |
| `runtime-config` | **不**加新字段 |
| `多厂商模型.md` | 第一语言 vs 方言；code-first 说明 |

## 测试 seam

| Seam | 方式 |
|---|---|
| `defaults` → `WirePolicy::default()` = generic + 全 false | bridge 单测 |
| helper：位关 → 不假装第一语言 wire | bridge 单测 |
| 省略 `api` → openai-responses | 既有 m13/m14 |
| 显式 openai-completions | 编译 + 可选冒烟；不扩 BDD |
| **无** YAML / env 策略加载测 | — |

## 非目标

Assembler、usage 映射、tool_search、YAML 新键、env overlay、厂商 compat 表、providers 表。

## 风险

- 用户不能 yaml 拧 extra_policy：接受；升格另开 change。
- sibling 仍写 flavor/可配置：已加本波 code-first 指针；propose 时再改词。
