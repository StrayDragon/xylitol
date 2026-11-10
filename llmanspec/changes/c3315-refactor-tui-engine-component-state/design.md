# Design — TUI 引擎组件状态收敛

## 1. 目标与边界

- 引擎组件内状态类型化；公共组件行为不变（needs_specs_change: false）。
- 边界：不动 Component trait 与渲染管线结构。

## 2. 决策摘要

| 项 | 方向 |
|---|---|
| follow_bottom | set_lines 后重算 + paint 后回写，消灭缓存漂移（E5） |
| terminal | kitty_pushed 死位删除或改读；6 bool 收敛 TerminalMode 结构（E6） |
| jump_mode | 显式 JumpDir 枚举 + Esc 取消（E8） |
| 自动补全 | list+state 合一 Option<AutocompleteState>（E9）；补全源索引边界检查（E23） |
| history | isize 哨兵改 Option + draft 单一生命周期（E10） |
| last_action | 动作改枚举（E11，37 写点机械替换） |
| paste_burst | 三谓词合一 is_pasting（E12） |
| 渲染/生命周期 | 脏标志簇单一 struct（E13）；with_terminal_suspended 复用 session 函数（E14） |
| 其余 | E15-E18/E20-E22/E25 按扫描点位逐条收敛 |

## 3. 实施顺序

与 c3285（指针/选区/焦点）同 crate 不同 seam，其后实施；机械替换类（E11）优先。

## 4. 测试边界

组件单测 + 既有引擎 harness；行为不变由 package-tui BDD 守住。
