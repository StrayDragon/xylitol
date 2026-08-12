# 综合：pi × zellij × xylitol → c2040 折叠命中

> 一手调研：[`pi-fold-hit-and-mouse.md`](./pi-fold-hit-and-mouse.md)、[`zellij-mouse-hit.md`](./zellij-mouse-hit.md)、既有 [`fold-glyph-and-hittest.md`](./fold-glyph-and-hittest.md) + c2070 `pi-altscreen-*`。
> **禁止实施**：本稿只服务 explore / propose 决策。

## 一句话

| 来源 | 与「点三角折叠」的关系 |
|---|---|
| **pi TUI** | **没有** transcript 折叠三角点击；只有全局键盘 expand + Alt-screen 选区/滚轮/OSC8 |
| **zellij** | **没有** fold 控件；有 **gather→determine→execute**、geom 命中、**控件优先于选区**、**手势 latch** |
| **xylitol 已有** | AO + mouse；`set_transcript_hit_priority`（Down 吞按 → 不启拖选）专为 c2040 预留 |

→ 产品缝是 **xylitol 自研**；可借 **优先级 / latch / 双坐标**，不可抄 pi 控件（没有）或 zellij pane mux。

## 推荐架构（Rust / 本仓）

```
Left mouse (screen col,row)
  │
  ├─ if selection_dragging → 只更新/结束选区（zellij latch；忽略 fold 重命中）
  ├─ else if dock → Editor / dock 路径（已有）
  ├─ else if fold_hit_regions.contains(col,row)  [产品表，绑 paint gen]
  │     → set_transcript_hit_priority 返回 true
  │     → toggle 该块覆盖态；清 transcript 选区；request_render
  └─ else → transcript 选区（现有 SelectionController）
```

| 层 | 职责 | 不放 |
|---|---|---|
| `xylitol-tui` | 已有 hit_priority hook + 选区 | 业务 FoldTarget / 覆盖表 |
| `src/app/tui` | render 建 `fold_hit_regions`；toggle overrides；字形 | 第二套 mouse 协议 |

## 从对照源吸收 / 拒绝

| 模式 | 源 | 对本仓 |
|---|---|---|
| 全局键盘仍批量翻折 | pi `ctrl+o`/`ctrl+t` | **保留** `Alt+E` / `Ctrl+T` 全局语义 |
| 选区 vs 激活：release 无拖才当 click | pi OSC8 | **候选**；与现 Down-swallow API 二选一（见 Open Q） |
| 引擎吞鼠标、不进 editor VT | pi Alt-screen | **已对齐**（AO dispatch） |
| gather → determine → execute | zellij | **决策表化**（不必搬类型名） |
| 控件 hit ≻ 选区起笔 | zellij frame intercept | **已对齐** `hit_priority` |
| 拖选 latch | zellij | **SHOULD**：拖中不重算 fold |
| 整块 HTML onclick | pi export | **拒绝**（终端选区不同） |
| 组件树 mouse API | — | **拒绝**（pi Component 也无；本仓 Mouse 在 host/引擎） |
| 通用任意块 FoldWidget | — | **延后提案**（本期只三角 L1） |

## 本期 vs 延后（人拍意向，待深挖钉死）

| 本期 c2040 | 延后 [`c2045`](../c2045-add-tui-fold-target-remaining/proposal.md)（Q6=B） |
|---|---|
| 三角列：Tool / Diff / Ask / Thinking(per-id) | 广义 `FoldTarget` 总装 + 剩余块 + **可收薄/吸收 c2050** |
| 字形 `▸`/`▾`；A+latch；仅三角列；全局清族 overrides | Bash/Compaction/Ctrl+O/段级等 |

## 仍须深挖钉死的决策（非事实）

1. ~~Toggle 手势~~ → **已钉 A+latch**（Down 吞按；拖选中忽略 fold）
2. ~~命中几何~~ → **已钉仅三角列**
3. ~~本期 entry~~ → **Tool+Diff+Ask+Thinking(per-id)**
4. ~~全局键~~ → **A**：改 default + 清该族 overrides；不拆 compaction
5. ~~拖选 latch~~ → 并入 #1
6. ~~延后 draft~~ → **c2045**（FoldTarget 总装 + 剩余块；可吸收 c2050）
