---
depends_on:
- c1940-update-multi-api-named-compat
- c1970-update-per-vendor-thinking-levels
---

# 可选 models.dev / 本地 JSON catalog（用户覆盖合并）

## Why

多厂商模型条目手写 YAML 可行但易漏（context window、能力旗标、api 形态、thinking 档原料）。[models.dev](https://models.dev) 一类公开 catalog 覆盖面大，适合作为**可选**补全源。但：

- 许多网络环境拉 catalog 需要**专用代理**；与 LLM 网关 HTTP proxy **不是同一条线**。
- 用户需要「下载 → 改几个 key → 本地优先」的覆盖故事；形状应与上游 JSON **同构**。
- 对照 pi：build-time 生成 + runtime store；xylitol **默认 OFF**，显式刷新，缓存可清；未知条目 **suggestion-only**。

c1940 已钉 api×compat；c1970/c1990 已钉配置声明档位与精确 opaque。本 change 把 catalog 收成可选 feedstock，不得自动注册、不得填 STANDARD 五档。

## What Changes

- **默认关闭**：未显式 `catalog.enabled` → 行为与今日一致。
- **数据源优先级**：用户 override JSON → curated/本地 pack JSON → 远程 cache → 无。
- **同构 JSON**：override/pack 与上游 api.json 形状（或文档钉死子集）一致。
- **catalog.proxy**：仅用于 catalog 拉取；禁止硬编码；禁止与 LLM proxy 隐式合并。
- **显式 refresh**：CLI；ETag/缓存；失败保留旧 cache。
- **映射表**：npm/api → xylitol `api`×`compat`（及 thinking_levels **建议**）；未知 → suggestion-only；**禁止**自动 register。
- **非目标**：pi 编译期全表；TUI 内嵌浏览器（后置）；默认 ON；自动注册。

## Capabilities

- `runtime-config` — `catalog.*`
- `cli-entry` — `catalog refresh|status|suggest`
- `runtime-model-registry` — suggestion-only 边界
- 文档：`docs/architecture/多厂商模型.md`

## Impact

- 用户可选用公开 catalog / 本地 pack；网络差可靠 YAML。
- 实现多一层可选 I/O + 合并；必须兼容 api×compat 显式哲学。

## Research

见 `research/`：源仓 TOML 全量字段统计（`source-toml-field-stats.json`）、schema/样例、对照表。官方 `api.json` 待 catalog.proxy 交叉校验。

## Ethics

- risk_level: low
- prohibited_actions: 硬编码代理；默认开启远程拉取；未知条目自动注册；把 catalog JSON 当代码执行
- required_evidence: research 字段统计；proxy 分离样例
- refusal_contract: 不承诺 catalog 完整/时效
- escalation_policy: 默认 ON 或自动注册须单独确认
