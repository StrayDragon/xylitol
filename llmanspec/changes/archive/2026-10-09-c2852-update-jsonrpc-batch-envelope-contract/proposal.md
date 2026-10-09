---
depends_on: []
needs_specs_change: true
branch: sdd/2026-10-review-fixes
base_branch: main
base_sha: c10be4e9983564fac14b797ff9c6defaff15c4cf
---

# JSON-RPC batch 数组请求判非法信封——契约显式化

## Why

c2850 前序（c2835，jsonrpsee 退役）把手写 `dispatch_raw` 的等价判据锚在单请求形态：数组 batch（`[{…},{…}]`）在新分发下落入「缺 `jsonrpc` 成员」分支 → 非法信封（HTTP 400）。jsonrpsee 时期 `raw_json_request` 原生支持 batch，这是**未声明的语义收窄**——设计文档 D2 等价判据未覆盖此项。产品客户端（TUI remote driver、库嵌入）全为 unary 单请求，batch 无产品消费者；且 batch 的幂等键（每元素一 id）与写者租约（每调用一准入）语义从未被定义。

## What Changes

1. `server-core` 追加规则 **r1928「JSON-RPC batch 判非法信封」**：JSON-RPC 2.0 文本调试通道只承载单请求对象；数组 batch 请求 MUST 判非法信封（与缺 method/载体版本不符同 `-32600` / `illegal_envelope` 语义，HTTP 400），MUST NOT 静默支持（batch 的幂等键与写者租约语义未定义）。
2. `rpc_module.rs` 单测补 batch 数组 → `dispatch_raw` 返回 `None` 判否（当前实现已如此，测试锁行为）。

## Capabilities

- `server-core`（JSON-RPC 调试通道分发契约）

## Impact / 风险

- 对 v2 时代假想 batch 客户端是 breaking——协议版本已硬切 3（attach 预检硬等值），无兼容承诺受众。
- v3 二进制载体（fory ClientRequest）无 batch 形态，不受影响。
