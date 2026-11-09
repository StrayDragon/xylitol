# Design — 会话侧栏有界尺寸与固定区避让

## 1. 目标与边界

- 引擎新增通用有界面板 `PanelRegion`：宽 / 高约束、滚动、裁剪、四向 placement。
- app 用 `DockRight` 把侧栏限制在 transcript 区；`FloatRight` 用于窄 / 矮降级；下缘固定区保持全宽。
- 左栏只做 API 准备，本 change 不接产品。
- 不重命名既有 `Panel`；app 保持“会话侧栏”命名。
- 顶部 loaded-resources 卡片不单独避让；只避开下缘固定区（队列条 / 待办栏 / 通知条 / status / tray / footer）。

## 2. 决策摘要

| 决策 | 内容 |
|---|---|
| 布局 | app 用 B（transcript 内驻列）；引擎同时支持 A/B |
| 引擎形状 | `PanelRegion` + `PanelBounds` + `PanelPlacement::{DockLeft, DockRight, FloatLeft, FloatRight}`；`OverlayOptions` 补 `clip` / `max_width` / `min_height` |
| 高度 | 可见高度上限 + 内部滚动；默认 `max_height = min(60% × transcript_rows, 24 行)`，`min_height = 3 行` |
| 宽度 | preferred + min/max；24/32/40 快捷档；拖拽 separator 调任意宽度 |
| 持久化 | 宽度 / 高度上限存全局 key（client-local 存储），所有会话共享 |
| 降级 | DockRight → FloatRight → Hidden；保留 `open_intent`，空间恢复自动回到 DockRight |
| 阈值 | DockRight：`width_tier >= Standard`、content >= 40、transcript >= 3；FloatRight：`term_cols >= 26`、transcript >= 3、面板宽 `clamp(preferred, 24, term_cols - 2)` |
| Specs | `app-tui-fixed-zone` + `package-tui-engine` + `package-tui-pointer` |

## 3. 引擎设计

### 3.1 类型

~~~rust
pub struct PanelBounds {
    pub width: SizeValue,
    pub min_width: usize,
    pub max_width: Option<usize>,
    pub min_height: usize,
    pub max_height: SizeValue,
    pub anchor: PanelAnchor,
    pub insets: PanelInsets,
    pub overflow: PanelOverflow,
}

pub enum PanelPlacement {
    DockLeft,
    DockRight,
    FloatLeft,
    FloatRight,
}

pub struct PanelRegion {
    child: Box<dyn Component>,
    bounds: PanelBounds,
    placement: PanelPlacement,
    scroll: ScrollView,
    available: Option<ScreenRect>,
    last_rect: Option<ScreenRect>,
}
~~~

- `PanelRegion::render(width)`：宽度 clamp 到 `min_width` / `max_width`；子组件按该宽度渲染；高度 clamp 到 `min_height` / `max_height` 与 `available`；超出按 `overflow` 裁剪或滚动；内容不足按实际高度收口。
- `PanelRegion::rect()` 返回上次渲染的实际矩形，供命中 / 选区 / 滚轮使用。
- 内容滚动复用 `ScrollView`；若 c3227 的 `ScrollRegion` 适配，则组合它，不新增第二套滚动状态。
- 引擎不硬编码产品动作 id；键盘 / 滚轮由 app 传入或由 app 路由。

### 3.2 Overlay 扩展

- `OverlayOptions` 新增 `max_width` / `min_height` / `clip`。
- `composite_overlays`：先按 anchor / margin 算矩形，再与 `clip` 求交；宽度 / 高度 clamp；只合成可用矩形内的行；`last_screen_rect` 记录裁剪后矩形。
- 不得把 overlay 合成到 `clip` 之外；不得越过下缘固定区。

### 3.3 Placement 状态机

~~~text
closed
  │ toggle
  ▼
open_intent
  │ open / resize
  ├─ DockRight 可用 → DockRight
  ├─ FloatRight 可用 → FloatRight
  └─ else → Hidden（不渲染；◨ 隐藏；切换静默；保留 open_intent）
~~~

- 空间恢复时：`open_intent` 为真且 DockRight 可用 → 自动回到 DockRight；否则 FloatRight；否则 Hidden。
- 用户关闭：`open_intent = false`，回到 closed。

### 3.4 命中 / 滚动 / 选区

- 命中按 `last_rect` 现算，不按“整列到屏幕底”的旧口径。
- 面板带内滚轮只滚面板；内容未溢出时保持 0。
- transcript 滚轮 / 选区以内容右边界为界，不进入面板带。
- 固定区命中仍按终端绝对行，不受面板影响。
- 面板正文点击语义待定：现有 r2904 写“点击侧栏带 须收起侧栏”；建议只让 separator / 关闭 chip 关闭，正文点击留给内容交互与选区。

## 4. 应用端设计

### 4.1 渲染管线

- 计算 `panel_w`、`panel_h`、`transcript_rows`、`content_w`。
- `DockRight`：transcript / loaded-resources 按 `content_w` 渲染；下缘固定区按 `term_cols` 渲染；面板按 `panel_w` 渲染；transcript 行拼接 `content_line + separator + panel_line`；固定区行全宽。
- `FloatRight`：transcript 与固定区按 `term_cols` 渲染；面板作为 overlay，`clip` 到 transcript 矩形；不重排 transcript。
- `Hidden`：全宽；不渲染面板。
- 不再由 `overlay_viewport` 把侧栏带画到所有行；DockRight 在 render 阶段拼接，FloatRight 走引擎 overlay。

