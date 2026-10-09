# Design — c2849 追溯收口

## 决策表

| # | 决策 | 取舍 |
|---|---|---|
| D1 | 追溯 spec 而非行为修复 | 行为已上线（`9de00e26`，PR #8 内）且带回归测试；缺的只是合约与编号。改行为无收益、有风险。 |
| D2 | 认领悬空编号 **c2849** 作为本 change id | 代码注释（host.rs / remote.rs）已写 c2849；认领后注释即刻归位，不再有检索死端。 |
| D3 | r1925 采用 anchor-only（无嵌套场景），verified-by 锚向 `arm_tool_freeze_conflict_falls_back_lease_free_not_stuck` | 与 r1923 同形态：单测回归已是自动化锁；为它再写一条 BDD 场景会重复全链路（测试分层：BDD 管编排、单测管组件边界）。 |
| D4 | 正常路径租约语义不动 | `writer_lease_window_across_connections`（mint/冲突/续用三观测点）继续锁定；仅 arm_tool_freeze 的冲突分支降级——它冻结的是写者内部门、不写会话状态，与 loaded_resources 同梯队。 |

## 影响面

- 无行为 diff；仅 spec 追加 + 既有注释编号校验。
- 失败模式：无（纯文档/合约层）。
