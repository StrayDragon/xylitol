---
depends_on: []
title: 会话详情覆盖弹出框收起语义补可执行场景（r2903）
needs_specs_change: true
---

# 会话详情覆盖弹出框收起语义补可执行场景（r2903）

## Why

r2903 现仅有「切换显隐」一个场景；c2875 design 深挖审计修正过的两个实伤行为——忙碌态 Esc 透传中止链（不收框）、收框 Esc 不进双 Esc 开树链——没有 BDD 场景护住，回归全靠人工记忆。按 review 结论补场景锁行为。

## What Changes

- `app-tui-fixed-zone` r2903 增两场景：
  - `popup-idle-esc-close-no-tree-chain`：闲置 Esc 收起覆盖弹出框；随后一次 Esc 不得打开会话树（收框不入双击链）。
  - `popup-busy-esc-passes-abort`：忙碌态 Esc 计一次中止且覆盖弹出框仍渲染（中止优先于收框）。
- `tests/bdd/steps_app_tui_host.rs` 增对应 when/then 步骤（复用既有 stats：`aborts` / `tree_open` / `tree_calls`）；bindings 同步。

## 非目标

- 不改产品代码（两行为已在 slot_input 实现，本票仅补合约护栏）。
- 不动 r2904 侧栏场景。
