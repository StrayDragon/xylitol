---
depends_on: []
---

# 宽度断点信息分级：横向披露档位表

## Why

终端宽度变化时各组件「藏什么、留什么」目前是逐组件 ad-hoc 决定：
footer 有紧凑 token 规则、models picker 有 `width_budget` 参数、
atoms 有截断规则——但没有统一的**断点档位表**。结果是同类信息在不同
组件里的降级顺序不一致，新组件无表可查。

## What Changes

- 在 `src/app/tui/DESIGN.md` 增补横向断点档位表（建议五档：
  极窄 / 窄 / 标准 / 宽 / 极宽），每档写明：哪些固定区元素保留 /
  降级 / 隐藏，字段级（如模型信息：先隐 provider 再隐 thinking 档）。
- 断点判定收敛为布局根的单点函数，组件按档位取值；
  现有 footer / models picker 的既有规则迁入档位表（行为不变优先）。
- designing 各模块 states 增补对应宽度的固定态样例（先覆盖 footer/status/editor）。

## 非目标

- 不改纵向槽高预算逻辑（`layout` 模块既有行数预算独立运作）；
- 不做鼠标窗口尺寸记忆/持久化。

## Impact

- DESIGN.md + 布局根单点函数；三五个组件的降级分支对表重排；
- `just check-tui-designing` states 随动；视觉回归靠现有 harness 快照。

## Further Notes

- 现状核对（各组件现存宽度处理清单）：[research/width-breakpoint-notes.md](./research/width-breakpoint-notes.md)
