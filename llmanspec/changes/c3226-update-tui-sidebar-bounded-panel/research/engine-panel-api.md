# 引擎有界面板（PanelRegion）调研与设计底稿

本文件是 change 作用域调研，不是 live spec。结论已摘要回写 `proposal.md` 的 Further Notes。

## 目标

- 引擎提供通用有界面板：宽 / 高约束、滚动、裁剪、四向 placement。
- app 用 `DockRight` 实现 transcript 内驻列；`FloatRight` 用于窄 / 矮降级；左栏后续复用同一组件。
- 不重命名既有 `Panel` 或其他组件；app 保持“会话侧栏 / 左栏”产品命名。

## 非目标

- 不做通用布局引擎 / `PanelSlot`。
- 不做内容长度截断或硬内容上限。
- 不做左栏产品接线（本 change）。

## 引擎 API 草案

~~~rust
pub struct PanelBounds {
    pub width: SizeValue,          // preferred
    pub min_width: usize,
    pub max_width: Option<usize>,
    pub min_height: usize,
    pub max_height: SizeValue,
    pub anchor: PanelAnchor,
    pub insets: PanelInsets,
    pub overflow: PanelOverflow,
}

pub enum PanelAnchor {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
    LeftCenter,
    RightCenter,
}

pub struct PanelInsets {
    pub top: usize,
    pub right: usize,
    pub bottom: usize,
    pub left: usize,
}

pub enum PanelOverflow {
    Clip,
    Scroll,
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
    available: Option<Rect>,
    last_rect: Option<Rect>,
}
~~~

方法草案：

~~~rust
impl PanelRegion {
    pub fn new(
        child: Box<dyn Component>,
        bounds: PanelBounds,
        placement: PanelPlacement,
    ) -> Self;

    pub fn set_available(&mut self, rect: Rect);
    pub fn rect(&self) -> Option<Rect>;
    pub fn set_scroll(&mut self, top: usize);
    pub fn scroll_by(&mut self, delta: isize) -> bool;
}

impl Component for PanelRegion {
    fn render(&mut self, width: usize) -> Vec<String>;
    fn handle_input(&mut self, event: InputEvent);
}
~~~

`OverlayOptions` 扩展：

~~~rust
pub struct OverlayOptions {
    pub width: Option<SizeValue>,
    pub min_width: Option<usize>,
    pub max_width: Option<usize>,   // 新增
    pub max_height: Option<SizeValue>,
    pub min_height: Option<usize>,  // 新增
    pub clip: Option<ScreenRect>,   // 新增
    pub anchor: Option<OverlayAnchor>,
    pub offset_x: Option<i32>,
    pub offset_y: Option<i32>,
    pub row: Option<SizeValue>,
    pub col: Option<SizeValue>,
    pub margin: Option<OverlayMargin>,
    pub non_capturing: bool,
}
~~~

`composite_overlays` 口径：

1. 以 `anchor` / `margin` 算初始矩形。
2. 与 `clip` 求交，得到可用矩形。
3. 宽度 clamp 到 `min_width` / `max_width`；高度 clamp 到 `min_height` / `max_height`。
4. 内容超出高度时按 `overflow` 裁剪或滚动；只合成可用矩形内的行。
5. `last_screen_rect` 记录裁剪后的实际矩形，供命中 / 选区使用。

## Placement 状态机

~~~text
closed
  │ toggle
  ▼
open_intent
  │ resize / open
  ├─ DockRight 可用（width_tier >= Standard，content >= 40，transcript >= 3）
  │    → DockRight
  ├─ FloatRight 可用（term_cols >= 26，transcript >= 3）
  │    → FloatRight
  └─ else
       → Hidden（不渲染；◨ 隐藏；切换静默；保留 open_intent）
~~~

空间恢复时：若 `open_intent` 为真且 DockRight 可用，自动回到 DockRight；否则回到 FloatRight；否则 Hidden。

## 渲染管线

### DockRight

- `content_w = term_cols - panel_w`。
- transcript 按 `content_w` 渲染；下缘固定区按 `term_cols` 渲染；面板按 `panel_w` 渲染。
- transcript 行拼接：`content_line + separator + panel_line`；固定区行保持全宽。
- 面板高度 = `clamp(content_rows, 3, min(60% × transcript_rows, 24))`；不足部分留空。
- 命中 / 选区：面板矩形 `row 0..panel_h, col content_w..term_cols`；separator 在 `content_w`；transcript 选区以 `content_w` 为界；固定区命中不受影响。

### FloatRight

- transcript 与固定区都按 `term_cols` 渲染，不重排。
- 面板作为 overlay，`clip` 到 transcript 矩形；宽度 `clamp(preferred, 24, term_cols - 2)`；高度 `clamp(content_rows, 3, min(60% × transcript_rows, 24))`。
- 面板只盖 transcript 右侧；固定区不被覆盖。
- 命中 / 选区：面板矩形捕获指针；面板带内滚轮只滚面板；transcript 选区不得穿透面板。

