---
depends_on: []
blocks: []
title: TUI 提示行恒占位与帮助框停靠固定区顶
---

# TUI 提示行恒占位与帮助框停靠固定区顶

## Why

两点观感问题，同源于「下缘固定区高度随内容条件变化」：

1. `?` 提示行按 editor 是否为空条件出现/消失（`hint_rows()` 0/1 行），dock 高度随之抖动——输入第一个字符即整屏重排，观感一跳一跳且带来重绘开销。
2. 快捷键帮助面板（c3342）渲染在 spinner（status）行与输入框之间；dock 底部锚定布局下，插入点上方的所有行（队列条/待办栏/通知条/status/tray）在面板开合时整体位移，spinner 跳动。

## What Changes

- 提示行恒渲染 1 行：不可见时为空白占位行；`? shortcuts` / `? / Esc close` 文案规则与 `?` 打开规则（仅 editor 空）不变。
- Fixed-Zone Footprint SSOT：hint 行恒按 1 行计入（`reserved_lower_fixed_zone` 收敛 hint 形参）。
- 帮助面板渲染位次移至固定区最顶（滚动区之下、队列条之上）：面板开合仅由滚动区让位，status / tray / editor / footer / hint 行的屏上绝对行位不变。
- specs：`app-tui-fixed-zone` r1228（footprint）与 r3356（footer-hint-line）修订；新增裸规则「帮助框停靠固定区顶」。

## Design Choice

| 方案 | 问题 | 结论 |
|---|---|---|
| hint 行维持 0/1 条件行（现状） | dock 高度抖动、整屏重排 | 拒绝 |
| hint 占位行压缩它槽行高 | 违反 status 恒 1 行等既定条款 | 拒绝 |
| **hint 恒 1 行占位** | 短终端恒定少 1 行滚动区；行为恒定无跳动 | **采用** |
| 帮助框留在 spinner 与 editor 之间（现状） | 底部锚定下插入点上方各行整体位移，spinner 跳动 | 拒绝 |
| 帮助框置于 spinner 行上方（通知条之下） | spinner 不动，但队列条/待办栏仍被推移 | 否决 |
| **帮助框置于固定区最顶（队列条之上）** | 开合仅滚动区让位；固定区全部行位锚定 | **采用** |

选择器（`/`、`$` 候补）预算来自 slot 预算，恒定减 1 行不引起跳动；实施后实测候补行数，若不足再另立覆盖行为。

## Impact

- crates：`xylitol-app-tui`（layout/root 装配与 footprint、tests）；`tests/bdd` 仅在既有 helper 依赖 hint 行位置时同步。
- specs：`app-tui-fixed-zone`（r1228 / r3356 修订 + 新增裸规则）。
- 兼容：`固定区槽顺序` 场景（队列条、待办栏、通知条、status、editor）不打开 help，不受影响；待办栏/通知条邻接条款不受影响。

## Further Notes

- r3356 现文即声明「由 harness 与 snapshot 覆盖，不扩 BDD step」；新规则沿用裸规则形态（几何行位断言落 harness/单测）。
- c3342（帮助面板）当时无位置条款，本 change 补齐位置合约。