### 4.2 宽度与高度

- 宽度：preferred + min/max；DockRight 的 `max_width = term_cols - 40`；FloatRight 的 `max_width = term_cols - 2`。
- 高度：`min_height = 3`；`max_height = min(60% × transcript_rows, 24)`；内容不足按实际高度收口。
- 拖拽 separator：命中列 `content_w`；拖动更新宽度并 clamp；释放时持久化；拖动期间只重绘面板带与 separator。
- 快捷档 24/32/40 仍经 `app.session.sidebarWidth` 循环。

### 4.3 状态与持久化

- `UiRoot` 持有 `open_intent`、`placement`、`panel_width`、`panel_height_cap`、`scroll`。
- 持久化 key `sidebar-layout`：`{ width, height_cap }`；缺失 / 损坏回落默认（宽度 24，高度 60% / 24 行）。
- 切换会话不改变布局；所有会话共享同一 key。

### 4.4 降级与入口

- `◨` 在 DockRight / FloatRight 时反色激活；Hidden 时隐藏；窄档 < 80 列仍隐藏入口。
- 降级切换静默，不推通知条；保留 `open_intent`。
- fallback：若 `FloatRight` / Hidden 复杂度超预期，apply 阶段退回“压缩保持驻列”（仅 DockRight），`FloatRight` 后置接线。

### 4.5 固定区与宿主让宽

- 保留 `extra_reserved_cols = panel_w` 用于 transcript 渲染宽度与 gutter 定位；app 在 dock 行改用 `term_cols`，避免固定区一起让宽。
- `session_sidebar_reserved_cols` 只对 DockRight 返回 `panel_w`；FloatRight / Hidden 返回 0。
- 滚动槽贴 `content_w`（侧栏左缘）；侧栏带内不画滚动条。

## 5. Spec 影响

### app-tui-fixed-zone r2904

- 侧栏只占 transcript 区，不覆盖输入框与下缘固定区；固定区全宽。
- 宽度 / 高度约束、拖拽、快捷档、滚动、降级与 `open_intent` 自动恢复。
- 命中 / 选区 / 滚轮按面板矩形裁剪。
- 顶部卡片不单独避让。

### package-tui-engine

- 新增 `PanelRegion` / `PanelBounds` / `PanelPlacement` 公开行为。
- `OverlayOptions` 新增 `max_width` / `min_height` / `clip`；合成只落在 clip 内；`last_screen_rect` 为裁剪后矩形。

### package-tui-pointer

- 面板矩形命中 / capturing 口径；面板带内滚轮与选区不穿透。
- overlay 几何以裁剪后矩形为准；固定区命中语义不变。

## 6. 性能

- DockRight：打开 / 关闭 / 拖拽宽度触发一次 transcript 重排；upper cache 按宽度失效；不做逐帧重排。
- FloatRight：不重排 transcript；只合成面板矩形内的行。
- 面板渲染缓存按宽度 / 高度 / 内容版本失效；内容未变不重渲。
- 命中区从 `last_rect` 现算，不整屏扫描。
- 宽度拖拽期间只重绘面板带与 separator，释放时再收口持久化。

## 7. 测试 seam

- 引擎单测 / snapshot：`PanelRegion` 宽度 / 高度 clamp、overflow、overlay clip 求交、`last_screen_rect`。
- app harness：DockRight / FloatRight / Hidden 三态 snapshot；resize 迁移与 `open_intent` 自动恢复；拖拽宽度 clamp；命中 / 滚轮 / 选区边界；全局持久化。
- BDD：更新 `app-tui-fixed-zone` 场景；包侧 `package-tui-engine` / `package-tui-pointer` 场景。
- 真终端 E2E：可选，覆盖 resize 与鼠标拖拽。
- 门禁：根 `llman-sdd validate c3226-update-tui-sidebar-bounded-panel`；包 `llman-sdd validate --check --directory crates/xylitol-tui`。

## 8. 风险与待定

1. `Component::render(width)` 只收宽度；`PanelRegion` 需要 `set_available(rect)` 还是新增 render 上下文方法？
2. DockRight 拼接由 app helper 做（Q2 已选）；helper 如何复用 dock 行全宽渲染？
3. FloatRight 绘制走引擎 overlay 还是 `overlay_viewport`？Q2 选 overlay + clip；app 命中 / 滚轮如何与引擎 overlay 矩形对齐？
4. 面板正文点击是否关闭；建议只 separator / 关闭 chip 关闭。
5. 面板内容选区 / 复制是否纳入本 change；当前占位内容可能不需要。
6. 持久化 key 命名与 schema 版本；`sidebar-layout` 是否够。
7. fallback 决策点：FloatRight 复杂度超预期时，何时退回“仅 DockRight + 压缩驻列”。
8. 左栏产品接线：后续复用 `PanelRegion` + `DockLeft` / `FloatLeft`，本 change 不做。
9. FloatRight 的 overlay 是否 `non_capturing`：侧栏不抢键盘焦点时宜非捕获，但滚轮 / 命中仍由 app 路由。

## 9. 参考

- API 草案、状态机、效果示例与测试底稿：`research/engine-panel-api.md`。
- 决策全文：`proposal.md`。
