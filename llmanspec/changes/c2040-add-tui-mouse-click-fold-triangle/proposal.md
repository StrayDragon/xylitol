---
depends_on:
  - c2020-add-package-tui-mouse-input
  - c2070-add-package-tui-dual-interaction-modes
blocks:
  - c2050-update-activity-fold-mouse-leader
---

# 鼠标点击折叠三角 + 折叠标记字形

> **状态**：active 规划草案（自 c2070 nested cascade 拆出）。**硬前置** [`c2070`](../archive/2026-08-12-c2070-add-package-tui-dual-interaction-modes/proposal.md) Mode B。架构调研见 c2070 `research/`；本目录保留字形/hit 切片 [`research/fold-glyph-and-hittest.md`](./research/fold-glyph-and-hittest.md)。
>
> **c2020 提示**：点击折叠仍依赖已归档 **c2020** 的 mouse 管道。该管道**保留**；`XYLITOL_TUI_MOUSE` 仅为 lab/e2e。产品 inline **不开** capture——须在 **Mode B（c2070）** 落地后才有正式点折叠 UX。

> **一句话**：点折叠**三角列**（`▸`/`▾`）toggle **单块**（Tool/Diff/Ask/**Thinking per-id**）；`Alt+E`/`Ctrl+T` 改对应族 default 并清 overrides；自带覆盖表（弃 c2030 leader）。

## Why

今日 `Alt+E` 全局翻转所有 tool/diff（并连带 compaction）。定点折叠的最低学习成本是 **点 ▶/▼**（对齐 Web 同源，见 `docs/roadmaps/Web与TUI同源.md`）。

**产品分工（2026-08-11 拍板，弃 c2030 leader）**：

| 路径 | 作用 |
|---|---|
| **键盘** `Alt+E`（及既有 Ctrl+T / Ctrl+O） | **全局** 展开/折叠 |
| **鼠标** 点头行标记 | **单块** per-block 覆盖 toggle |

弃键盘 leader+数字的理由：β′ 屏外块一改行高，贴底 content-end 重绘会「跳回底部」；终端历史上滚无法廉价保留。鼠标点的是 **live 视口内可见头**——用户已在贴底画面上，重绘不额外制造「从历史上滚跳回」的断裂感（行高变化仍会重排上方内容，但交互锚点仍是眼前的标记）。

引擎侧：`c2020` 已提供 opt-in Mouse；产品尚无 hit-test / 覆盖表。

字形：今日 `GlyphSet::fold/unfold` = `▶`/`▼`（Ascii `>`/`v`）。可在不破坏宽度/旁注前提下微调。

## What Changes

1. **手势（已钉）**：`Left Down` 命中三角列 → 立即单块 toggle，吞按不启拖选；**拖选中忽略 fold**（latch）。复用 AO 已开 mouse + `set_transcript_hit_priority`。
2. **范围（已钉）**：Tool / Diff / Ask / **Thinking**；Thinking MUST **per-id**（今日仅全局 `thinking_expanded` → 升 default+overrides）。
3. **覆盖表**：tools 族与 thinking 族各自 `default + overrides`；`Alt+E` / `Ctrl+T` **改对应 default 并清空该族 overrides**；本波不拆 Alt+E×compaction。
4. **Hit-test**：仅三角列 1 cell；render 维护 `fold_hit_regions`（绑 paint gen）。
5. **Paint**：单块 toggle → ath25 局部 miss；禁止全历史 MD 重解析。
6. **字形**：Unicode `▸`/`▾`；Ascii `>`/`v`；`visible_width==1`。
7. **非目标**：整行可点；leader/数字；L2/L3 / Bash / Compaction / Ctrl+O → [`c2045`](../c2045-add-tui-fold-target-remaining/)（可吸收 c2050）。

## Capabilities

- `app-tui-transcript` — 覆盖表 + Thinking per-id + 三角 hit + 字形
- `app-tui-host` — hit_priority 接线 + latch

## Impact

| 层 | 影响 |
|---|---|
| 渲染 | hit 表与 paint-cache 同代；toggle 局部 miss |
| 输入 | Mouse Down/Up 去抖；忽略 move；Shift+click 跟 `c2020` |
| 键位 | **不改** `Alt+E` 全局语义（除非另钉拆 compaction） |
| 性能 | hit O(可见折叠头) |

## 依赖与排序

```text
c2020 + c2070 ──depends→ [本 change c2040] ──blocks→ c2050
```

- **硬依赖**：`c2020`（Mouse 管道）+ **`c2070`（ApplicationOwned；点折叠正式 UX 挂 AO）**。
- **`blocks`**：`c2050`（段级点击语义）。

### 库侧已就绪（c2070，本 change 接产品 hit 表）

| 需求 | API |
|---|---|
| AO 选区优先于 fold？ | 相反：`TUI::set_transcript_hit_priority` — Left Down 回调 `true` 则吞按下并清 transcript 选区（不启拖选） |
| Host 清单 | `packages/xylitol-tui/AGENTS.md` § ApplicationOwned（含 fold hit 行） |
| 单测锚 | `application_owned_transcript_hit_priority_swallows_press` |

本 change **MUST** 在产品 render 维护 `fold_hit_regions`，经上述 hook 接入；**不必**再扩引擎选区状态机。

## Out of scope

- Fold-leader / 数字定点（**已废弃**原 `c2030`）
- Activity L2/L3 摘要文案（`c1760`）
- 改差分引擎算法；产品自管 transcript scroll

## Open Questions

> 2026-08-12：先深挖 pi/zellij 再钉；**未全钉前禁止 Specs landing / apply**。综合稿：[`research/synth-pi-zellij-xylitol-fold-hit.md`](./research/synth-pi-zellij-xylitol-fold-hit.md)。

### 已拍（人）

| # | 钉 |
|---|---|
| 字形 | 本期用 **`▸`/`▾`**（Ascii 仍 `>`/`v`）；须验 `visible_width==1` |
| 范围 | **本期三角 L1 点折**：Tool + Diff + Ask + **Thinking（须新增 per-id 态）**；其余可折叠块 → **另开 draft change**（最终目标：凡可折块均可鼠标独立点开） |
| mouse Enable | **复用 AO 会话已开 capture**；不经 `XYLITOL_TUI_MOUSE` 当产品开关（ath30） |
| 对照源 | pi TUI **无**点折控件；zellij 贡献 **优先级/latch/分层**，不贡献 fold 产品语义 |

### 待深挖（一次一问）

| # | 状态 | 钉 |
|---|---|---|
| 1 Toggle 手势 | **已钉** | **A + latch**：`Left Down` 命中三角 → 立即 toggle，吞按不启拖选（现 `set_transcript_hit_priority`）；**拖选进行中忽略 fold 重命中**（zellij latch） |
| 2 命中几何 | **已钉** | **仅三角列**（glyph 1 cell；点正文/旁注不 toggle） |
| 3 本期 entry 类型 | **已钉** | **Tool + Diff + Ask + Thinking**；Thinking **MUST** 从全局-only 升为 **per-id**（与 tool 覆盖表同构：default + overrides）；点三角 = 单块 toggle。`Ctrl+T` 仍为 thinking **全局** default（清/改 overrides 规则见 #4 同类） |
| 4 全局 Alt+E / Ctrl+T | **已钉** | **A**：改对应族 default **并清空该族 overrides**（`Alt+E`→tools 族；`Ctrl+T`→thinking 族）；**本波不拆** Alt+E×compaction 连带 |
| 5 （并入 #1） | — | latch 已随 #1 钉死 |
| 6 延后 draft | **已钉 B** | [`c2045-add-tui-fold-target-remaining`](../c2045-add-tui-fold-target-remaining/proposal.md)：广义 `FoldTarget` 总装 + 剩余可折块鼠标独立点 + **可收薄/吸收 c2050** |

## 验证（自动化 + 人类）

| 层 | 自动化 | 人类 |
|---|---|---|
| Harness | 合成 `Mouse Down` 在标记列 → 单块 toggle；点正文 → 态不变；全局 Alt+E 清覆盖 | Kitty/foot：点标记收起/展开；误点 Markdown 不折 |
| Paint | 单块 toggle miss 上界（ath25） | 点眼前块时无明显「从历史上滚跳回」断裂（已在 live 底） |
| 字形 | `visible_width==1`；Ascii 回退 | `XYLITOL_TUI_GLYPH_SET=ascii` |

**人类最短路径**：开 mouse → 点可见折叠标记收起 → 再点展开 → `Alt+E` 全局翻转并清覆盖。

## Ethics

- risk_level: low–medium
- prohibited_actions: 点正文大面积误 toggle；无 `c2020` 透传策略就默认常开 capture；复活默认 Alt+digit leader
- required_evidence: harness Mouse→单块 toggle；错点正文不变；字形宽度；覆盖与全局 Alt+E 清表
- escalation_policy: 默认 EnableMouse 改变选区习惯须确认

## Further Notes

- 字形/hit 切片：[`research/fold-glyph-and-hittest.md`](./research/fold-glyph-and-hittest.md)
- **pi 一手**：[`research/pi-fold-hit-and-mouse.md`](./research/pi-fold-hit-and-mouse.md) — TUI 无点折；全局键盘 + Alt-screen 选区/OSC8
- **zellij 一手**：[`research/zellij-mouse-hit.md`](./research/zellij-mouse-hit.md) — gather/determine/execute、控件≻选区、latch
- **综合推荐**：[`research/synth-pi-zellij-xylitol-fold-hit.md`](./research/synth-pi-zellij-xylitol-fold-hit.md)
- **架构 / 选区 oneof / starline 对等**：见 [`c2070 research/`](../archive/2026-08-12-c2070-add-package-tui-dual-interaction-modes/research/)
- 差分适切性见 archive `c2020` `diff-engine-mouse-fit.md`
- `c1760` 曾拍 `▶/▼`；**本期人拍改为 `▸`/`▾`**（与 c1760 合流时再对齐文档）
- **2026-08-11**：废弃 `c2030` fold-leader；定点改由本 change 鼠标路径独占
- **2026-08-12**：c2070 暴露 `set_transcript_hit_priority`；库 hook 已就绪，产品表与手势语义仍须深挖钉死后再 apply
