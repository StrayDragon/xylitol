# Tasks

## T1 抽屉核心

- [x] 键位 `app.session.drawer`（alt+i；alt+d 与包级 editor 冲突已避让）入产品键位表
- [x] 根：`PendingSlotOps.session_drawer` + 显隐状态 / request-take-show-close API + `overlay_session_drawer` 渲染（右缘、muted、截断；先于通知栈绘制）+ 输入头切换/Esc 收起（先于槽派发）
- [x] host drain：pending → 身份供给上屏（`pending_ui`）
- [x] 包 `render_with_viewport_overlay` 探针去 cfg(test)（BDD 断言面）

## T2 合约与护栏

- [x] DESIGN 硬规则 2 例外注记（瞬态抽屉 ≠ 双栏 / 常驻侧栏）
- [x] `app-tui-fixed-zone` r2903 会话详情抽屉 + 场景（toggle 渲染 / 复按收起），host pump BDD 绑定

## T3 验证

- [x] `cargo test --all-features`（lib + bdd）全绿；clippy 零警告；`llman-sdd validate c2875-add-tui-sidebar-panes --strict` 绿
