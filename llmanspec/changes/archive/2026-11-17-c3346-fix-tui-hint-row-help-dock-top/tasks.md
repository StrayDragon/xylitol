# Tasks

- [x] t1 提示行恒占位:`hint_rows()` 恒 1、`render_hint_slot` 不可见时输出空白占位行、`reserved_lower_fixed_zone` 去 hint 形参并同步调用点与单测;既有 hint 可见性文案断言改为「占位行恒在、非空 editor 时该行为空白」(`crates/xylitol-app-tui/src/layout/`、`tests.rs`)。验证:`cargo nextest run -p xylitol-app-tui` 相关切片绿。
- [x] t2 帮助框移固定区顶:root dock 装配顺序 help 最先(队列条之前)+ 行锚记账同步;新增帧断言「开合帮助面板前后 status / tray / editor / footer / hint 绝对行号不变」;既有依赖 hint 行位置的 BDD helper 同步。验证:`cargo nextest run -p xylitol-app-tui` 全量绿 + fixed-zone BDD 场景绿。
- [x] t3 门禁:fmt / clippy / uncomment;`cargo nextest run --all-features --test bdd` 全量绿;`llman-sdd validate c3346-fix-tui-hint-row-help-dock-top --strict` 全绿(含 tasks 全勾)。
