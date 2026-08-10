---
depends_on:
- c1940-update-multi-api-named-compat
- c1970-update-per-vendor-thinking-levels
summary: "可选 models.dev/本地 JSON 模型目录补全源，用户覆盖合并，补手写 YAML 易漏字段"
---

# 可选 models.dev / 本地 JSON catalog（用户覆盖合并）

> **DELAYED / parked（2026-08-09）**：不做 runtime catalog overlay / refresh / suggest。
> 原因：主线仍是 **YAML 一次性配置**；login/logout 快接全表的 ROI 低（不像 pi 的机制）。
> **Live specs 已丢弃**（未合入 main：无 rc28/ce21/m16）。本目录仅保留 research + 规划块供日后重开。
> Feature 分支 `sdd/c1980-…` 已删除；勿 `change start` 除非产品决定重开。

## Why

多厂商模型条目手写 YAML 可行但易漏（context window、能力旗标、api 形态、thinking 档原料）。[models.dev](https://models.dev) 一类公开 catalog 覆盖面大，适合作为**可选**补全源。但：

- 许多网络环境拉 catalog 需要**专用代理**；与 LLM 网关 HTTP proxy **不是同一条线**。
- 用户需要「下载 → 改几个 key → 本地优先」的覆盖故事；形状应与上游 JSON **同构**。
- 对照 pi：build-time 生成 + runtime store；xylitol **默认 OFF**，显式刷新，缓存可清；未知条目 **suggestion-only**。

c1940 已钉 api×compat；c1970/c1990 已钉配置声明档位与精确 opaque。本 change 原拟把 catalog 收成可选 feedstock，不得自动注册、不得填 STANDARD 五档。

**产品重审**：配置是一次性的；更可能的后置是「作者工具 / codegen → YAML 片段」，而非常驻 merge 层（见 c2000，亦 delayed）。

## What Changes

（原规划，**当前不实施**）

- **默认关闭**：未显式 `catalog.enabled` → 行为与今日一致。
- **数据源优先级**：用户 override JSON → curated/本地 pack JSON → 远程 cache → 无。
- **同构 JSON**：override/pack 与上游 api.json 形状（或文档钉死子集）一致。
- **catalog.proxy**：仅用于 catalog 拉取；禁止硬编码；禁止与 LLM proxy 隐式合并。
- **显式 refresh**：CLI；ETag/缓存；失败保留旧 cache。
- **映射表**：npm/api → xylitol `api`×`compat`（及 thinking_levels **建议**）；未知 → suggestion-only；**禁止**自动 register。
- **非目标**：pi 编译期全表；TUI 内嵌浏览器（后置）；默认 ON；自动注册。

## Capabilities

（原预期；未落地 specs）

- `runtime-config` — `catalog.*`
- `cli-entry` — `catalog refresh|status|suggest`
- `runtime-model-registry` — suggestion-only 边界
- 文档：`docs/architecture/多厂商模型.md`

## Impact

- 暂停：无运行时行为变化。
- 保留：`research/` 官方 api.json 交叉校验与字段统计，供 YAML 对齐 / 未来 codegen 参考。

## Research

见 `research/`：源仓 TOML 统计、`api-json-official-cross-check.md`。**若重开**：refresh SSOT 仍建议仅为 `api.json`。

## Ethics

- risk_level: low
- prohibited_actions: 硬编码代理；默认开启远程拉取；未知条目自动注册；把 catalog JSON 当代码执行
- required_evidence: research 字段统计；proxy 分离样例
- refusal_contract: 不承诺 catalog 完整/时效
- escalation_policy: 重开前须确认是否改为「codegen→YAML」而非 overlay；默认 ON 或自动注册须单独确认