### Hidden

- 不渲染面板；全宽；`◨` 隐藏；切换静默；`open_intent` 保留。

## 约束表

| placement | 宽度 | 高度 | 可用条件 |
|---|---|---|---|
| DockRight | preferred / 24/32/40；min 24；max `term_cols - 40` | min 3；max `min(60% × transcript_rows, 24)` | `width_tier >= Standard`、content >= 40、transcript >= 3 |
| FloatRight | preferred；min 24；max `term_cols - 2` | min 3；max `min(60% × transcript_rows, 24)` | `term_cols >= 26`、transcript >= 3 |
| Hidden | — | — | 以上都不满足 |

## 命中 / 滚动 / 选区

- 面板矩形由 `last_rect` 提供；命中按实际矩形裁剪，不能沿用“整列到屏幕底”的旧口径。
- 面板带内滚轮：只滚面板内容；面板内容未溢出时保持 0。
- transcript 滚轮 / 选区：以内容右边界为界，不进入面板带。
- 固定区命中：仍按终端绝对行，不受面板影响。
- 待定：面板正文点击是否仍关闭侧栏。现有 r2904 写“点击侧栏带 须收起侧栏”；未来面板有业务内容时，建议只让 separator / 关闭 chip 关闭，正文点击留给内容交互与选区。

## Spec 落点草案

### app-tui-fixed-zone r2904

- 侧栏只占 transcript 区，不覆盖输入框与下缘固定区；固定区全宽。
- 宽度：preferred / 24/32/40 快捷档；拖拽 separator 在 min/max 内调任意宽度；DockRight 保证内容区至少 40 列。
- 高度：min 3；max `min(60% × transcript_rows, 24)`；内容超出滚动；内容不足收口。
- 降级：DockRight → FloatRight → Hidden；保留 open_intent；空间恢复自动回到 DockRight；切换静默。
- 命中 / 选区 / 滚轮按面板矩形裁剪；固定区不受影响。
- 顶部 loaded-resources 卡片不单独避让。

### package-tui-engine

- 新增 `PanelRegion` / `PanelBounds` / `PanelPlacement` 公开行为：宽度 / 高度 clamp、overflow、四向 placement。
- `OverlayOptions` 新增 `max_width` / `min_height` / `clip`；合成只落在 clip 内；`last_screen_rect` 为裁剪后矩形。

### package-tui-pointer

- 面板矩形命中 / capturing 口径；面板带内滚轮与选区不穿透。
- overlay 几何以裁剪后矩形为准；固定区命中语义不变。

## 性能

- DockRight：打开 / 关闭 / 拖拽宽度触发一次 transcript 重排；upper cache 按宽度失效；不做逐帧重排。
- FloatRight：不重排 transcript；只合成面板矩形内的行。
- 面板渲染缓存按宽度 / 高度 / 内容版本失效；内容未变不重渲。
- 命中区从 `last_rect` 现算，不整屏扫描。
- 宽度拖拽期间可只重绘面板带与 separator，释放时再收口持久化。

## 测试计划

- 引擎单测：宽度 / 高度 clamp、min/max、overflow Clip/Scroll、overlay clip 求交、`last_screen_rect`。
- app harness：DockRight / FloatRight / Hidden 三态 snapshot；resize 迁移与 open_intent 自动恢复；拖拽宽度 clamp；命中 / 滚轮 / 选区边界；全局持久化。
- BDD：更新 `app-tui-fixed-zone` 场景；包侧 `package-tui-engine` / `package-tui-pointer` 场景。
- 真终端 E2E：可选，覆盖 resize 与鼠标拖拽。

## 待定实现问题

1. `Component::render(width)` 只收宽度；`PanelRegion` 需要 `set_available(rect)` 还是新增 render 上下文方法？
2. DockRight 拼接由 app helper 做（Q2 已选）；helper 如何复用 dock 行全宽渲染？
3. FloatRight 绘制走引擎 overlay 还是 `overlay_viewport`？Q2 选 overlay + clip；app 命中 / 滚轮如何与引擎 overlay 矩形对齐？
4. 面板正文点击是否关闭；建议只 separator / 关闭 chip 关闭。
5. 面板内容选区 / 复制是否纳入本 change；当前占位内容可能不需要。
6. 持久化 key 命名与 schema 版本；`sidebar-width` / `sidebar-height` 是否够。
7. fallback 决策点：FloatRight 复杂度超预期时，何时退回“仅 DockRight + 压缩驻列”。
8. 左栏产品接线：后续复用 `PanelRegion` + `DockLeft` / `FloatLeft`，本 change 不做。
