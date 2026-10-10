---
depends_on: []
blocks:
  - c2860-add-tui-block-click-hover
branch: sdd/c2858-add-package-tui-pointer-hit-pipeline
base_branch: main
base_sha: a536bf44690e36ba8a9e215b432e7a21324daebd
needs_specs_change: false
---

# 引擎指针命中管线（click / hover / capture）

## Why

后续鼠标增强（整块折叠、悬停 tint、可点 Tab、overlay 抽屉、mermaid 图内平移）都卡在同一条管道，而不是各自缺一个 widget。

现行 ApplicationOwned 鼠标是 **选区优先、Down 闩锁、Moved 双丢**：

1. 产品 fan-in 在进 `dispatch_event` 前丢掉 `Moved`（ath28 / r1258）。
2. 引擎对无态 `Moved` 再丢一次，除非 editor 正在拖选（r1632）。
3. `HitPriorityFn` 只在 **Left Down** 返回 bool；Up/Drag **禁止**再问（包测写死）。命中则 swallow、不启拖选（ath33）。
4. 几何只有 transcript 全宽带 + 底 `dock_rows`。AO 鼠标在 overlay **之前**消费，抽屉点到的是 transcript。
5. 脏标记只有 `ao_components_stale`（全量 `Component::render`）vs wheel `reproject`。没有「只补丁可见行」。

ratatui 并不更强：widget **不收鼠标**，app 用上一帧 `Rect.contains`。xylitol 不该抄 `Buffer`/`Constraint`（D12、无双栏），但应抄这个分工——**引擎持几何与手势分类，产品持 id 语义**。

本票只加引擎缝；默认行为与今日测试相同。产品折叠几何 / 停丢 Moved 归 c2860。

## What Changes

- 包层 host API `Hit::on_line` / `Hit::at_cell` + `TUI::mark_hit`（引擎存储仍是 `HitRegionList`）。正文行事件时刻用 live `scroll_top` 做 screen→id；可见格钉屏，滚动不漂移。host 不必自算 `content_row = scroll_top + screen_row`。
- 手势相位 **默认 `DownSwallow`**（今日 ath33）。可选 `UpIfEmptySelection`：Up 仅当无选区且位移为 0 才报 click；Drag 永不进 click。
- 可选 pointer-probe：武装后 `Moved` 送到探针；id 未变 **不排帧**；id 变则 **reproject + 可见行 overlay**（学 `apply_highlight`），MUST NOT `ao_components_stale`。
- z / capture：Overlay 与 capturing region 在选区 **之前**；Down 命中则后续 Drag/Up 直到松开不启 transcript 拖选。
- `HitPriorityFn` 保留为薄适配（`id.is_some()`），避免一次打碎 c2040。
- **不**把 `FoldTarget` / 三角列 / 产品 toggle 放进包。

## 非目标

- 不改产品 r1352 三角列-only、不改 ath33 默认（c2860 再翻）。
- 不引入 ratatui `Buffer` / `Constraint` / `StatefulWidget`。
- 不给 Inline 兼得原生选区 + 点选。
- 不做悬停视觉 token（产品 DESIGN / lab hover-highlight）。
- 不实施边栏、Tab 内容、mermaid pan（只提供它们以后能挂的缝）。

## Capabilities

- `package-tui-pointer`（新：命中区 / click 相位 / 探针 / capture / 廉价悬停绘出）
- `package-tui-interaction-modes` r1632 **不改句**（无态 Moved 仍不得强制整帧；探针是同一管道的 opt-in）

## Impact

- 包测：`hit_priority_runs_only_on_left_down_not_drag` 在默认相位必须继续绿。
- 新产品测走 `dispatch_event` 直驱；产品 fan-in 仍丢 Moved，直到 c2860 停丢。
- `agent_demo` / `host_loop_application_owned` 可演示通用 hit id，不必 import 主 crate。

## Decisions

1. 相位用枚举 `DownSwallow`（默认）/ `UpIfEmptySelection`，不用双回调。
2. hover 独立 muted tint 层，不复用 selection invert。

## Further Notes

- 管道与 ratatui 对照：[research/pointer-pipeline.md](./research/pointer-pipeline.md)
- 决策与缝：[design.md](./design.md)
- 可执行合约 SSOT：`packages/xylitol-tui/llmanspec/specs/package-tui-pointer/`（`package-tui-*` 落在嵌套包实例）。根 specs-landed 只扫 `llmanspec/specs/`，且 `@req` 跨根必须唯一，故不在根树再抄一份；本 change 标 `needs_specs_change: false` 以过 apply 门。
