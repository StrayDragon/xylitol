# Design：驻列 = 内容区让宽的第二列（非 overlay）

## 关键决策

- **重排而非覆盖**：`UiRoot::render` 顶部单点改道——`effective = width − panel_w`（panel_w = min(24, width/3)，仅驻列开启时），全部 section 以 effective 渲染；upper 缓存以内容宽为键，切换即失效重建一次（wrap 正确换行，非截断）。收起恢复全宽。
- **右缘绘制**：每行 pad 到全宽后，右缘 `panel_w` 列绘 muted `│` 分隔 + 信息列（标题 ┌ 会话详情 ─ + 四行字段），余行留白；整高、无阴影卡片。
- **overlay 收口**：通知栈 / 瞬态抽屉等 viewport overlay 的右对齐宽度改用内容宽（`session_dock_layout` 纯函数两处共用）——overlay 永不压驻列。
- **键位**：`app.session.sidebar`（alt+u；全表无占位）二态切换，与抽屉 `alt+i` 独立；开启路径走既有 host drain 握手供给会话身份（`PendingSlotOps.session_sidebar`）。
- **状态机**：root `sidebar_on` + pending；驻列与抽屉互不改对方状态；id 共享一份（任一 show 刷新）。

## 已知取舍（记录不修）

- host 层宽度档（width_tier / TooSmall）仍按物理全宽判定：极窄屏开驻列时内容档位与视觉宽存在一档偏差；待实机反馈再定口径。
- AO 选区 x 映射按内容坐标：驻列右缘的选区行为为穿透（与抽屉同款已知限制）。
- 驻列开启时若同时开瞬态抽屉，抽屉贴内容右缘、驻列之外，两者并存不重叠。

## 落点

`src/app/tui/layout/root/{mod,render,slot_input,pending_slot}.rs`、`effects/pending_ui/mod.rs`、`keybindings.rs`；spec `app-tui-fixed-zone` r2904；BDD 复用 host pump 帧探针。
