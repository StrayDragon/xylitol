# Design：门槛、降级落点与口径

- **门槛挂档位表**：`width_tier(width) >= WidthTier::Standard`（80 列起），
  不比较裸列数（tier.rs 纪律）。宽度口径 = 物理 `Terminal::columns`：
  与引擎传给 `UiRoot::render` 的宽同源（BDD render probe 120 = 物理宽），
  边界不漂移。
- **降级落点在 host drain**（`drain_pending_ui` 侧栏 pending 分支）：root
  不引入宽度状态；drain 处可取 `Terminal::columns`，且身份供给本就在 drain。
  标准档以下侧栏 pending → `show_session_popup`。root 的 alt+u 分支不动
  （开着的仍收起；未开仍请求侧栏）。
- **保留常量 24**：可预测（同键位跨机器同宽）+ 引擎 seam 对账简单（c2910
  动机）；不回比例宽度。
- **reserve 单源**：`session_sidebar_reserved_cols(width)` 由
  `session_sidebar_layout(width).0` 推导；`sync_session_sidebar_reserve`
  每轮 drain 以物理宽对账，窄档推 0、回宽自愈。
- **缩窄行为**：宽屏开启后缩到标准档以下——渲染层关带（layout off）、
  `session_sidebar_on` 状态保留，回宽后 drain 对账恢复；期间 alt+u 先收状态。
