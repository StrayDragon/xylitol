---
depends_on: []
title: TUI @ 路径补全接入 fd
needs_specs_change: true
---

# TUI @ 路径补全接入 fd

## Why

产品 `AtPathSource` 目前只用 `std::fs::read_dir` 扫单层并 `.take(20)`（`crates/xylitol-tui/src/completion.rs:416` / `:442`）；大仓库里 `@` 补全召回弱，深层路径要逐级输入。引擎已有 `walk_directory_with_fd` / `new_with_fd`（`crates/xylitol-tui/src/autocomplete_fd.rs:48`、`autocomplete.rs:78`），但产品未接。

## What Changes

- 引擎新增通用 `fd_path_suggestions`（或等价公共 helper）：给定 base / fd / query / is_at / is_quoted，返回 `AutocompleteItem`；复用 `walk_directory_with_fd` + 现有 fd 评分逻辑（`autocomplete.rs:297-332`）。
- 产品 `AtPathSource` 增加可选 `fd_path`；启动时在 PATH 里找 `fd`（兼容 Debian `fdfind`）；找不到则回落现有 read_dir 单层逻辑。
- 补全行为：`@` 前缀走 fd（max 100，取前 20）；fd 无结果时回落 read_dir；结果按 name / path 评分；保留目录后缀 `/` 与 quoted 语义。
- 不改 `@` 触发符 / 编辑期粘贴 / 光标语义；不改 completion source 列表结构（仍一个 `at-path` source）。
- 性能：fd 进程同步调用，沿用引擎现有实现；结果缓存不在本 change（后续可加）。

## Capabilities（拟）

- `app-tui-input`：`@` 路径补全在 fd 可用时使用 fd；fd 不可用/无结果时回落单层扫描。
- `package-tui-*`：新增公共 fd 路径补全 helper（若放引擎）。

## Impact

`crates/xylitol-app-tui/src/layout/root/mod.rs`（AtPathSource 构造）、`crates/xylitol-tui/src/completion.rs` / `autocomplete_fd.rs`（公共 helper）。不改 Host / 协议 / 折叠。

## 非目标

- 不做 CancellableLoader / Tabs（列入 handoff 的后续引擎接入）。
- 不做异步补全 / 后台索引 / 持久缓存。
- 不做 `@` 之外的触发符。
- 不引入新 crate 依赖；fd 二进制缺失时静默回落。

## 状态注记

> **designed**：proposal + design 已完成；未写代码、未绑分支、未改 live specs。apply 时按 design 的「实施任务」生成 tasks.md。
