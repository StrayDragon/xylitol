# Tasks — 宽度断点信息分级（c2520）

垂直切片；每个 task 可独立验证。收口（finalize / archive）是流水线步骤，不列于此。

## T1 tier 基座

- [x] `src/app/tui/layout/tier.rs`：`WidthTier` 五变体（Narrowest → Widest，派生可比较）+ `width_tier(cols) -> WidthTier` + 档界常量（40 / 60 / 80 / 100 / 120）；`layout/mod.rs` 导出
- [x] 表驱动边界单测：40 / 59 / 60 / 79 / 80 / 99 / 100 / 119 / 120 两侧判定（含 <40 仍返回极窄的文档化行为）
- 验证：`cargo test --lib width_tier` 全绿；`just test-tui` 不红

## T2 离散断点收编 [blocked-by: T1]

- [x] todo_bar：删除 `TODO_BAR_BREAKPOINT`，双列判定改查档位（≥ 宽）；既有单测（含 100 两侧用例）零改动保持绿
- [x] app scrollback diff：显式传 `DiffOptions::side_by_side_min_width = Some(宽档下缘常量)`；引擎 `packages/xylitol-tui` 零改动
- 验证：`just test-tui` 全绿（快照不变 = 行为不变）；`rg -n "TODO_BAR_BREAKPOINT"` 无残留引用

## T3 档位表 SSOT + states 样例 [blocked-by: T2]

- [x] `src/app/tui/DESIGN.md` 增「横向断点档位」节：五档 × 元素矩阵（保留 / 紧凑 / 隐藏；连续截断组件注明无档位开关）+ 纪律句（组件只许消费 `width_tier`，禁自写阈值）+ `spacing.diff-side-by-side-min-cols` = 宽档下缘注记（token 名值不动）
- [x] designing states：新增极窄（40）/窄（60）降级样例 `tier-narrowest` / `tier-narrow`（footer ×2、status、editor）；标准档 = 既有 `default`（80）；宽档 = 既有 `*-wide`（100，todo-bar / diff）；极宽不设专态（连续渲染，DESIGN 表注明）
- 验证：`just check-tui-designing` 通过；`just test-tui` 快照不变
