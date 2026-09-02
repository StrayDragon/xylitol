---
depends_on:
  - c2500-unify-select-list-protocol
---

# 命令面板：product_commands 的第二视图

## Why

产品 slash 目录已有单一真源（`app::product_commands` + `XyDriver::get_commands`），
但发现入口只有编辑器内 `/` 触发一条路：用户必须记得命令名才能用；
命令数量增长后（MCP / skill / 会话族），无模糊搜索的入口会成为瓶颈。
命令面板是同一次真源之上的第二个视图，不改 SSOT。

## What Changes

- 以 c2500 SelectList 协议渲染面板：全量命令 + 模糊过滤 + 键位提示列
  （数据来自 keybindings manager 反查，永不失同步）。
- 入口键位待拍板（历史约束：Ctrl+P 曾预留给已冻结的 Plate stub）。
- 面板执行 = 等价于用户输入对应 slash 后回车，复用既有 pending/effects 路径，
  **不开第二条执行通道**。

## 非目标

- 不解冻 Plate / Settings stub；不做运行时改配置。
- 不做跨 surface 的远程触发。

## Impact

- `src/app/tui/layout/` 新增一个 slot；`commands.rs` 目录零改动（只读）；
- BDD 补「面板执行与 slash 直输等价」场景。

## Further Notes

- 调研补充（现状核对 / 决策点）：[research/command-palette-notes.md](./research/command-palette-notes.md)
