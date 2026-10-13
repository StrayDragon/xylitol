# Design — 宽度断点信息分级（c2520）

决策均已在 propose 阶段与用户逐项确认（2026-06 会话记录，四问四答）。

## D1 不改行为合约（`needs_specs_change: false`）

全仓唯一的离散宽度断点是 100 列，已被双重钉住：`agent-todo` feature（100 须三栏、更窄竖叠）与 DESIGN token `spacing.diff-side-by-side-min-cols: "100"`。其余组件（footer / status / models picker / toast）均为连续截断，无阈值可钉。本 change 行为不变，故无新合约；apply 期如确需补 specs 仍可补（该 frontmatter 仅豁免门禁，不禁写）。

## D2 档界：40 / 60 / 80 / 100 / 120（五档）

| 档 | 极窄 | 窄 | 标准 | 宽 | 极宽 |
|---|---|---|---|---|---|
| cols | 40–59 | 60–79 | 80–99 | 100–119 | ≥120 |
| 硬度 | 下界硬 | 锚点 | 下界语义锚 | 下界硬 | 锚点 |

- **40 硬**：`MIN_COLS = 40`（`src/app/tui/host/mod.rs:31`，注释 ath4）；低于 40 即 `LayoutMode::TooSmall`（“请放大终端”），不是渲染档。`width_tier` 操作域为 ≥40；<40 的输入仍返回极窄，文档注明。
- **100 硬**：`TODO_BAR_BREAKPOINT`（`src/app/tui/widgets/todo_bar.rs:11`）、引擎 `DiffOptions::side_by_side_min_width` 默认（`packages/xylitol-tui/src/components/diff.rs:103`）、DESIGN token 三处同源，且有 spec 引用。
- **80 语义锚**：终端默认宽度，必须落在标准档（标准 = 完整默认体验），故为标准档下界而非档界两侧各半。
- **60 / 120 纯锚点**：无既存依据；写 states 目检时可调，调整不触合约（D1）。

## D3 单点化机制（“100 自适应”）

- 新增 `WidthTier`（五变体， Narrowest→Widest 可比较）与 `width_tier(cols) -> WidthTier`，落 `src/app/tui/layout/tier.rs`。产品层纯函数；引擎 `packages/xylitol-tui` 不感知。
- todo_bar：删除自有常量 `TODO_BAR_BREAKPOINT`，双列判定改查档位（≥ 宽）。
- diff：app 侧显式传 `DiffOptions::side_by_side_min_width = Some(宽档下缘)`。引擎字段本为 `Option<usize>` 参数，**引擎零改动**；引擎内默认值 100 保留为通用件回退。
- DESIGN token `spacing.diff-side-by-side-min-cols` 名与值不动 = 宽档下缘；`agent-todo` spec 引用继续为真。
- 残余手动同步点：token 值 ↔ tier 常量（文档 ↔ 代码），与现状同水平；在 DESIGN 档位表节注记绑定关系。
- 纪律：组件只许消费 `width_tier` / 档位常量，不得自写阈值比较——写入 DESIGN.md 档位表节，不进 specs（实现纪律非行为合约）。

## D4 测试 seam（3 层 + 1 checker，无新缝）

1. 组件单测：todo_bar 既有 100 两侧用例（`todo_bar.rs:664-686` 一带）**零改动保持绿**，即行为不变证据；新增仅 tier 判定表驱动边界单测（40 / 59 / 60 / 79 / 80 / 99 / 100 / 119 / 120）。
2. root 层：既有 `*_for_test(width)` 访问器 + insta 快照（`src/app/tui/tests.rs`、`snapshots/`），门禁 `just test-tui`。
3. BDD 只读回归：`agent-todo` 14 场景等随 `just test` / `qa` 跑；均为数据投影绑定（`tests/bdd/bindings_agent_todo.rs`），无布局场景——布局回归由第 1 层承担。本 change 不写新场景、不动 bindings。
4. states 过 `just check-tui-designing`（`scripts/check_tui_designing.py`）。

## D5 极窄档承诺

极窄（40–59）只保证核心对话流可用，不承诺全功能。内部目标，非合约 MUST，不入 specs。

## 范围修正（对 proposal Impact 的收敛）

核查结论：footer / status / models picker / toast 均为连续截断，无可迁移阈值。离散收编仅 todo_bar、diff 两处；proposal 中“三五个组件降级分支对表重排”收缩为：档位矩阵标注（连续截断组件注明“无档位开关”）+ 后续新组件遵循档位表。
