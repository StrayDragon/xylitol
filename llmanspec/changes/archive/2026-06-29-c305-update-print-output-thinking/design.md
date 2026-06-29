---
change_id: c305-update-print-output-thinking
title: print 模式 thinking 输出 spec 补登记
---

# Design — print 模式 thinking 输出 spec 补登记

## 背景

本变更是对既有 commit `03f8866` 的 **spec 补登记**，不含任何代码改动。行为已经实现并通过测试，只是 `print-output` spec 当时没有同步更新。

## 为什么没有设计权衡

- thinking 输出到 stderr 而非 stdout：这是 commit `03f8866` 已确定的设计，目的是保持 stdout 纯净（便于管道/重定向），与 `ToolExecutionUpdate` 等辅助信息同样走 stderr 的既有约定一致。
- `<think>` 标签自动包裹 + 模型自带标签检测：已实现，本变更只把它写进 spec。

因此本变更没有新的设计决策需要记录，design.md 仅用于满足 llman stage 守卫（full stage 要求 design.md 存在）。

## 受影响 spec

- `print-output`：新增两条 requirement（`thinking-to-stderr`、`message-dedup`），与 `src/app/print.rs::render_stream` 的实际分支一一对应。

## 验证

- `llman sdd validate c305-update-print-output-thinking --strict --no-interactive` 通过。
- 归档后 `llman sdd validate print-output --strict --no-interactive` 通过。
