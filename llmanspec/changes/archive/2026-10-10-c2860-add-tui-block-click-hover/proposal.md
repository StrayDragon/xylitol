---
depends_on:
  - c2858-add-package-tui-pointer-hit-pipeline
branch: sdd/c2860-add-tui-block-click-hover
base_branch: main
base_sha: 2c9b6fa9d826ccff6ebd18df52303a108b49062d
---

# 整块可点折叠与悬停可发现性

## Why

单击折叠在 ApplicationOwned 已经接好，但可发现性差：r1352 / att22 强制三角列-only。opencode / crush 整块点、pi `MouseRegion` 整块 result 更易用。roadmap M4 与 `designing/tui-lab/modules/hover-highlight/` 未晋级。

引擎层做不到「Up 才 toggle / Moved 才 hover」——见前置 [c2858 指针命中管线](../archive/2026-10-10-c2858-add-package-tui-pointer-hit-pipeline/proposal.md)。本票是 **产品几何与合约**：扩 hit 矩形、翻 ath33、停丢 Moved、晋级悬停 tint。

ratatui 对照：widget 不收鼠标；app 对 Rect 命中。xylitol 产品应继续 paint 时往 `HitRegionList` `push`（今日 `FoldHitTable` 同拍），不要把 transcript 改成活 Component 树。

## What Changes

- 叶块（Tool / Diff / Ask / Thinking）命中：收起 = 块头全宽；展开 = 空选点 painted 正文也折。ActivityFold 信封/簇 = **仅块头行**。att30 提示带仍只折输出视口。
- **Compaction**：命中保持现行三角列-only（迁 `mark_hit` 时 summary 正文 MUST NOT 进可点体）。视觉分割线 → 延后 [c2892](../../delayed-changes/tui/c2892-update-tui-compaction-session-divider/proposal.md)。**待办栏** 本票不改。
- 产品相位切到 `UpIfEmptySelection`：有选区或发生过拖则不 toggle。body **不** capturing。crush 400ms delay 不作首切片。
- 产品 fan-in **在探针武装时停丢 Moved**（否则悬停到不了引擎）。
- 悬停：引擎 dim；画 **块头 / 可见左边轨 / 提示带**，不 dim 输出正文。拖选中不显示。id 未变不排帧。
- `FoldTarget` 仍只在产品；paint 时 `replace_hits`，click 走 `take_pointer_click`。摘掉产品 `HitPriorityFn` 接线。`ExpandableOutput` 不改 click 合约。
- 晋级 `designing/tui-lab/modules/hover-highlight/` → `tui/modules/`；intent 从整块淡底改为 dim + 块头/轨。

## 非目标

- 不把 transcript 改回每块 Component / 不移植 pi `MouseRegion` 当主路径。
- 不改 Inline；不经 `XYLITOL_TUI_MOUSE` 兼得原生选区 + 点选。
- 不把 Alt+E / Ctrl+T / Ctrl+O 改成「仅鼠标」。
- 不在本票扩展引擎相位（那是 c2858）。
- 不改 Compaction 视觉（分割线 / 流式单行摘要 → c2892）；不把 Compaction 当叶块空选点正文。
- 不改待办栏翼头命中（固定区，不是对话条目）。

## Capabilities

- `app-tui-transcript`（r1352 / att22：三角列-only → 叶块块头+空选正文；ActivityFold 仅块头）
- `app-tui-host`（ath33 Down-swallow → Up + 空选；ath28 Moved 丢弃的探针武装 carve-out）

## Impact

- harness「点正文不 toggle」改成：叶块空选点正文 toggle；ActivityFold 点非块头不 toggle；划词仍优先。
- 空单击今日 Down 已 `dragging=true`、Up 走 clear；切相位后空单击必须 **toggle 且不 OSC52**。

## Open Questions

1. ~~header 全宽 / 整块 / A′ 块尾带回？~~ **已定混合**（2026-10-10）：
   - **叶块**（Tool / Diff / Ask / Thinking）：收起只点 header 全宽展开；展开后空选点 painted 正文也折（opencode 守卫 = `UpIfEmptySelection`，body **不** capturing）。Compaction **不是**叶块。
   - **ActivityFold 信封/簇**：**仅 header 行**（含三角，全宽），不登记展开后的体。
   - att30 输出视口 hint 带后登记，点 hint 只折视口不折 L1。
2. ~~悬停 token？~~ **已定 A**（2026-10-10）：复用引擎 dim，不新开 hover-bg。
3. ~~悬停画哪一段？~~ **已定**（2026-10-10）：点击区可以大于悬停区。悬停画 **块头 / 可见左边轨 / 输出视口提示带**（折叠标记仍在块头行首），不 dim 输出正文。这些是对话条目上的块头与轨，**不是**固定区（状态条、页脚、待办栏、输入坞）。
4. ~~Compaction / 待办栏？~~ **已定**（2026-10-10）：Compaction 保持现行三角列-only（迁表时不得把 summary 登记成可点体）；视觉改分割线交给延后 c2892。待办栏本票不改。
5. ~~产品路径只走 `mark_hit`？~~ **已定**（2026-10-10）：产品 host 摘掉 `install_fold_triangle_hit_priority`；paint 时 `replace_hits`，click 走 `take_pointer_click`。包测若仍钉 `HitPriorityFn` 可留，产品路径不走。

## 折叠回收（深挖，相对 A）

问题不是「header 不够宽」，是 **L1 块头滚出视口后没有回收点**。xylitol 其实已经把「内容很多」拆到另一层，不要用整块 click 去补：

| 层 | 现行 | 长内容怎么折回来 |
|---|---|---|
| 输出视口（Ctrl+O / att30） | 收起：窗口下 hint；展开：块尾 `ctrl+o to fold` | 读完就在块尾，点 hint 折回 **已交付** |
| Activity 栈 | `collapseNearest` 从输入侧盖上 | 不必滚回块头 **已交付** |
| 工具族默认 | Alt+E 翻默认 + 清覆盖 | 粗粒度逃生口，不是定点 |
| L1 块头（三角 / 本票） | 只有头上一列 | 展开后滚走 → **本票缺口** |

同类产品：

- opencode / crush / pi：整块 `onMouseUp`，空选才 toggle。回收容易，但和 xylitol「复制友好优先于视觉热闹」、正文拖选优先对冲。
- Codex TUI：基本不靠 mouse-fold。
- IDE 钉住块头：终端扁平 scrollback 要每帧钉一行，像固定区而不是对话条目；也接近以后滚动槽的 Viewport-sticky，不宜塞进本票。

叶块走 opencode 空选点正文；ActivityFold **不要**抄整块——信封展开后的「体」是簇/工具/夹心 Markdown，不是匀质可点区。z-order：内层后登记赢；body 不 capturing，否则划词死。Bash 仍走 att30，不另开 L1 三角。

## Further Notes

- 产品对照：[research/click-fold-peers.md](./research/click-fold-peers.md)
- 引擎前置：[`c2858` 指针命中管线](../archive/2026-10-10-c2858-add-package-tui-pointer-hit-pipeline/research/pointer-pipeline.md)（已归档）
- Compaction 分割线（延后）：[`c2892`](../../delayed-changes/tui/c2892-update-tui-compaction-session-divider/proposal.md)
