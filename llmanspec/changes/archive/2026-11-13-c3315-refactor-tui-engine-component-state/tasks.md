---
depends_on: []
title: "Consolidate TUI engine component state"
needs_specs_change: false
---

# Tasks — c3315 TUI 引擎组件状态收敛

每条完成即勾选并跑验收命令；全绿后 validate --strict + just qa。

- [x] T1 kitty 全局位单真值：删 terminal.rs kitty_pushed 死字段（E6+E25），keys.rs 全局位为准。验收：`cargo nextest run -p xylitol-tui --all-features` 绿 + `rg -n 'kitty_pushed' crates/xylitol-tui/src` 零命中。
- [x] T2 补全源边界（E23）：apply_probed 校验 sources 下标，空源显式 no-op；补空 registry + 单项 tab 不 panic 回归单测。blocked_by: []
- [x] T3 panel pending_scroll 单真值（E16）：scroll_by 清 pending；Clip 模式消费或显式丢弃。blocked_by: []
- [x] T4 follow_bottom 重算点（E5）：set_lines / viewport 变更即刷新，at_bottom 派生；含视口变大仍跟随单测。blocked_by: []
- [x] T5 loader aborted 门（E22）：tick/render 先查 aborted。blocked_by: []
- [x] T6 paste_burst 谓词合一（E12）：is_pasting 单点。blocked_by: []
- [x] T7 jump_mode 枚举化（E8 修正版）：JumpDir 枚举 + 失焦清除 + cancel API。blocked_by: []
- [x] T8 history 单真值（E10）：isize 哨兵改枚举/Option，draft 单一生命周期。blocked_by: []
- [x] T9 last_action 枚举化（E11 修正版）：共用定义替换 46 写点，Editor/Input 去重。blocked_by: []
- [x] T10 自动补全单 Option（E9）：AutocompleteState 合一。blocked_by: [T2]
- [x] T11 渲染副产物分离 + 脏标志簇（E20/E13）：layout 副产物显式快照结构；脏标志单一 struct。blocked_by: [T4]
- [x] T12 scroll_region / tabs 命中数据一致（E17/E18）。blocked_by: [T11]
- [x] T13 暂停旁路统一（E14）：复用 with_terminal_suspended。blocked_by: []
- [x] T14 settings 子菜单显式关闭 + 字段拆义（E15）。blocked_by: []
- [x] T15 markdown 缓存键补齐（E21 防御）：键含 mermaid_mode / table_header_underline。blocked_by: []
- [x] T16 收口：`llman-sdd validate c3315 --strict` 绿、`just fmt && just lint && just test && just qa` 绿（WASI_SDK_PATH 导出）。
