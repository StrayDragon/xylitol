# Tasks

## T1 驻列核心

- [x] 键位 `app.session.sidebar`（alt+u）入产品键位表
- [x] 根：`PendingSlotOps.session_sidebar` + `sidebar_on` 状态 / request-take-show-close API + `session_dock_layout` 纯函数（内容宽口径）+ render 顶部单点让宽 + 右缘整高绘制（`│` 分隔 + 信息列）+ 输入头切换
- [x] overlay 收口：通知栈 / 瞬态抽屉右对齐改内容宽
- [x] host drain：`pending_ui` 驻列分支（身份供给）

## T2 合约与护栏

- [x] `app-tui-fixed-zone` r2904 会话侧栏 + 场景（驻列让宽 / 复按恢复），host pump BDD
- [x] 与 c2875 抽屉场景共存回归（互不扰动）

## T3 验证

- [x] lib + bdd 全绿；clippy 零警告；`llman-sdd validate c2905-add-tui-session-sidebar --strict` 绿
