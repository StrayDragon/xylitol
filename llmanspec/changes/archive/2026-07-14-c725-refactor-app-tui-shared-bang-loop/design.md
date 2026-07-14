# Design — c725-refactor-app-tui-shared-bang-loop

## 拓扑选项

| 方案 | 形状 | 取舍 |
|---|---|---|
| **A（默认）** | `if bash { shared_bang_select_loop(...) } else { shared_main_select(...) }`；两环共用 `handle_term`/`handle_agent`/`handle_tick` 函数 | 改动面小；ath7「等价扇入」满足；允许语法上仍有嵌套 |
| B | 外层单一 `select!`，bang_fut/chunk 为 Optional 臂 | 字面「一个 select」；状态机更绕 |

**默认 A**：一次完成优先风险可控；若 A 落地后臂仍重复再升 B。

## 与 c715 关系

c715：helper 存在、HRS 折叠。
c725：生产消重、RPB 退役、ath7 字面收紧。

## 不变式

- bang Esc → `note_bash_cancelled`
- agent Esc → c720 suppress + `note_user_abort`
- chunk → dirty only
