# Tasks

- [ ] T1 specs：`app-tui-fixed-zone` r2904 补标准档门槛 + 降级条款 + 场景
      `sidebar-below-standard-tier-falls-back-to-popup`（新 given「窄终端」与
      then「弹出框上屏侧栏不出」两步；when 复用「切换侧栏」）。
- [ ] T2 门槛与口径：`session_sidebar_layout` 挂 `width_tier >= Standard`；
      `session_sidebar_reserved_cols(width)` 由 layout 推导；
      `sync_session_sidebar_reserve` 以 `Terminal::columns` 对账。
      [blocked-by: T1]
- [ ] T3 降级与同步件：`drain_pending_ui` 侧栏 pending 标准档以下改开覆盖
      弹出框；designing draft/intent 与 DESIGN.md 档位表补侧栏行。
      [blocked-by: T2]
- [ ] T4 验证：布局边界单测（79/80 与 reserve 派生、关态）；BDD 窄终端场景；
      `just test` + `just test-tui` 全绿。[blocked-by: T3]
