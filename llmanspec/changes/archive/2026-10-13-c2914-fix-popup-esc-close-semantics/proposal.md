---
depends_on: []
title: 会话详情覆盖弹出框收起语义补可执行场景（r2903）
needs_specs_change: true
branch: sdd/c2914
base_branch: main
base_sha: 9d3d1562a782dfa67cdc4d5d4f32f8e678bedff2
---

# 会话详情覆盖弹出框收起语义补可执行场景（r2903）

## Why

r2903 现仅有「切换显隐」一个场景；c2875 design 深挖审计修正过的两个实伤行为——忙碌态 Esc 透传中止链（不收框）、收框 Esc 不进双 Esc 开树链——没有 BDD 场景护住，回归全靠人工记忆。按 review 结论补场景锁行为。

## What Changes

- **产品修复（实伤）**：c2875 审计③「收框不入双 Esc 开树链」实际未生效——引擎级 input listener（`mount.rs`）先于 UiRoot 守卫调 `on_escape()`，收框那次 Esc 的时间戳被记入 `last_esc_at`，500ms 内再按 Esc 即误开会话树。修复：`on_escape` 顶部收覆盖弹出框（最上层瞬态面优先；闲置即收且不记链，忙碌让位中止链）。
- `app-tui-fixed-zone` r2903 增两场景：
  - `popup-idle-esc-close-no-tree-chain`：闲置 Esc 收起覆盖弹出框；随后一次 Esc 不得打开会话树（收框不入双击链）。
  - `popup-busy-esc-passes-abort`：忙碌态 Esc 计一次中止且覆盖弹出框仍渲染（中止优先于收框）。
- `tests/bdd/steps_app_tui_host.rs` 增对应 when/then 步骤（复用既有 stats：`aborts` / `tree_open` / `tree_calls`）；bindings 同步。

## 非目标

- 不动 r2904 侧栏场景。
