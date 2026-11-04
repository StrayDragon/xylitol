---
depends_on: []
title: 补齐 TUI UX 验收缺口
branch: sdd/c3150-update-tui-ux-gaps
base_branch: main
base_sha: 221c75aea59ad67ebc593c4433063ee5635b610e
---

## Why

c3110–c3145 已归档，但对照 `_HANDOFF/next-20261105.md` 的逐项验收仍有缺口：c3120 的流式 thinking 单行与 recent/full 展开、c3125 的完整参数/用户消息预览、c3130 的详情逻辑删除与 spinner 脉冲、c3135 的 MessageUpdate 合批与 tick 预算、c3140 的键位冲突 harness/OSC8/mermaid snapshot、c3145 的 CJK 文件名。此 change 只补这些缺口，不重做已落地部分。

## What Changes

- c3120：summary/quiet 流式 thinking 单行带最新文本；recent/full 全文；enabled=false 不画信封头；recent 轮末展开最近簇。
- c3125：Tool 完整参数进 args viewport；args hidden/preview/full；用户消息 3 行 preview + 点击展开；缓存指纹纳入展开态。
- c3130：删除详情 popup/chip/hit/pending/keybinding/测试/spec 例外；`◨` 预留 2 列并可命中；spinner `⬢` 100ms 亮暗脉冲。
- c3135：MessageUpdate 合批；busy tick 33ms；内容 delta 不标 AO stale；spinner 100ms；补 lab 记录。
- c3140：键位冲突 harness；OSC8 产品 snapshot；mermaid snapshot。
- c3145：fd CJK 文件名与产品 harness 回落用例。
