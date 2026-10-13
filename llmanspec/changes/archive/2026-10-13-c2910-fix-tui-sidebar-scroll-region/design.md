# Design：驻列滚动分区与 gutter 贴靠

- **单一边界口径**：包层 `ApplicationOwnedRuntime.content_right(term) = term - reserved - extra`——选中 rect、滚轮分区、gutter stamp/hit/drag 全部同源；`extra` 为宿主侧栏让宽（c2891 预留 seam 的兑现）。
- **宿主对账**：每轮 drain `sync_session_sidebar_reserve()` 把根常量（24，`SESSION_SIDEBAR_W`）推给包层；开关切换 / 会话重建自愈；`Tui` 门面 + begin 双重放同 c2891 模式。
- **滚轮分区**：驻列带内滚轮不消费（不滚正文、不进选区）；gutter 列与内容列照旧。选区 rect 同步收窄，拖选不入驻列。
- **验证**：包层单测 `extra_reserved_owns_sidebar_band`（带内滚轮不消费/不动、内容照常、边界公式）+ c2891 全量回归（extra=0 行为不变）+ app bdd 回归。BDD 场景不重复包层几何（r2904 规则文本已述，unit 为 SSOT 执行层）。
