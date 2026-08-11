# Research: 原生选区/滚动 × 点击折叠（c2070）

> 日期：2026-08-11
> Change：`c2070`（自 c2040 调研迁入）
> 动机：产品期望「未修饰拖选 + 滚轮/历史上滚与今日一致，同时可点折叠标记」；纠正「库不保证」的含糊表述。

## 1. 纠正表述

| 说法 | 是否准确 |
|---|---|
| 「crossterm 库拒绝保证开 capture 仍保选区」 | **易误导**。一手事实是：库**未文档化**选区行为；`EnableMouseCapture` 固定开 `1000/1002/1003`。 |
| 「开标准 mouse reporting 后，多数仿真器把**未修饰**拖选从终端切走」 | **准确**（协议层 + 仿真器惯例）。 |
| foot/xterm/Kitty：未修饰选区旁路 | 默认用 **Shift**（或配置的 override modifiers）临时把事件还给终端选区。 |

一手/一手级来源：

- crossterm 0.29：`EnableMouseCapture` → `?1000h` + `?1002h` + `?1003h`（仓内 `c2020` research）。
- foot(1)：client 开 mouse tracking 后，左键拖选「normally disabled」，**hold shift** 强制选区；`selection-override-modifiers` 默认 `Shift`。
- xterm / Ghostty：同类 Shift（或 XTSHIFTESCAPE）旁路惯例。
- 行业综述（二手，机制与一手一致）：[Why TUI apps can't have both scroll and text selection](https://yogirk.dev/posts/why-terminal-tui-apps-cant-have-both-scroll-and-text-selection/) — **同一套 xterm mouse mode 无法同时**「应用收 click/wheel」与「终端原生未修饰拖选」。

## 2. 期望 UX vs 协议能力

**期望（自然）**：

1. 未修饰拖选 / 复制 = 今日（capture 关）
2. 滚轮 / 终端历史上滚 = 今日
3. 单击折叠标记 → toggle 单块

**协议事实**：鼠标要么由**终端**处理（选区+滚），要么由**应用**经 reporting 接收（可点折叠）。**没有**「只把折叠列的点击上报、其余仍归终端选区」的标准模式。
同一左键 Down：既是「开始拖选」也是「点三角」——终端无法按屏幕区域拆给两边。

因此：

- **仅** `EnableMouseCapture` + 点折叠 → **必然**改变未修饰选区/滚轮归属（与期望 1–2 冲突）。
- 这不是 xylitol 实现疏忽，是 VT 鼠标模式的分界。

## 3. 能逼近「自然」的路径与成本

| 方案 | 未修饰选区 | 滚动 | 点折叠 | 成本（量级） | 备注 |
|---|---|---|---|---|---|
| **A. 不调 capture** | ✓ 今日 | ✓ 今日 | ✗ | 0 | 无点击 |
| **B. 开 capture，Shift+选**（今日 foot 手测） | ✗ 要 Shift | ✗ 轮进 app 且产品未处理 | ✓ | **低**（c2040 原 MVP） | 违反「像之前一样」 |
| **C. 开 capture + 应用内选区**（crush 路线） | ✓ 应用自绘选区 | 须应用处理 wheel/或放弃终端历史上滚 | ✓ | **高**（约数日～1–2 周+） | 差分引擎上叠选区高亮、拖、OSC52/primary；与 ath25/inline 重绘交互要设计 |
| **D. 开 capture + 瞬时关捕获骗选区** | 不可靠 | 不可靠 | 半 | 中且脆 | Down 已消费则无法「还给」终端 |
| **E. OSC8 超链接伪装折叠** | 多半 ✓ | ✓ | 半 | 中～高 | 点击常走出进程（xdg-open）；应用内 toggle 需自定义 scheme/桥，端差异大 |
| **F. 开关式 fold-click 模式** | 关模式时 ✓ | 关时 ✓ | 仅开模式时 ✓ | **低** | 明确 tradeoff，非「始终自然」 |

**xylitol 额外约束**：产品是 **inline 差分 TUI**（历史画进模拟器 scrollback），不是纯 alt-screen 自管视口。用户「像之前一样滚动」很大程度依赖 **capture 关** 时终端滚历史。开 capture 后轮事件进 app——要恢复同等「上滚看旧帧」，几乎要 **应用自管 scroll**（更高成本，先前已排除）或关 capture。

## 4. 结论（给拍板）

1. 「未修饰选区 + 终端式滚动 + 点折叠」**不能**靠「打开 crossterm mouse 就自然具备」实现。
2. 若坚持三者同时、且选区手感贴近系统选区：主路径是 **C（应用内选区）**，点击折叠作同一 capture 下的命中分支；滚动策略须另钉（应用滚 vs 接受历史上滚变弱）。
3. 若 c2040 要保持 **低成本 / 高 ROI**：只能选 **B** 或 **F**，并在产品文案写清 tradeoff——**不要**再写「开了 mouse 还保证与今日完全相同的未修饰选区」。

## 5. 建议产品钉（待用户选）

- **C′（分 phase）**：c2040a = 覆盖表 + hit + capture + 点标记（接受 Shift 选 / 滚轮另说）；c2040b 或独立 change = 应用内选区，达成「自然拖选」。
- **或** c2040 直接上 **C**（大切片，tasks 含选区）。
- **或** **F** 为默认推荐（保今日选区/滚，需要点折叠时再开模式）。
