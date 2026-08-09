---
depends_on:
- c1980-add-optional-models-dev-catalog
summary: "维护脚本把 models.dev 生成 curated pack JSON（可选 Rust 静态表）；后置，不默认编进二进制"
---

# models.dev → 生成 curated pack / 可选 Rust 静态表（pi 式 codegen，后置）

> **DELAYED / parked（2026-08-09）**：与 c1980 一并推迟。YAML 主线不变；是否做「codegen→YAML 片段」另议。
> 仅保留草案块；无 design/tasks、无 live specs、未 attach。

## Why

### 产品张力

[models.dev](https://models.dev) 的 `api.json` 是多厂商模型元数据的公开全表（provider → nested models）。原 c1980 方向（runtime 可选 catalog）已 **parked**。

若未来要：

- 离线精选 feedstock（不必 runtime refresh）
- 发布节奏可控（`just gen` + PR）
- 把「映射表 + 过滤 + thinking 建议」固化成 **可粘贴进 YAML** 的产物

则需要 **维护期生成**，而不是常驻 overlay。login/logout 式全表快接对「一次性 YAML」ROI 偏低。

### 对照 pi（事实，非追平义务）

`../pi` 的 models.dev 用法是 **编译期 codegen**，不是 runtime overlay：

| 层 | pi | xylitol |
|---|---|---|
| models.dev 数据源 | **只** `fetch(api.json)` | 可作 YAML 作者原料（c1980 overlay parked） |
| 生成物 | `*.models.ts` + 可选 JSON catalog | 无 codegen；候选见本草案 |
| 用户覆盖 | `~/.pi/agent/models.json`（同名异物） | YAML `ModelEntry` SSOT |

### 调研锚点（已有证据，勿重做空转）

c1980 `research/`：官方交叉校验与字段统计仍有效。

故本草案只锚定 **后置候选**；与 c1980 overlay 一并 delayed。

## What Changes

- **维护脚本生成 curated pack JSON**（推荐默认候选）：输入锁定 `api.json` + include/exclude / npm→api×compat 映射；输出与 c1980 pack 同构 JSON；`just gen-models-dev-pack`（或等价）；**不**在 `cargo build` 热路径强制拉网。
- **可选 Rust 静态表**（需单独产品拍板）：`include_str!` / 小子集 const 等；**禁止**默认把全量 ~6231 模型编进二进制除非明确接受体积；仍 MUST NOT 自动 `register`。
- **文档与闸**：生成物校验；说明生成 pack ≠ c1980 remote refresh。
- **非本草案范围**：runtime catalog CLI（c1980）；TUI 浏览；默认 ON；自动注册；JSON 当代码执行；追平 pi R2 拓扑。

## Capabilities

- 可能：维护脚本 / `just`；可选示例 pack（`infra`/`docs`）
- 可能不改 `runtime-model-registry` MUST（若仍 suggestion-only）
- 若 bake-in 改变未配置可观察模型集 → 正式化时须 triage specs

## Impact

- **收益**：离线精选 feedstock、映射可审查、更新可走 PR；衔接 c1980 pack。
- **成本**：维护生成器与映射表；RS 全表则体积/CI 上升。
- **风险**：过时、精选当全表、与用户 YAML 优先级纠缠。

## Open decisions

1. 产物：**仅 JSON pack** vs **JSON + 可选 RS 子集** vs **RS 为主**？
2. 精选：白名单 provider / 仅 openai-compatible / 手工 seed？
3. 更新：人工 `just gen`+PR vs CI 定时（网络策略；勿把本机 egress 写入产品配置）？
4. 与 c1980：仓库示例默认路径 vs 仅文档？
5. thinking：生成时写建议 `thinking_levels`（`off`+effort.values）vs 保留上游 `reasoning_options`？

## Research

- c1980：`llmanspec/changes/c1980-add-optional-models-dev-catalog/`（design 对照 pi、research 交叉校验）
- pi：`packages/ai/scripts/generate-models.ts`（api.json）；`scripts/publish-model-catalog.mjs`（自有发布物）
- 架构：`docs/architecture/多厂商模型.md`

## Ethics

- risk_level: low
- prohibited_actions: 硬编码本机/用户代理进产品默认配置；未确认就默认 ON 或自动 register；把未审查全表编进默认二进制；把生成 JSON 当可执行代码
- required_evidence: c1980 官方 api.json 交叉校验仍有效；生成器输入/过滤规则可审查；体积与更新策略有书面选择
- refusal_contract: 不承诺生成表完整/时效；不承诺与 pi catalog 行为一致
- escalation_policy: 任何「bake-in 改变未配置默认可调用模型集」或「CI 自动拉网写回 main」须用户确认后再 propose
