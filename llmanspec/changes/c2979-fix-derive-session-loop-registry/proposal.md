---
depends_on: []
title: 会话槽从 root 注册表派生独立中间件链
needs_specs_change: false
---

# 会话槽从 root 注册表派生独立中间件链

## Why

- `agent-runtime` 的 `r2931`（loop-middleware-session-scope）要求：Host 每个会话槽须持有自 root 派生的独立链；派生须冻结 root 当时的链快照；root 后续注册的中间件不得流入已存在的会话槽。
- 当前实现把 root registry 的 `Arc` 直接交给会话槽，`loop_chain()` 在每次 run 开始时 `snapshot()`，所以 root 后续注册会流入已存在会话。
- `LoopMiddlewareRegistry::derive()` 已实现，但只在 `crates/xylitol-kernel/src/contract.rs` 与 SDK 单测里调用；生产路径从未派生。
- 当前官方中间件都在 boot 时注册，因此没有可见行为差异；但 spec 承诺的“会话隔离 / 派生冻结父快照”实际上不存在，未来一旦开放 per-session 注册就会跨会话泄漏。

## What Changes

- 会话槽创建时从 root registry 派生独立 registry（`factory.loop_registry().map(|r| r.derive())`），并以 `LoopScope::Session` 挂到 writer driver。
- root 后续注册不得影响已创建槽；槽内注册/注销不得回写 root。
- 新增 Host 层测试：两个槽各自派生，root 后续注册不流入；两槽互不可见。
- 保持 reader driver 的 `Root` 作用域行为不变（只读命令不需要会话级链）。

## Capabilities

- `agent-runtime`：`r2931` 已描述目标行为，本次只补实现与测试，不改合约。
- `server-core`：会话槽持有派生链的载体行为（实现层）。

## Impact

- `crates/xylitol-serve/src/occupancy/host.rs`：`SessionSlot` 创建时派生 registry。
- `crates/xylitol-kernel/src/occupancy/factory.rs`：`loop_registry()` 仍是 root 入口；派生在调用方。
- 测试：Host 层或 contract 层的两槽隔离测试。

## Out of Scope

- 新增 per-session 中间件注册 API；本次只保证“派生 + 冻结”语义成立。
- reader driver 的作用域调整。
- 链的观测属性（`loop.middleware` span 已有 `scope`）。

## Further Notes

- 由 PR #30 review F2 记录。
- `derive()` 是浅拷贝（中间件 `Arc` 共享），派生本身很便宜；隔离性来自“条目集合独立”，不是中间件实例复制。
