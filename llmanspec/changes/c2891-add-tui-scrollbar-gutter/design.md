# Design — c2891 右缘滚动槽（^ | v）

本文全部结论来自代码深挖（2026-10-11，main @ post-c2893）；行号可漂移，以符号为准。

## 0. 可行性裁定

**能做，无阻塞。** 三根支柱全部现成：

| 支柱 | 真值 |
|---|---|
| 命中管线（c2858 已归档） | `packages/xylitol-tui/src/pointer.rs`：`Hit::at_cell` → `HitSpace::Viewport`，doc 原文即 "scrollbar `^`/`v`, overlay chrome"；`.capturing()` latch 保证 Left Down 不启拖选 |
| 滚动原语 | `scroll_view.rs` / `application_owned_runtime.rs`：`scroll_top()` `scroll_to_end()` `scroll_by` `max_scroll` `at_bottom` 齐备 |
| 产品接线通路 | host `sync_pointer_hits`（每帧 paint 后全量 replace）→ `take_pointer_click`（host/mod.rs 单一 match 点） |

外部参照（pi `scroll-view.ts` + `getScrollbarGeometry`）仅用于 thumb 几何常识；本票 v1 无 thumb，不抄结构。

## 1. 数据流三链（落点）

### 写链（paint stamp）

AO 渲染双路（`tui/render.rs do_render`）：
- 全渲染：components.render(w) → `runtime.project_frame(lines, h)`
- wheel 快路径：`runtime.reproject_frame(h)`

两路**必经** `ApplicationOwnedRuntime::paint_visible`（application_owned_runtime.rs）：
`visible = scroll.visible_lines()` → selection highlight → hover tint → pad/truncate → dock。

**D1 拍板：stamp 落点 = `paint_visible` 尾部**（hover tint 之后、pad 之前）。
理由：单点覆盖双路；engine 此处持有全部判定状态（`scroll_top` / `viewport_height` / `content_len` / 开关）；visible 是 copy（`to_vec`），stamp 幂等无副作用。产品 `UiRoot::overlay_viewport` 钩子拿不到 scroll_top/content_len，**不采用**。

### 读链（hit 注册）

每帧 paint 成功后：host `sync_fold_hit_viewport` → `sync_pointer_hits` → `tui.replace_hits(...)` 全量替换。
Viewport 命中在引擎 `hit_at` 里与 scroll 无关（screen row 直配），故**注册几何恒定**：`(last_col, row 0)` 与 `(last_col, transcript_rows-1)`，随每帧 replace 自然跟随窗口尺寸。proposal 要求的「live 重登」由既有每帧 replace 免费满足。

**D3 拍板：gutter hit 走独立表 + disjoint id 常量。**
`FoldHitTable` id 空间 = `index+1`（Content space，`pointer_click_hits` 只发 `on_line`）。gutter 是 Viewport space 且动作语义非 fold → 新 `scroll_gutter` 产品侧小表，id 用固定常量（`GUTTER_TOP`/`GUTTER_BOTTOM`，取 `1<<62` 起，与 fold index 空间天然不交）。host `sync_pointer_hits` concat 两表；`take_pointer_click` 处先查 gutter 表再回落 fold（一处 match 改两行）。

### 动作链（click → scroll）

host left_up 分支 `take_pointer_click` 命中 gutter id → `scroll_top=0` / `scroll_to_end`（复用 `application_owned_scroll_by`/runtime 方法）→ 触发 repaint 走既有 `request_render` 路径（同 wheel：`can_reproject` 快路径，`ao_wheel_render_requested` 同族）。**不** `mark_ao_components_stale`（组件树没变，reproject 足够——与 wheel 同待遇）。

## 2. 宽度 carve（−1 列）

**D2 拍板：carve 落点 = `do_render` 入口**：AO + gutter 开启时 `render_w = terminal.columns() - 1`，组件仍按 `render_w` render（上层 `upper_cache_width` 按 width key，自动失效重排；包层 diff/markdown 按传入宽自适应，无全局阈值硬编码）。

已核实安全边界：
- finalize 宽度不变量是 `lw > width → Err`（**≤** 非必须满宽）——carve 后行宽 `render_w < width` 合法；stamp 后 gutter 列恰补到 `width` 也合法
- `can_reproject_application_owned` 的 `previous_width == width` 比较：**统一改比 carve 后 render_w**（否则 resize 检测与内容宽错位）。此接缝是全票唯一需测试钉死的点 → tasks 刀 2
- selection `handle_mouse` 的 transcript `ScreenRect.width: term_cols` → gutter 开启时同 carve（`render_w`），gutter 列自身由 capture hit skip，双保险

## 3. 交互语义拍板

| Open Question（proposal） | 拍板 | 依据 |
|---|---|---|
| thumb？ | v1 无，`^ │ v` 三段；中轨整列 `.capturing()` 吞拖选、无点击动作 | proposal 草案默认；thumb 几何后票（pi 公式已存此档） |
| 中轨点击？ | 忽略（capture 仅防误选区，不 page） | proposal「先只吞拖选」 |
| 隐藏阈值？ | `content_len ≤ viewport_h` 或 `render_w < 20` 隐藏（隐藏时同时不 carve、不注册 hit） | 内容不满屏无滚动语义；20 = 极窄档下限（对齐 c2520 档位表精神，窄屏不挤内容） |
| phase？ | 沿用产品现值 `UpIfEmptySelection`（host 已设）+ hit `capturing=true` | Up+无拖才报 click：拖选路过 gutter 不误触发跳转；Down capture 不启选区。两机制正交，各管一件事 |
| 产品开关 | host 构造处 opt-in 常量，**产品默认开**（仅 ApplicationOwned；Inline 永远走终端 scrollback 不接） | proposal「包层默认关 host opt-in」＝引擎 API 契约；产品价值要可观察 |

## 4. 悬置（apply 时核实，不阻塞）

- `selection.set_hit_priority`（c2040 旧 hook）与 c2858 pipeline 的分工：gutter 预期只走 pipeline capture；若 selection 另有「Down 落点优先级」需同步，在刀 4 实测补
- hover：v1 不进 `pointer_hover_hits`（无 tint）；
- reproject 快路径上 `ao_wheel_render_requested` 与 click 触发的 repaint 合流点

## 5. 测试策略

- 包层单测：stamp 几何（顶/底/中轨字符、隐藏阈值）、carve 后 reproject 一致、`at_cell` capture 命中（管线已有测试模式照抄）
- BDD：`packages/xylitol-tui/llmanspec/specs/package-tui-scrollbar-gutter/`（新 capability）+ `tests/bdd/bindings_package_tui_scrollbar_gutter.rs`（照 `bindings_package_tui_pointer.rs` 的 `#[scenario(path=packages/...)]` 模式）
- 产品接线：`app-tui-transcript` 增补规则 + `interaction_scene`（`left_click` 已通）断言点击 ^ 后 `scroll_top==0`
- 门禁：`just qa`

## 6. 明确不做

- thumb / 拖 thumb / page-up-down 中轨 / 宽度动画 / overlay 抽屉 / Workspace 侧栏（c2875 park 不动）
- Inline 模式接线
- DESIGN「无双栏」修订（gutter 是 transcript 视口右缘 1 列，非第二栏）
