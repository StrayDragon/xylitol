# Design — c1175-refactor-slash-command-ssot

## 问题

| 面 | 现状 |
|---|---|
| `agent/prompt/commands.rs` | 22 条 pi 短名 + `#![allow(dead_code)]`；注释仍提已删 stdio rpc / c355 |
| `app/tui/layout/slash_catalog.rs` | 产品 `session-*` 补全 |
| `app/tui/commands.rs` | 产品解析（A03） |
| `Driver::get_commands` | 映射 agent 表 → 对 Server/客户端撒谎 |

## 决策：SSOT 放哪

**SSOT 放在 `agent` 可见、`app` 可依赖的模块**（推荐 `agent/prompt/product_commands.rs` 或收紧现有 `commands.rs`），因为：

- 分层：`app → agent` 合法；`agent → app` 禁止
- `InProcessDriver::get_commands` 已读 agent；Remote 经 REST 同源语义

表项最小字段：`name`、`description`、可选 `argument_hint`。
**仅包含产品已实现（或 debug 条件编译）的命令**——与 TUI 补全一致；未实现的 `/trust` `/reload` 等 **不**进 SSOT（留给对应 feature change）。

`quit`：产品解析仍接受 `exit|quit`；补全主名保持 `exit`（与现 catalog 一致）。SSOT 可只列 `exit`，解析侧保留 quit 别名。

## 接线

```text
product_commands::PRODUCT_SLASH_COMMANDS
        ├─ agent get_all_commands / get_commands
        ├─ app/tui slash_catalog（描述 + hint）
        └─ （可选）单元测：parse 可识别名 ⊆ SSOT 名 ∪ {quit 别名} ∪ debug-only
```

`parse_slash_command` 可继续手写 match（行为合约已在 atm*）；本变更 MUST 保证 **名称集合** 不漂移，而非强制把 match 生成器化。

## 明确不做

- 产品重新识别 `/tree` `/fork` `/compact` `/export` `/import` `/resume` `/new` `/clone` `/name`
- 为「未来命令」预填 GetCommands 空壳（避免假发现）
- 改 bang / effects 执行语义
