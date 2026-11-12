# Tasks — TUI 引擎指针 / 选区 / 焦点状态修复

> 实施者导读：design §7 是复核结论（hidden 可聚焦与旧矩形命中部分不成立；指针不新建单枚举）。收口走 finalize，不列为任务。

- [x] T1 spec delta：app-tui-mouse-actions r3336 捕获回收 / r3337 gutter 复位；app-tui r3338 overlay 焦点单一真值；app-tui-ask r3339 Review 显式页
- [x] T2 指针：capture 时间戳 + 750ms 回收；set_phase 清 pending_click；Down/Up 生命周期；单测（丢 Up、相位改写）
- [x] T3 选区：SelectionCore 抽取，引擎与 editor 薄包装接入；点击计数/bounds 单份；行为对照单测
- [x] T4 焦点：FocusState 单字段 + reconcile_focus 单一对账方法；hidden 不可聚焦不变量测试
- [x] T5 ChoiceTab 枚举：消灭 tab==len 越界编码；Review 下标安全单测
- [x] T6 gutter：新 Down / 指针移出列复位 grab；单测
- [x] T7 回归：just fmt / lint / test / qa；llman-sdd validate c3285 --strict 绿
