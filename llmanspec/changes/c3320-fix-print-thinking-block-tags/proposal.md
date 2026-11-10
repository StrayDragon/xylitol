---
depends_on: []
title: "Fix print thinking block tag pairing"
---

# Print 思考块标签配对修复

## Why

print 端 thinking 块用两个 bool 编码四态（已扫描定位，实施前复核）：in_thinking_block + thinking_has_tags（print.rs:30-31）——MessageStart 直接复位未先闭合即丢开标签；标签只在首片采样，后到标签会多打一次闭合；close 点四处分散。输出错乱直接写到 process stderr，render_stream 的 writer 测不到。

## What Changes

- 思考块状态改显式枚举（块外 / 块内·自带标签 / 块内·需补标签），单点开闭；MessageStart 前先收口旧块。

## Capabilities

- cli-print（reasoning 流式 stderr）。

## Impact

- crates/xylitol-app-print/src/print.rs；补 writer 可注入的闭合断言测试。

## Open Questions

1. 是否允许 stderr 标签风格配置（当前固定 <think> 形态）？
