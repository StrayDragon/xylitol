# Research: xylitol Mode B 子系统切分建议

> 日期：2026-08-11
> Change：`c2070-add-package-tui-dual-interaction-modes`
> 合并：既有 `pi-dual-tui-modes-and-xylitol-cost.md` + 本轮 Pi AltScreen / Zellij 专篇（见同目录）。
> 性质：Change 调研文档，**不是** live specs。

## 0. 结论（给 design / apply）

| 决策 | 建议 |
|---|---|
| 要不要第二套渲染器 | **要**：Mode B ≈ Pi `TuiAltScreen` 族；保留今日 Mode A inline 差分，不要在 A 上硬塞应用选区 |
| mouse 管道 | **复用 c2020**；Mode B 启动路径显式 Enable；禁止 env 冒充产品开关 |
| 子系统 | `mode` · `viewport` · `selection` · `clipboard` · `input-exclude` · `hit-priority`（钩子） |
| 输入框 | 底部 Editor **排除** transcript 拖选；click 落输入区抢焦点 / 取消选区 |
| 复制 | 松手默认复制；OSC52 与本地工具都允许，优先级可配置 |
| 与 cascade | 折叠 hit 只留优先级钩子；实现归 `c2040`/`c2050` |

## 1. 对照摘要

| 能力 | Pi AltScreen | Zellij | xylitol Mode B |
|---|---|---|---|
| 双模式入口 | `TuiMainScreen` / `TuiAltScreen` 择一 | 本身即应用管 pane（非 coding-agent 双 TUI） | 库双模式 + 产品默认 A |
| 拖选 | `selectionAnchor`/`Focus` + 字/词/行 | `Selection{start,end,active}` + Start/Update/End | 同构状态机 |
| 越界续选 | 贴边 **50ms** timer + `ScrollView.scrollBy` | 越界 **10ms**/行；拖选中只移锚点 | MUST（间隔可取中/可加速） |
| 松手复制 | **仅 OSC52** + flash | `copy_on_select`；`copy_command` 否则 OSC52 | MUST 默认开；允许 OSC52 与/或本地 |
| 输入/不可选 | dock 在 ScrollView **外**；无 click-to-focus | tracking / frame / `set_selectable(false)` | 结构 dock 排除；MAY click-to-focus |
| 点折叠 | AltScreen **未实现** fold hit | n/a（multiplexer） | **后续** change（预留 hit 优先钩子） |

细节与行级引用见：

- [`pi-altscreen-selection-scroll-copy-input.md`](./pi-altscreen-selection-scroll-copy-input.md)
- [`zellij-selection-scroll-copy-input.md`](./zellij-selection-scroll-copy-input.md)
- [`pi-dual-tui-modes-and-xylitol-cost.md`](./pi-dual-tui-modes-and-xylitol-cost.md)

## 2. 建议状态机（产品可观察）

```text
idle
  press(left, in_transcript, not_hit_consumed)
    → selecting(anchor=focus=cell)
  drag / move
    → update focus; if at viewport edge → auto_scroll + extend
  release
    → if non-empty selection → copy (default on) → idle(selected?)
  press(in_editor_or_chrome)
    → clear or ignore selection; focus editor
  press(on_fold_hit)          # hook only in c2070
    → consume click; no selection start
```

## 3. 与今日 xylitol-tui 的缝

- 已有：inline 差分引擎、`Editor`、`c2020` mouse、host `ath29` 扇入纪律。
- 没有：alt-buffer 渲染器、应用 ScrollView 选区、OSC52/松手复制绑在选区上、输入矩形 hit 分流。
- 风险：把 selection 塞进 Mode A 差分路径 → 与终端原生选区 oneof 冲突（见 `emulator-vs-app-selection-oneof.md`）。

## 4. 非目标（再钉一次）

整包换 ratatui · 产品默认切 B · 实现折叠点击 · viewport slice 性能大修
