# Design: c1505 entry-level scrollback viewport slice

## Problem

`render_scrollback` 每帧：

1. 遍历**全部** `UiModel.entries`
2. cache hit 仍把该 entry 的全部行 `extend` 进输出 `Vec<String>`
3. 上层差分引擎再对整份 upper 做视口写屏优化——**不省**本函数的 CPU / 分配

c1500 解决的是「重复 Markdown paint」；本 change 解决「屏外历史仍进 flatten」。

## 当前现实（post-c2070/c2040）

产品已经由 c2071 固定为 ApplicationOwned。当前链路是：

```text
UiRoot::render
  → render_scrollback：完整 entries → 完整 component lines
  → ApplicationOwnedRuntime::project_frame：完整 content → ScrollView
  → paint_visible：只把可见窗口 + dock 交给终端
```

滚轮和拖选走 `reproject_frame`，不重新调用 component render；`ScrollView` 持有完整 transcript 行，应用内选区使用完整内容坐标。c2040 的 `FoldHitTable` 也记录完整内容坐标，再由 `scroll_top` 映射屏幕命中。因此「切片后的 Vec 直接替换完整 ScrollView 内容」不是兼容实现。

## Approach A（候选，尚未选定）

```text
entries[0..N]  →  per-entry line counts (from ScrollbackPaintCache)
total_lines    →  sum(+ spacers)
follow-bottom  →  start_line = max(0, total - viewport_h - overscan)
               →  skip entries fully above window; partial first entry if needed
streaming_*    →  always append after historical window (live tail)
```

该算法只有在新的 viewport-aware / lazy-content seam 下才成立。必须同时保留完整逻辑内容长度、scroll_top、selection 坐标和 fold-hit 坐标；不能把窗口局部行当作新的完整 transcript。

### Seam

| 层 | 职责 |
|---|---|
| `widgets/scrollback` | 提供 entry 行数 / 窗口 paint 能力；具体签名待 seam 决策，不能先钉孤立 `scroll_line_offset` |
| `UiRoot` / host | 提供 AO 的 viewport 高度、逻辑 scroll_top、follow-bottom 与 fold/宽度失效信号 |
| `xylitol-tui` AO runtime | 继续持有完整逻辑内容、selection 坐标和 cheap reproject；必要时增加 lazy/content-provider 口，而不是接收不完整 transcript |
| 差分引擎 | 不变；终端仍只接收 ≤ height 的最终 paint surface |

### c2040 fold-hit 不变量

- 可见折叠三角必须仍映射到完整 transcript 的 content row。
- AO wheel / selection reproject 不得因为没有重新 render 而丢失当前可见 fold hit。
- fold 改变块高后必须同时重算窗口边界、`scroll_top` 夹值与命中表；不能只清局部 paint cache。

### Non-goals

- Codex `insert_history` 分裂
- 按字符估高的虚拟列表
- 去掉宽度不变量
- 把 ApplicationOwned 的完整逻辑 transcript 改成只保存当前窗口
- 为每个 AO wheel 事件强制完整 component render

## Risks

| 风险 | 缓解 |
|---|---|
| 上滚空白或回底错位 | 先确定完整逻辑内容与窗口 paint 的 seam；overscan（≥1 屏）只作为补充 |
| fold 改高度 | fold 变更同时 bump generation、重算逻辑长度 / `scroll_top` / hit table |
| 流式空尾 | streaming tails 不参与「可跳过」集合 |
| cheap reproject 退化 | AO wheel / selection edge scroll 必须保持不触发完整 component render |
| 选区或折叠命中错坐标 | 以完整内容坐标为唯一映射；增加上滚、拖选、三角命中回归 |

## Verification

**当前阶段（pre-start）**

1. 真实 ApplicationOwned 长历史 + 长流 profile：比较历史规模增长时的帧耗时、CPU 与 `component_lines`，不能使用 B-scroll 假阴性替代。
2. 明确验收预算：是以实际帧成本下降为主，还是以输出行数 / flatten 工作集上界为主；没有预算不 promote。

**若 promote 后**

1. Harness：大 N entries，follow-bottom 时 paint 工作集 ≤ viewport + overscan + streaming；完整逻辑内容仍可上滚。
2. AO wheel / edge selection：保持 cheap reproject、选区跨页与回底跟随。
3. c2040：可见三角命中、正文误点不 toggle；fold 高度变化后 `scroll_top` / hit table 对齐。
4. 可见内容 ath/PTY：首屏、贴底、上滚和流式尾无缺字。
5. `profile-suite` E-hist-stream 或等价 AO probe：历史放大后 flatten/clone 的绝对成本或工作集有可解释下降。
