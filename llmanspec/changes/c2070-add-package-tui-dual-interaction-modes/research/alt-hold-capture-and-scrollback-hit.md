# Research: Alt 按住开 capture ×「滚到任意折叠区再点」（c2070）

> 日期：2026-08-11
> Change：`c2070`（自 c2040 调研迁入）
> 动机：探索「按住 Alt + 点击 toggle，松 Alt 恢复原生选区/滚」；对照「最好无修饰键直接点」与「滚到任意可折叠区再 peek」；诚实对齐 inline/原生终端模型。
> 相关：[`native-selection-vs-click-fold.md`](./native-selection-vs-click-fold.md)、归档 `c2020` `diff-engine-mouse-fit.md`。

## 0. 结论（可引用）

| 问题 | 答案 |
|---|---|
| Alt+click 在协议上能否点折叠？ | **能**——前提是点击发生时 **mouse capture 已开**，且事件带 `ALT`（SGR 鼠标上报含修饰位）。 |
| 「松 Alt 就恢复原生选区/滚」能否逼近？ | **能逼近**：默认 **关** capture；**仅 Alt 按住期间** Enable；松开 Disable。松手后未修饰拖选/终端历史上滚回到今日。 |
| 能否靠「点时才看有没有 Alt」且平时关 capture？ | **不能**。capture 关时左键归终端，应用**收不到**这次 click，谈不上读修饰键。必须先开 capture，再点。 |
| 「按住 Alt」怎么知道？ | 需要 **Alt 单独 Press/Release**。今日 Kitty 协商 flags=`7`（`DISAMBIGUATE`∪`EVENT_TYPES`∪`ALTERNATE_KEYS`），**不含** `REPORT_ALL_KEYS`（bit 8）→ **默认不报修饰键 alone**。脚要加 flag 8（或等价），且终端支持 Kitty keyboard（foot/Kitty/Wez/Alacritty 等）。 |
| 「最好无修饰键直接点」？ | 与「始终原生未修饰选区」**协议互斥**（见前一篇）。要无修饰点折叠 → 常开 capture，或应用自管选区，或短暂「点击模式」。 |
| 「滚到**任意**可折叠区再 toggle」？ | **鼠标做不到**（在现有 inline 模型下）。已滚进**模拟器 scrollback** 的历史行不在差分 live 视口；应用收不到有意义 hit。只能点 **当前终端可见的、仍由引擎 paint 的活帧**。 |

→ **Alt-hold capture** 是相对诚实的低成本折中：平时像今日；需要定点 peek 时按住 Alt 点 **眼前** 折叠头。它**不**兑现「无修饰」与「点任意历史上滚区」。

## 1. 用户需求拆解

| # | 表述 | 产品意图 |
|---|---|---|
| R1 | 滚到可折叠区 → toggle peek | 定点看内容，为之后合并折叠/展开做自然交互 |
| R2 | 最好无修饰，直接鼠标 | 学习成本最低，对齐 Web「点三角」 |
| R3 | 或：Alt+click；松 Alt 恢复原状 | 在原生选区/滚不被长期抢走的前提下加点折叠 |
| R4 | 保持终端原生（非 Ratatui 全掌控） | 不换 alt-buffer / 不全盘自管选区与 scroll（除非另开大切片） |

R2∩「始终原生选区」∩标准 VT mouse = **不可能**（前篇方案表）。
R1∩「终端历史上滚再点」∩ inline = **不可能用鼠标**（见 §3）。
R3 与 R4 **可对齐**（§2），并部分服务 R1（仅 live 视口）。

## 2. 方案 G：Alt-hold 临时 capture

### 2.1 交互

```text
默认：mouse capture OFF  → 未修饰拖选 / 终端滚历史 = 今日
用户按下 Alt
  → 应用收到 Alt Press（需 §2.3）
  → EnableMouseCapture
用户 Alt+左键点折叠标记（live 视口）
  → Mouse Down 带 ALT → hit-test → per-block toggle
用户松开 Alt
  → Alt Release → DisableMouseCapture
  → 选区/滚恢复
```

引擎侧已有成对 API：`CrosstermTerminal::enable_mouse_capture` / `disable` + `sync_mouse_capture`（可运行时切，不必重启 TUI）。

### 2.2 与「始终开 capture」对比

| | 始终开（方案 B） | Alt-hold（G） | 模式键（F） |
|---|---|---|---|
| 未修饰选区 | 要 Shift | **平时 ✓**；仅 Alt 按住期间 ✗ | 关模式时 ✓ |
| 终端历史上滚 | 轮进 app | **平时 ✓**；Alt 按住时轮进 app | 关模式时 ✓ |
| 点折叠 | ✓ | 仅 Alt+点 ✓ | 仅开模式时 ✓ |
| 学习成本 | 「选区要 Shift」 | 「点折叠要按住 Alt」 | 「先开模式再点」 |
| 实现门槛 | 低 | 中（键 alone + 动态 sync） | 低 |

### 2.3 硬依赖：Alt alone + Release

Kitty keyboard protocol（一手）：只有打开 **Report all keys as escape codes**（增强位 `0b1000`）才会报告**修饰键本身**的 press/release。
仓内今日：`KITTY_FLAGS_REQUEST = 7`（= bits 0+1+2），**没有 bit 8**。

因此落地 G 须至少：

