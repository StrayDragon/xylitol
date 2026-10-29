---
depends_on: []
title: 在 footer 与观测中透出 prompt cache 命中状态
needs_specs_change: true
branch: sdd/c2994-update-cache-usage-surface
base_branch: main
base_sha: ec2af0456afeb566ae9d6482764ac1140427bc5f
---

# 在 footer 与观测中透出 prompt cache 命中状态

> 来源：GitHub issue #22（上下文缓存与极致压缩）M1 余量。
> 收口约定：实现落地时 commit / PR 文案须含 `fixed #22`，由默认分支合并自动关闭 issue；本规划 commit 只写 `refs #22`，避免提前关单。

## Why

- provider 侧已解析 `PromptCacheRead::{Tokens, NotReported, NotApplicable}`，并在 provider trace 写 `xylitol.prompt_cache_read`。
- 产品 footer 与用户可读观测仍只显示 `used … tokens`，用户无法区分「这次吃到了缓存」与「上游未上报」。
- 长会话的成本判断依赖这一信号；当前是黑盒。

## What Changes

- 扩展 `ContextTokenSettlement`（或等价 settlement 载荷）携带 prompt cache 读取状态：命中 token 数 / 未上报 / 不适用。
- TUI footer 在 `used … tokens` 后追加短字段：命中为 `· cache N`，未上报为 `· cache ?`；不适用时省略，避免噪声。
- 观测出口：provider trace 已发 `xylitol.prompt_cache_read` 三态与 `usage_details.cache_read`；本 change 只校验两族一致，不新增本地观测 CLI。
- 更新 `app-tui-fixed-zone` 的 `r1224` 与 `protocol-app` / `server-core` 的 settlement 事件契约。
- 补协议往返、footer 渲染与未知/不适用分支测试。

## Capabilities

- `app-tui-fixed-zone`：footer token 字段。
- `protocol-app` / `server-core`：settlement 事件载荷。

## Impact

- `crates/xylitol-plugin-sdk/src/settlement.rs` / `model.rs`：settlement 载荷。
- `crates/xylitol-protocol/src/event.rs`：`ContextTokenSettlement` 字段。
- `crates/xylitol-wire/src`：事件映射。
- `crates/xylitol-app-tui/src/{layout,host}`：footer 渲染与刷新。
- `llmanspec/specs/app-tui-fixed-zone/`、`protocol-app/`、`server-core/`。

## Out of Scope

- 不做稳定前缀断点策略（issue M2）。
- 不做动态压缩档（M3）与工具结果分级压缩（M4）。
- 不改 provider HTTP 请求体。

## Open Questions

- footer 字段名用 `cache N` 还是 `cache-read N`？倾向 `cache N`，与 `used` 同层级。
- 是否把 `cache ?` 也写入 status 区，还是仅 footer？倾向仅 footer，避免状态区噪声。
