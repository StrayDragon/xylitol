# Design — TUI 引擎组件状态收敛

## 0. 复核结论（2026-11-13，HEAD 7d36d7e 逐条开文件核对）

18 点位：成立 13、表述需修正 3（E8/E11/E21）、不成立 0、已被修复 0。c3275–c3300 未触及本 crate；c3285 仅 pointer/selection/focus seam，18 点零重叠。

修正三处：

- E8「jump_mode 不可取消」不成立：任意下一键（含 Esc）即消费退出（editor/mod.rs:246-251）。真缺陷 = Option<bool> 双义编码 + 失焦不清除 + 无显式取消 API。
- E11「37 写点」计数过期：现值 46（editor 29 + components/input.rs 17）；魔法值 "type-word"/"kill"/"yank" 双组件重复，实质成立。
- E21「缓存不随 mermaid_mode 失效」不成立：唯一入口 with_mermaid_mode 已调 invalidate（markdown.rs:101-105）。真缺陷 = 缓存键 (text,width) 不含 mermaid_mode / table_header_underline，正确性靠单一入口纪律维持。

高价值 bug（实施排序依据）：E23 补全源无界下标（completion.rs:157 → autocomplete.rs:102 unwrap_or(0)，空 registry + 单项 tab 可 panic）；E16 pending_scroll 双真值（panel_region.rs:90-100 scroll_by 不清 pending、:131-145 Clip 模式永不消费）；E5 follow_bottom 在 project_frame 改 viewport/lines 后不刷新（application_owned_runtime.rs:170-197）；E22 loader aborted 后仍转圈（cancellable_loader.rs:28/:42）；E6+E25 kitty_pushed 只写不读（terminal.rs:124/144/169/180/190）与 keys.rs:5 全局位双副本同根。

## 1. 目标与边界

- 引擎组件内状态类型化；公共组件行为不变（needs_specs_change: false）。
- 边界：不动 Component trait 与渲染管线结构；不与 c3285 已落的 SelectionCore/FocusState/EscWindow 重叠。

## 2. 决策摘要

| 项 | 方向 |
|---|---|
| kitty（E6+E25） | 删 terminal.kitty_pushed 死字段，keys.rs 全局 Mutex<bool> 为单一真值 |
| E23 | apply_probed 前校验 sources 边界；空源路径显式 no-op |
| E16 | pending_scroll 单真值：scroll_by 清 pending；Clip 模式消费或丢弃并记录 |
| E5 | follow_bottom 在 set_lines / viewport 变更点重算，at_bottom 派生 |
| E20/E13 | 渲染副产物与输入分离：layout 副产物改显式快照结构；脏标志簇单一 struct |
| E22 | loader tick/render 先看 aborted |
| E12 | paste_burst 三谓词合一 is_pasting |
| E8 | JumpDir 显式枚举 + 失焦清除 + cancel API |
| E10 | history isize 哨兵改枚举/Option，draft 单一生命周期 |
| E11 | last_action 改枚举，Editor/Input 共用定义（46 写点机械替换） |
| E9 | 自动补全 list+state 合一 Option<AutocompleteState> |
| E17/E18 | scroll_region/tabs 命中数据以单一持有为准 |
| E14 | 暂停旁路统一走 with_terminal_suspended |
| E15 | settings 子菜单显式关闭信号，字段拆双义 |
| E21 | 防御性：markdown 缓存键补 mermaid_mode / table_header_underline |

## 3. 实施顺序

见 tasks.md T1–T15（bug 类在前，机械替换居中，防御类收尾）。

## 4. 测试边界

组件单测 + 既有引擎 harness；行为不变由 package-tui BDD 守住。E23 补空源 panic 回归单测。
