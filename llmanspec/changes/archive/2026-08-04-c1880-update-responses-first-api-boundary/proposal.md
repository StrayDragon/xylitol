---
depends_on: []
branch: sdd/c1880-update-responses-first-api-boundary
base_sha: e1acc4bd196d2a87f71dae37174e64e99669838c
checkpointed: true
checkpoint_sha: e1acc4bd196d2a87f71dae37174e64e99669838c
---

# Responses 默认主路径 + Completions 显式类型 + Anthropic 桩

> **调研底稿**（非本 change）：[`docs/research/responses-context-layout-and-cache-2026.md`](../../../docs/research/responses-context-layout-and-cache-2026.md)（§5.1；旧称 `flavor` → **`compat`**）
> **工程约定（本波次）**：`compat` / `extra_policy` = **code-first**：`xylitol-ai-bridge` 内 **`defaults.rs` 纯常量**；**不**新增 YAML；**本波不做** env overlay。用户面 YAML **仅保留既有** `api` 等字段。
> **自包含**：本草案单独可认领；不依赖其它未归档 change。

## Why

要把上下文布局、Prompt Cache、工具披露等优化做到可配置、可复现，必须先钉死：**主交付只围绕 `openai-responses`（含兼容端）**。若继续把 Anthropic Messages / `openai-completions` 与 Responses **不同协议族**揉进同一套「断点与布局」假设，Assembler 会拖成不可维护的中间层。

同时完全删掉 Completions 有耦合风险。折中：**Completions 保留为显式 YAML `api: openai-completions`**；Anthropic **留桩**不进默认开箱反设计。

另一易忘坑：**方言端点形似第一语言 ≠ 语义等价**。须有与 `api` 正交的 **`compat` + `extra_policy`（仅 API req/resp）**，禁止「凡 `openai-responses` ≡ OpenAI 官方」。

若本波（c1880–c1935）把每个旋钮都做成 YAML：serde / schema / bootstrap / 单测矩阵会先于行为爆炸。个人 harness 调试默认值应 **改一处代码重跑**。故本 change 交付 **代码内 Wire 默认板 + 只读契约**；不扩配置面。

### 术语：第一语言 vs 方言

| 概念 | 含义 | 例 |
|---|---|---|
| **第一语言** | 厂商**自己的**原生 API | OpenAI 官方 Responses/Completions；Anthropic Messages；Kimi 官方 API |
| **方言** | 他方实现/兼容某一第一语言的协议形状 | DeepSeek / llama.cpp / 网关实现 `openai-responses` |
| **`api`** | YAML 可选的协议族字符串（全称；**本波唯一相关用户旋钮**） | `openai-responses` / `openai-completions` / `anthropic-messages` |
| **`compat`** | 代码内兼容策略档（方言端尤重要） | 首版常量 `generic` |
| **`extra_policy`** | 代码内、仅 API req/resp 的布尔策略 | `prompt_cache_usage` 等；非 agent 能力 |

叙事用「方言 / 第一语言」；配置键不用 `dialect`（避免与旧「dialect≈adapter」撞名）。

## What Changes

- YAML：`models.*.api` 省略时 OpenAI → **`openai-responses`**；显式允许 **`openai-completions`**。全称；禁止真值简写。Completions 回归：**仅编译 + 冒烟**。
- Anthropic：保留模块；**禁止**为 `cache_control` 反设计 Responses Assembler。
- **代码内 Wire 默认板**（落点 **`packages/xylitol-ai-bridge`**；infra 只注入）：
  - 形态：**纯 `defaults.rs`（或等价）常量 / `Default`**——未暴露策略的默认值 **只**出现在此文件；业务路径禁止散落魔法数或私自读 env。
  - `compat = generic`；`extra_policy` 三 wire 位默认 **全 false**。
  - **禁止**把 `tool_search` / `defer_loading` 等 agent 策略塞进此板（→ `c1900` 等，同样 code-first + defaults 文件）。
  - 测试可构造 `WirePolicy { .. }` 覆盖；**无** YAML / schema / ModelEntry 新字段；**本波不做**环境变量 overlay（日后若要，须单点读入且标明 debug，另开确认）。
- 适配层契约：按 `api`（YAML 既有）× `WirePolicy`（defaults）做 req/resp 字段子集、usage 期望、降级。
- 产品文：第一语言 vs 方言；主轴 `openai-responses`；本波策略不进 YAML / 不进正式 env 配置面。

## Capabilities（意向）

> SDD capability specs 意向；非配置键名。

- `package-ai-bridge`（WirePolicy 默认板 + 适配只读契约）— **主**
- `infra-provider`（装配时把 `api` 与默认板交给 adapter；**不**解析新 YAML 字段）
- 产品叙事：`多厂商模型`（归档时迁）
- **不**为本 change 扩 `runtime-config` 新字段 req

## Impact

- 开发者调 wire 默认：改默认板一处 → 编译验证；无配置矩阵 gap。
- 用户仍用既有 `api` 选协议族 / Completions 逃生。
- 后继 change（usage / Assembler / 链式）**读 WirePolicy**，不各自发明 YAML 旋钮（本波约定）。

## Out of scope

- `models.*.compat` / `extra_policy`（及任何本波新 YAML 策略键）
- ContextPolicy / Assembler 全量（→ `c1890`；其策略档同样 code-first）
- cache usage 映射（→ `c1885`）
- tool_search / defer_loading（→ `c1900`）
- 厂商 `compat` 专档表、自动探测、`providers:` 表
- 物理删除 Completions / Anthropic 源文件
- 把默认板升格为用户 YAML 或正式 env 配置面（另开 change）
- 本波实现 env overlay / `XYLITOL_*` 策略开关

## Parallel / depends

- `depends_on: []`
- 可与 `c1885` 并行；`c1890` / `c1900` / `c1915` / `c1925` 依赖本 change

## Decisions（已钉死）

| 项 | 决定 |
|---|---|
| 用户 YAML | **仅既有 `api`（等）**；本波 **不**加 `compat`/`extra_policy` |
| Wire 策略 | **code-first**：`defaults.rs` 纯常量 / `Default`；测试可注入 |
| 环境变量 | **本波不做**；禁止业务路径散落 `env::var` 当影子配置 |
| `compat` | 常量 **`generic`** |
| `extra_policy` | 三 wire 位默认 false；不含 tool_search |
| 概念 | 第一语言 vs 方言（见上表） |
| `api` 字面量 | 全称 `openai-responses` / `openai-completions` |
| Completions 回归 | 仅编译 + 冒烟 |
| 默认板落点 | **`packages/xylitol-ai-bridge`**；infra 仅装配注入 |

## Ethics

- risk_level: medium
- prohibited_actions: Anthropic 断点反设计 Responses；静默把 Completions 当 Responses 优化；方言≡第一语言；为本波堆 YAML 策略旋钮；agent 策略混进 `extra_policy`
- required_evidence: 默认 `openai-responses`；显式 Completions 可选；WirePolicy 默认板可定位修改；无新 YAML 字段
- refusal_contract: 不承诺凡 Responses 行为一致；不承诺本波用户可 YAML 拧满策略
- escalation_policy: 升格 YAML 或物理删源文件须单独确认
