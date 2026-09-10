---
depends_on:
- c2710-refactor-driver-command-dispatch
branch: sdd/c2730-remove-product-dead-knobs
base_sha: b2604d2845b4bdb5f8c2de881dd5073e442e473f
rules_edit_acked: true
checkpointed: false
---

# 删除产品死旋钮：DatePlacement 消融与 cycle_thinking

## Why

`DatePlacement` 非 Omit 分支是消融/实验路径；产品默认 Omit（date/cwd 走 session_env）。`XyDriver::cycle_thinking_level` 文档写明仅 demo/legacy；产品 TUI 只经 `/model`（ati36）。发布后这些会变成「未交付能力的兼容旋钮」。Pre-0.0.1 删代码与 API，不留 alias。

## What Changes

- 删除 `DatePlacement` 枚举及 `SystemPinnedAtSession` / `SystemAsToday` 装配；只保留 Omit 等价行为（session_env）。
- 删除 `cycle_thinking_level`（trait/Command/执行器/harness）。thinking 只经 `SetThinkingLevel` / `/model`。
- 删除仅为消融存在的 Driver/Capabilities 方法与测试。
- 配置若暴露 datePlacement 键：一次性删除键，不 deprecated。

## 非目标

- 不改 session_env 产品行为。
- 不删 `/model` 切换 thinking。
- `/debug` 留给 `c2740`。

## Capabilities

产品可观察：无 thinking 循环 API；无 date placement 消融。start 后改 live spec（prompt 装配 / Driver 方法表）。**先不改** `.feature`。

## Impact

破坏：脚本/harness 调 `cycle_thinking_level`；任何依赖 DatePlacement 的实验配置。

## 本批依赖

`c2710`：方法已是 Command 执行器上的一格，删一变体即可，避免先删 trait 再塌 dispatch 打架。
