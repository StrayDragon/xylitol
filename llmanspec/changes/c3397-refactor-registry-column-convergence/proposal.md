---
title: wire 方法表零消费者列收敛（registry 方向 B）
depends_on: []
needs_specs_change: false
---

# wire 方法表零消费者列收敛（registry 方向 B）

## Why

`crates/xylitol-protocol/src/registry.rs` 的 `MethodEntry` 有六列：`name` / `auth` / `idem` / `resp` / `command_backed` / `exec`。其中两列**零产品消费**：

- `Resp`（`Result` / `Stream` / `Job`）：`rg '\bResp\b|Resp::' --glob '!target' --glob '!llmanspec/changes/archive/**'` 只命中 `registry.rs` 定义与 `lib.rs` 导出，无任何调用方读取。50 行中 49 行 `Resp::Result`，仅 `prompt`（`registry.rs:135`）一行为 `Resp::Stream`；`Resp::Job` 从未使用。
- `Idem`（`PerRpc` / `Bypass`）：同样只命中定义与导出。真实幂等语义由 `server-core` 的「unary 幂等准入」按 JSON-RPC request id 实现（`llmanspec/specs/server-core/server-core.feature:555`），与 registry 列无关。

同时 `names()`（`registry.rs:495-497`）零消费者：`rg 'registry::names'` 无命中。

后果：新增一个 wire 方法要在 `REGISTRY` 行里手填两个无语义的列（`idem`：45 行 `Idem::PerRpc` + 5 行 `Idem::Bypass`（reload / mcp_oauth_start / mcp_oauth_cancel / loaded_resources / host.describe，即守卫测试钉死名单）；`resp`：49 `Result` + 1 `Stream`——纯样板）。`writer_set_and_bypass_set_match_declaration`（`registry.rs:622-678`）这个守卫测试把 `Bypass` 名单逐字钉死在测试里，钉的是一列没有产品消费者的数据。

## 与既有约束的关系

`app-tui-commands` spec 的 `command-execution-class` 规则（`llmanspec/specs/app-tui-commands/app-tui-commands.feature:147-149`，`@req:r1197`）明文要求 Exec 声明「与 wire REGISTRY 属性行同源并由守卫测试锁定逐变体穷举一致」。因此：

- `Exec` 列**必须保留**——它是受合约保护的 SSOT，本 change 不动它。
- `exec_class(cmd)`（`registry.rs:499-550`）与 `REGISTRY.exec` 的双真相源也**必须保留**——守卫测试 `exec_class_matches_registry_declaration`（`registry.rs:812-1050`）正是 r1197 要求的锁定机制。

本 change 只收敛**不受合约保护、且零消费者**的列。

## What Changes

- 删 `Resp` 枚举与 `MethodEntry.resp` 字段；`REGISTRY` 每行去掉该实参。
- 删 `Idem` 枚举与 `MethodEntry.idem` 字段；`REGISTRY` 每行去掉该实参。
- 删 `names()` 函数与 `lib.rs` 对应导出。
- 删 `writer_set_and_bypass_set_match_declaration` 里的 `Bypass` 断言段（保留 `Writer` 断言段——`Auth` 列有产品消费者 `is_writer_method`）。
- 简化 `m()` / `mx()` 构造器签名（去掉两个实参）。
- `MethodEntry` 收敛为四列：`name` / `auth` / `command_backed` / `exec`。

## 删除验证

对 `Resp` / `Idem` 各做删除想象：

- 删掉 `Resp`：无调用点需要重新实现「响应形态」。真实响应形态由 `xylitol-serve` 的 codec（`codec::response_for`）按方法名与 `Exec` 决定，`prompt` 的流式由 `Resp::Stream` 之外的路径承载。复杂度直接消失，不是「在 N 个调用点重新冒出来」。**判为纯透传样板。**
- 删掉 `Idem`：幂等由 `server-core` 幂等账本按 request id 实现（`server-core.feature:555`）。删掉后无调用点需要补。**判为纯透传样板。**
- 删掉 `names()`：无调用点。**判为零价值。**

对照：`Auth` 与 `Exec` 做删除想象后**会**在调用点重新冒出来（`is_writer_method` 在 `host.rs:1114-1117`；`Exec` 在 `effects/mod.rs:32` 与 `commands.rs:113`），且被 spec 钉死——保留。

## 合约影响

`needs_specs_change: false`。依据：

- `Resp` / `Idem` 未被任何 live `.feature` 提及（`rg 'Resp|Idem' llmanspec/specs/` 无相关命中）。
- `protocol-app.feature:375` 只要求「方法名与方法语义的映射须与产品方法表一致」，不涉及 `Resp` / `Idem`。
- `server-core.feature:555` 的幂等语义按 request id，与 registry 列无关。
- r1197 只钉 `Exec`，本 change 不动。

## 实施路径

quick（不改行为合约的小改动：删除零消费者列与零消费者函数）。按 `xylitol-dev` 硬原则，纯代码组织演进走 quick；本 change 的 `needs_specs_change: false` 与此一致。

## Further Notes

架构审查（llman-sdd-arch-review）候选 C5「registry 方向 B」：延续 c3349/c3355/c3371/c3375 的 zero-consumer 清理动量，半天级小改。
