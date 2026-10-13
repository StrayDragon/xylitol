---
depends_on: []
title: 编辑器历史合并召回顺序修复：跨实例条目最新优先
needs_specs_change: true
---

## Why

c2915 使用点懒并盘引入顺序反转：`storage.editor_history()` 返回新→旧序，
直传包层 `merge_history`，而后者逐条 `insert(0)` 的循环按旧→新输入才能得到
最新在前。结果合并后 `history[0]` 是通道最旧条目；`↑` 从 index 0 开始召回，
跨实例 / 重启后第一次召回的是最旧提示词，最新发送的要按到底才出现。
包层 doc 注释「按给定新旧序前置」与实现矛盾；BDD 只断条数不断顺序，未拦截。

## What Changes

- app 层 seam（`merge_client_history`）把通道历史反转为旧→新再传包层，
  对齐 `replace_history` 的 oldest→newest 输入约定。
- 包层 `merge_history` doc 契约显式化为 oldest→newest 输入、合并后最新在最前；
  行为不变（循环体原样），补顺序单测。
- specs `app-tui-storage` r2902 补召回顺序条款与场景：他实例先写旧条目再写
  新条目后按 `↑`，召回文本须为通道最新条目（BDD 真按键路径断言）。

## Capabilities

- `app-tui-storage`（r2902 增补顺序条款 + 嵌套场景）

## Impact

`src/app/tui/host/editor_history.rs`、`packages/xylitol-tui/src/components/editor/mod.rs`
（doc + 单测）、`tests/bdd/steps_app_tui_host.rs`；无迁移 / 兼容面。
