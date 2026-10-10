# 设计：引擎指针命中管线

## 1. 决策表

| # | 决策 | 理由 |
|---|---|---|
| D1 | 相位用枚举 `DownSwallow`（默认）\| `UpIfEmptySelection`，不是两个回调 | 默认必须钉死今日 ath33；opt-in 一处切换，避免 host 漏接 Up |
| D2 | hover tint **独立层**，不复用 selection invert | invert 是选区语义；hover 是可点暗示。混用会导致「悬停看起来像选中」 |
| D3 | 命中几何用 content 坐标 `{id: u64, row, col_start, col_end, height}`，事件时读 live `scroll_top` | host 拷贝的 scroll_top 会在 wheel 与下一帧之间过期 |
| D4 | `HitPriorityFn` 保留为 `id.is_some()` 薄适配 | 一次打碎 c2040 / 现有包测成本高；c2860 再迁产品表 |
| D5 | overlay / capturing region **先于** transcript 选区 | 今日 AO 鼠标在 overlay 之前消费；不先做 capture，抽屉/pan 永远点穿 |
| D6 | 武装探针后 Moved 才进引擎；id 未变不排帧；变则 reproject + 可见行 overlay，禁止 `ao_components_stale` | 守 r1632；学 `apply_highlight` |
| D7 | **不**引入 ratatui `Buffer` / `Constraint` / widget 树 | D12；产品 transcript 仍是扁平 ANSI |
| D8 | 产品 fan-in 仍丢 Moved（ath28） | 本票只保证 `dispatch_event` 直驱可测；停丢归 c2860 |
| D9 | 不写入 `FoldTarget` / 三角列 / 产品 toggle | 产品词表留 c2860 |
| D10 | Host 口语 API `Hit::on_line` / `Hit::at_cell` + `TUI::mark_hit`；`HitRegion` 是引擎存储 | `src/` 作为库用户不应自算 `content_row = scroll_top + screen_row`。Viewport-sticky 给滚动条按钮等钉屏控件 |

## 2. 事件序（默认 = 今日）

```
Moved ── 探针未武装 ──► 丢（r1632）
      ── 已武装 ──► hit id；id 变 → cheap reproject；未变 → 不排帧
Down  ── capture/overlay? ──► latch，不启选区
      ── DownSwallow + hit ──► swallow（今日）
      ── 否则 ──► 启拖选
Drag  ── 永不 click；capture 则吞
Up    ── DownSwallow：不报 click（包测继续绿）
      ── UpIfEmptySelection：无选区且未拖 → click id；空选不 OSC52
```

## 3. 公开缝（包 API，产品类型不进入）

Host / 包测只依赖：

- `TUI::dispatch_event(InputEvent::Mouse)`（已有）
- 命中登记：`TUI::mark_hit(Hit::on_line / at_cell)`（本票新增；`HitRegionList` 是引擎存储）
- click 相位 + 探针武装（本票新增，名称以代码为准）
- 可观察：click id、hover id、是否启拖、是否 OSC52、`InputReaction`、组件是否被标 stale

`FoldHitTable` 仍在 `src/app/tui`。本票产品代码 **零改**。

## 4. spec 映射

独立 capability `package-tui-pointer`，不塞进 `package-tui-interaction-modes`：后者是双模式生命周期与选区；本票是新的指针缝。文件在嵌套包实例（`packages/xylitol-tui/llmanspec/specs/package-tui-pointer/`）；根树不复写 `@req`（跨根 id 必须唯一）。

| 新 @req | WHAT |
|---|---|
| r1931 | 不透明 id 的命中区；正文行用 live scroll，可见格钉屏 |
| r1932 | 默认 DownSwallow；opt-in UpIfEmptySelection |
| r1933 | 探针武装后 Moved；同 id 不排帧；同一鼠标输入管道 |
| r1934 | capture/overlay 先于选区 |
| r1935 | hover 绘出 MUST 走可见行 overlay / reproject，MUST NOT 整段组件 stale |

r1632 **不改句**：无态 Moved 仍不得强制整帧；新条款是同一管道上的 opt-in。

## 5. 测试缝

复用：`packages/xylitol-tui/tests/interaction_modes_test.rs`、`selection.rs` 单测、`tests/bdd/steps_package_tui_interaction_modes.rs`（`dispatch_event` / `SelectionController` / `RecTerm`）。不新开 harness crate。BDD 新场景在 apply 绑到既有步骤文件。