1. 协商 flags 含 `REPORT_ALL_KEYS_AS_ESCAPE_CODES`（并保留 release：已有 `REPORT_EVENT_TYPES`）。
2. host 在 `KeyEvent { code: LeftAlt|RightAlt, kind: Press }` → enable；`Release` → disable。
3. 过滤：Alt 按住期间的 `Moved` 仍不 dirty（c2020 闸）。
4. 终端不支持 Kitty 时：**降级**——无 G；只能 F（显式模式键开 capture）或始终 B，或无鼠标点折叠。

脚注：即使用户「以为」按住了 Alt，若从未收到 Alt Press（无 flag 8），capture 仍关 → click 变选区起点，**静默失败**。产品须可见反馈（如 Alt 按住时 chrome 提示「可点折叠」）或接受「仅现代终端」。

### 2.4 竞态与边角

| 边角 | 处理意向 |
|---|---|
| Alt Down → Enable CSI 未生效前就点 | 极短窗内 click 仍归终端；可接受或要求「先按住再点」 |
| Alt 按住拖选 | 事件进 app；**不要**当选区；忽略 Drag 即可 |
| Alt 按住滚轮 | 进 app；本波可忽略（不改 scroll） |
| 焦点丢 / suspend | `stop` 已 `release_mouse_capture_active`；恢复后 desire 须与 Alt 态一致（建议清「Alt-held」假想态） |
| macOS Option=Alt | 终端把 Option 当 Meta 才可靠；文档说明 |

### 2.5 为何不是「OSC8 超链接当折叠」

超链接在 **未** tracking 时可点，但应用常收不到进程内 toggle（终端去 open URL）；开 tracking 后又与超链接抢 click。不适合作为主路径（前篇 E）。

## 3. 「滚到任意折叠区」——inline 的硬边界

xylitol-tui = **inline 差分**：长内容滚进**模拟器 scrollback**；应用只持有当前 paint 的行缓冲与 `previous_viewport_top`。

| 用户动作 | 鼠标能否 toggle 该折叠头 |
|---|---|
| 贴底 / 活视口内仍看得见的头 | **能**（capture 开且 hit 命中） |
| 终端历史上滚，只看见旧帧 | **不能**——坐标落在模拟器缓冲，不进应用 mouse reporting 的「当前屏语义」与 hit 表 |
| 希望「上滚 peek 很久以前的 tool」 | 须 **应用自管 scroll**（把旧 entry 留在 live 视口）或键盘全局/其它非鼠标路径 |

这与 Ratatui/alt-screen「整屏都是应用画布」不同：那边历史上滚也是应用状态，点哪里都可以命中。**本产品选原生 scrollback，就买了「历史不可点」这张票。**

对 R1 的诚实产品翻译：

- **可交付**：在**当前可见活帧**上 Alt+点（或模式内点）单块 peek；配合贴底习惯，最近工具块可点。
- **不可廉价交付**：终端上滚任意远 → 用鼠标 toggle 那一块。
- **若 R1 是硬需求且必须鼠标**：单独立案「应用视口/scroll」（高成本，先前排除）；或接受键盘全局 + 仅 live 点击。

「之后合并折叠/展开会很自然」——**交互隐喻**（点三角）仍成立，但**空间范围**锁在 live 视口；合并语义在数据层，不依赖「点到 scrollback 里的旧三角」。

## 4. 与「无修饰直接点」的关系

| 目标组合 | 可行路径 |
|---|---|
| 无修饰点 + 原生选区始终 | **否**（协议） |
| 无修饰点 + 牺牲原生选区（Shift 选） | 方案 B，低成本 |
| 无修饰点 + 应用自管选区 | 方案 C，高成本 |
| 有修饰（Alt）点 + 平时原生选区 | **方案 G**（本篇），中成本 |
| 显式模式后再无修饰点 | 方案 F，低成本 |

在坚持 R4（终端原生）且不接受长期 Shift-选的前提下：**G 或 F 是主候选**；「无修饰始终」只能放弃原生选区或上 C。

## 5. 建议（待拍板，不自动开 tasks）

**推荐默认钉（对齐 R3+R4，部分 R1）**：

1. **c2040 主路径 = G（Alt-hold capture）+ 标记列 hit + per-block 覆盖表**。
2. 文案写清：**仅 live 视口**；历史上滚请用键盘全局或滚回贴底再 Alt+点。
3. Kitty flags 增 bit 8；不支持时降级到 F（例如既有/新「折叠点击模式」键）或仅键盘。
4. **不**把「无修饰始终点折叠」写进本波 MUST。
5. 「应用自管 scroll 以点历史」**明确 out of scope**（可进 roadmap，不进 c2040）。

备选：若更讨厌按住 Alt，改 **F（短模式）**——按一次开 capture + chrome「点击折叠中」，再按/Esc 关；模式内可无修饰点。

## 6. 验证意向（若钉 G）

| 层 | 内容 |
|---|---|
| 单元/harness | Alt Press → capture active；Release → inactive；Alt+Down on marker → toggle；无 Alt 时 Down 不到产品（capture off） |
| 人类 foot/Kitty | 松 Alt 后未修饰拖选恢复；Alt 按住点 ▸/▾；上滚历史后点旧行 **不应**误 toggle（或无反应） |
| 回归 | 不开 mouse 时行为与今日一致；Moved 不刷帧 |
