---
depends_on:
- c1940-update-multi-api-named-compat
---

# 可选 models.dev / 本地 JSON catalog（用户覆盖合并）

> **草案 only**：本 change **不得**在当前 c1940 分支上实现；待 `c1940-update-multi-api-named-compat` 完成后再考虑 `ff` / `propose`。
> **产品定位**：models.dev（`catalog.json` / `models.json` / `api.json` 等）是**可选原料**，不是运行时硬依赖；xylitol 仍是个人 harness——远程 catalog = feedstock，不是 pi 式 build-time 生成 + runtime store 的必经路径。

## Why

多厂商模型条目手写 YAML 可行但易漏（context window、tokenizer、能力旗标、api 形态）。[models.dev](https://models.dev) 一类公开 catalog 覆盖面大，适合作为**可选**补全源。但：

- 许多网络环境拉 catalog 需要**专用代理**；这与打 LLM 网关的 HTTP proxy **不是同一条线**——不能把用户的 LLM proxy 硬塞进 catalog 拉取，也不该在代码里写死某代理。
- 用户需要「下载 → 改几个 key → 本地优先」的覆盖故事；形状应与上游 JSON **同构**，方便 copy-paste 单键覆盖。
- 仓库/发行侧也可能提供 **curated pack**（精选子集）：同样必须是 **JSON**（不要 YAML pack），以便与上游/用户覆盖同一套合并逻辑。
- 对照 pi：build-time 生成 + runtime store；xylitol 不走那条——默认 **OFF**，显式刷新，缓存可清。

c1940 已把「api×compat 显式接线、勿 URL 自动探测大表」钉成主线；本草案是其后置的 **可选 catalog overlay**，且未知条目只建议、不自动注册进可用模型表。

## What Changes

验收向意向（正式 propose 时再落 specs）：

- **默认关闭**：未显式启用时，行为与今日一致——只认用户 YAML/配置里的模型条目；不因缺远程 catalog 而失败启动。
- **可选数据源优先级（高 → 低）**：
  1. 用户 override JSON（本地路径；可只含要改的 key）
  2. curated / 本地 pack JSON（发行或用户放置的精选包，**JSON 而非 YAML**）
  3. 远程拉取后的本地 cache（ETag / 条件请求）
  4. 无 catalog（仅用户模型配置）
- **同构 JSON**：override 与 curated pack 使用与上游相同（或文档钉死的子集）JSON 形状，允许用户从 `models.json` 等直接复制单条/单键修改。
- **catalog 专用代理**：配置 / CLI / env 提供 `catalog.proxy`（名称以正式方案为准）——**仅**用于 catalog 拉取；与 LLM 请求代理分离；**禁止**硬编码用户代理地址。
- **显式刷新**：提供 CLI（或等价命令）触发 refresh；支持 ETag/缓存落在配置目录下；失败时保留旧 cache 并给出可理解错误，不污染模型表。
- **映射表**：models.dev（或 pack）字段 → xylitol `api`×`compat`（及少量 meta）经**小而显式**的映射表；无法识别的条目 → **suggestion-only**（提示/候选），**禁止**自动注册为可调用模型。
- **非目标**：不在本分支实现；不做 pi 式编译期全表生成；不把远程 catalog 变成 Trust/权限面；不把 catalog 代理与 LLM 代理合并成一个隐式全局 proxy。

## Capabilities（意向）

正式化时可能触及：

- `runtime-config` — `catalog.*`（enable、proxy、paths、cache）默认 off
- `cli-*` — refresh / 缓存状态 / 建议列表
- `runtime-model-registry` / `infra-provider` — 合并结果只作 feedstock；注册仍要用户显式接纳或映射命中
- 文档：`docs/architecture/多厂商模型.md` 中「后置 models.dev」条目落地说明

## Impact

- **用户**：可选用公开 catalog 补全；可本地改 JSON 覆盖；网络差时可只靠 curated pack 或纯 YAML。
- **运维/隐私**：catalog 流量可走独立代理；不强迫经 LLM 通道出网。
- **实现**：多一层可选 I/O + 合并；必须严格与 c1940 的 api×compat 手工/显式哲学兼容——catalog 不得变成「URL 自动探测大表」。
- **对照 pi**：pi = build-time + runtime store；xylitol = 可选远程/本地 JSON 原料 + 用户覆盖；默认不拉网。

## 依赖与时机

- **depends_on**：`c1940-update-multi-api-named-compat`（先稳定 api×compat 与模型条目语义，再谈 catalog→xylitol 映射）。
- **本分支**：仅本草案 `proposal.md`；**禁止**实现、tasks、design、specs、attach。
- **下一步**：c1940 归档后评估是否 `ff`/`propose`；映射表与 models.dev schema 漂移策略在 propose 时钉。

## Open Questions

- 启用开关粒度：全局 `catalog.enabled` vs 按 pack 启用？
- suggestion-only 的产品出口：CLI 列表、TUI 提示、还是只写日志？
- curated pack 由谁维护、发布路径（仓库内 `packs/*.json` vs 用户目录）？
- models.dev 多文件（catalog/models/api）合并键空间如何钉死，避免 override 歧义？

## Ethics

- risk_level: low（可选元数据；不执行远程代码）
- prohibited_actions: 本分支实现；硬编码代理；默认开启远程拉取；未知条目自动注册为可调用模型；把 catalog JSON 当任意代码执行源
- required_evidence: 正式 propose 前确认 models.dev 文件形状与映射表草案；代理与 LLM proxy 分离的配置样例
- refusal_contract: 不承诺 catalog 完整/时效；不承诺兼容端字段 ≡ 官方计费或能力
- escalation_policy: 若要把远程 catalog 改为默认 ON，或自动注册模型，须单独用户确认
