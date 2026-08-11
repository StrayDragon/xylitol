---
depends_on:
  - c1755-update-tui-travel-notice-placement
blocks:
  - c2050-update-activity-fold-mouse-leader
---

# TUI activity-fold — 多级折叠（含 Worked for 通用表达）
> **一句话**：长会话中间操作墙多级折叠（最狠为 Worked for 2m 3s 通用耗时表达），可配置降级
> **当前排序**：#3（2026-08-10 自 delayed-changes 升格入 active）


> **已升格（2026-08-10）**：自 delayed-changes 移入 active 待处理队列，当前排序 **#3**。


> 与 `domain-compaction` 不是同一层。前置：[`c1755`](../archive/2026-07-30-c1755-update-tui-travel-notice-placement/proposal.md)（已扩大为「禁顶插 / 贴底可滚通知」政策）。两案调研后可分别 propose。

## Why

长会话中间操作墙过高；resume/重建冷 paint 重。需要对齐 Cursor 式 **多级折叠**：从「单块收起」到「活动计数摘要」再到最狠的 **通用耗时表达**（如 `Worked for 2m 3s`），在可读性与少画之间可配置降级。

纯 UI；主 ROI 在长历史/resume——不宣称治流式尾。

## 多级折叠设计（意向心智）

由细到粗（展开键可沿级循环或按级绑定；正式化再钉）：

```text
L0  全细账
    Tool / Diff / Thinking / Bash 各块可见（今日默认 tools_expanded≈此层附近）

L1  单块折叠（已有）
    Ctrl+T / Alt+E / Ctrl+O — thinking / tool·diff / viewport

L2  活动摘要（本草案主交付意向）
    旧 turn 中间操作 → 一行计数摘要
    例：Explored 6 files, 5 searches, ran 9 commands   +22 -19
    保留：User + 最终 Assistant；System/Error 外显（travel 见 c1755）

L3  通用耗时表达（最粗；可配置 / 更旧历史）
    例：Worked for 2m 3s
    单行横幅；不枚举工具种类；时长 = 该段 wall clock（见调研笔记）
    Alt+Shift+E：先作用于最近一段，再更早段 …
```

| 级 | 用户看到 | 少画强度 | 典型触发 |
|---|---|---|---|
| L0/L1 | 细账或单块折 | 弱–中 | 近 `keep_recent_turns`；手动 |
| L2 | 计数 + 可选 `+/-` | 强 | rebuild 自动；超窗口旧 turn |
| L3 | `Worked for …` | **最强** | 更旧段 / 可配置阈值；或 L2 之上再压 |

**原则**：越旧可越粗；最近 K turn 保持 L0/L1；L3 **不得**假时长；无可靠时钟则停留 L2 或省略时长字段。

参照（产品外）：Cursor compact 既有活动计数条，也有纯 `Worked for Xm Ys` 条——本草案把二者收成 **同一折叠阶梯的两级**，而非互斥功能。

## 已拍板

| 项 | 决定 |
|---|---|
| 自动折叠 | rebuild 自动；直播回合结束 auto：调研通过后默认开 |
| 窗口 | `keep_recent_turns` 默认 2（可配置） |
| 交互 MVP | **M2** + **C1 双向栈** |
| 展开 | `Alt+Shift+E` = `activity.expandNearest`（最近 L2/L3 升一级） |
| 收纳 | `Ctrl+Alt+Shift+E` = `activity.collapseNearest`（最近 L0 Activity 降一级；地板=段默认粗级） |
| 无目标 | 两键均静默 |
| 远段自动 L3 | 可配，**默认关** |
| 旁注 | **先满和弦**（形态 A，对齐今日 `(Alt+E)`）；之后可调短标签 / 混合 |
| 标记 | `▶/▼` / `>/v`；不做 `(+)/(-)`；字形微调见并行 `c2040` |
| 鼠标 | **本波（c1760 MVP）仍可不实现点击**；行距缝预留。并行草案：`c2020` 地基 → `c2040` 点击；段级适配 `c2050` |
| 与 Alt+E | 分层（深挖 A）；L1 **全局**仍 `Alt+E`；L1 **定点**改并行 `c2040` 鼠标点三角（原 `c2030` leader **已废弃**） |
| 跨面 | **动作语义统一**；物理键/点击分面绑定（深挖 D） |
| 旧 turn | User + 最终 Assistant；中间 ≥L2，可 L3 |
| System/Error | 始终外显；travel 见 c1755 |
| 正式化 | 搁置 |

## What Changes（意向；propose 时拆）

1. **M2+C1**：段 level + `expandNearest` / `collapseNearest`；TUI 默认 `Alt+Shift+E` / `Ctrl+Alt+Shift+E`（可改绑）。
2. 摘要行：`▶/▼` + **满和弦旁注**（先对齐 `(Alt+E)`；后续可调）；render 维护 segment→行距（鼠标预留）。
3. 配置：`keep_recent_turns`；远段自动 L3 默认关；摘要开关。
4. 纯 UI；验收含双向栈与 Alt+E 分层。

## Capabilities（意向）

- `app-tui-transcript`、keybindings、`runtime-config`（MAY）
- 非目标：`domain-compaction`；不实现 delayed

## 与 delayed（缓解，非交付）

| delayed | 关系 |
|---|---|
| c1505 / c1370 | L2/L3 少画后紧迫性可能下降；仍独立 delayed |
| c1535 | 无替代关系 |

## 与 c1755 / 未来 System 统一案

- c1755：travel 只改放置，文案完整保留。
- **另案**（未建）：统一 System message 样式/文案族——不在本 change scope。

## 并行交互增强草案（2026-08-11 · 修订弃 c2030）

| id | 主题 | 依赖边 |
|---|---|---|
| [`c2020`](../archive/2026-08-11-c2020-add-package-tui-mouse-input/proposal.md) | 包级 Mouse | Wave 0✓ archive；`blocks` → c2040/c2050 |
| ~~c2030~~ | ~~Leader + 数字定点~~ | **已废弃**（屏外数字 + 贴底重绘体验差） |
| [`c2040`](../c2040-add-tui-mouse-click-fold-triangle/proposal.md) | 点击三角 + L1 覆盖表 + 字形 | Wave 1；`depends_on` **仅** c2020 |
| [`c2050`](../c2050-update-activity-fold-mouse-leader/proposal.md) | 多级适配鼠标/覆盖 | Wave 2；本 change **`blocks`** 之 |

本 change（c1760）与 c2020/c2040 **无** `depends_on` 边——MVP 可先落地；交互增强收口在 c2050。

性能并列（减行/切片，非本波交付）：`c1505` / `c1370` / `c1535`。

## Out of scope

- 实现 c1505/c1370/c1535；改 LLM 上下文
- 本波统一全部 System 文案；假 `Worked for` / 假 `+/-`
- `(+)/(-)` 标记；scrollback 常驻焦点槽
- **MVP 可不实现**鼠标点击（改由 c2020–c2050）；本波仍 **MUST** 预留行距缝（深挖 B）。**不做** keyboard fold-leader / 数字编号。

## 调研笔记（续 · L3 时长与展开）

### L3 时长：为何现阶段选 wall clock（推荐 a）

| 方案 | 优点 | 缺点（相对 xylitol 现状） |
|---|---|---|
| **段/turn wall clock** | 对齐 Cursor「Worked for」体感；会话 `SessionEntry` base 已有 `timestamp` 字符串可算差；含等模型/思考间隔，用户感知的「忙了多久」 | 时钟漂移/缺字段时要降级；并行不敏感（本来也不该用求和） |
| 工具 span 之和 | 更像「干活 CPU」 | UI/`UiEntry` **无**稳定 per-tool 起止；并行会双计或难定义；漏掉 LLM 等待 → 常远小于体感；实现重 |

**现结论**：L3 用 **该折叠段的 wall clock**——意向公式：`t(最终 Assistant 或段末) − t(该段 User 或段首中间操作)`；缺任一端 timestamp → **停在 L2** 或省略时长，**MUST NOT** 伪造。工具 span 之和若将来有观测数据，可作 L2 旁注，不作 L3 主文案。

### `Alt+Shift+E`（已拍：先最近一段）

循环对象是 **段（segment）**：先展开/升细最近一段 ActivityFold，再按更早段。避免一次铺开整屏细账。同段内 L3↔L2↔L1 是否共用该键或要第二和弦 → 仍 Open。

### 与已有时间戳

- `protocol` 消息 / session base 带 timestamp（ISO 或 ms 视条目）；重建路径需确认 travel/resume 投影后 UI 侧能否读到——propose 时核对 `session_entry_to_ui_entries` 是否丢戳。

## 交互复盘与设计讨论（2026-07-30）

> MVP：**M2 + C1**；收纳键 `Ctrl+Alt+Shift+E`（库支持、终端有条件）；跨面统一动作语义。

### 今日已有行为（代码 / DESIGN 事实）

| 层 | 状态机 | 键 | 作用域 | 默认 | 旁注 |
|---|---|---|---|---|---|
| Thinking 显隐 | `thinking_expanded` | Ctrl+T | **全局**所有 thinking | 关（流式中展开） | `(Ctrl+T)` + `▶/▼` |
| Tool/Diff 块显隐 | `tools_expanded` | Alt+E | **全局**所有 tool/diff；**同键**切 compaction | **开** | `(Alt+E)` + marker |
| 块内 viewport | `tools_output_expanded` | Ctrl+O | **全局**满高↔预览行数 | 预览 | `ctrl+o to expand` |
| 会话树节点 | `folded_nodes` 按 id | Ctrl/Alt+Left/Right | **每节点** | 展开 | `⊞/⊟` |
| Activity L2/L3 | （无） | — | — | — | — |

**关键结构债**：产品 scrollback 折叠是 **三个全局 bool**（`ScrollbackFold`），**不是** per-entry / per-segment。树才是按 id 折叠。
因此「先展开最近一段 ActivityFold」**无法**只靠再绑一个全局和弦完成——必须先有 **段级（或 entry 级）状态**，再谈键/鼠标怎么改它。

Glyph 现状：`GlyphSet` → Unicode `▶/▼`，Ascii `>/v`；**没有** `(+)/(-)`。DESIGN 要求折叠行可读、旁注完整和弦，禁止装饰性 ⚙。

鼠标：`InputEvent` 目前实质是 **Key | Paste**；组件几乎只处理 Key。点按折叠 = **新的引擎能力**（mouse enable + 行命中 → entry id），不是改文案能解决的。

### 与「多级折叠」的张力

```text
已有：全局一键（好记、实现简单、无法对准「这一段」）
需要：L2/L3 段级 +「最近一段」优先（精准、要状态与焦点模型）
树已有：per-id fold + 标记（可借鉴，但是 overlay 不是 scrollback）
```

### 候选模型（归档）→ MVP = **M2**

- M1 全局再加键 — 否（无法「最近一段」）
- **M2 段状态 + 最近段和弦 — 是（本波）**
- M3 块焦点 + 标记第一公民 — 后置；标记仍用 `▶/▼`，不做 `(+)/(-)`
- M4 鼠标点击 — 后置；本波只预留行距映射
- M5 纯年龄自动 — 不作主模型（远段自动 L3 可作可选配置）

### 底层能力（M2 MVP）

| 能力 | 本波 |
|---|---|
| per-segment fold level | **要** |
| 全局 `ScrollbackFold` L1 | 保留，分层共存 |
| scrollback 块焦点 | 不要 |
| 新字形 `(+)/(-)` | 不要 |
| 引擎 Mouse | 不要；产品侧预留 segment↔行映射 |

### 已拍板（交互 · 续）

| 项 | 决定 |
|---|---|
| MVP 模型 | **M2** |
| 标记 | 现有 `▶/▼` / `>/v`；**不支持** `(+)/(-)` |
| 鼠标 | **本波不做**；**预留** hit-test 缝（entry/segment id ↔ 行范围），供日后 M4 |
| M3 块焦点 | 非 MVP；下文把与 Editor 的关系写清，避免以后踩坑 |

### 深挖 A — 与全局 Alt+E / Ctrl+T / Ctrl+O 共存

今日 L1 是 **全局 bool**；M2 的 L2/L3 是 **每段 level**。必须分层，避免「按了 Alt+E 却像没反应」或「展开一段却全局炸开」。

**推荐分层规则（意向）**：

```text
段 level == L3 或 L2
  → 只渲染摘要行（带 ▶/▼）；段内 Tool/Thinking/Bash **不进入** render
  → Alt+E / Ctrl+T / Ctrl+O **不改变该段外观**（可 no-op，或仅影响「窗口内仍为 L0/L1 的近 turn」）

段 level == L0（细账露出）或近 keep_recent_turns 的原生细账
  → 现有全局 ScrollbackFold 照常：Alt+E 切工具块、Ctrl+T thinking、Ctrl+O 视口

Alt+Shift+E
  → 只改「最近仍折叠段」的 level（L3→L2→L0，或 L2→L0；收纳可用对称和弦或再次循环，propose 钉）
  → MUST NOT 翻转全局 tools_expanded
```

**共存表**：

| 用户动作 | 近窗口细账 | 旧段 L2/L3 |
|---|---|---|
| Alt+E | 全局工具块显隐 | 无可见块 → 外观不变 |
| Ctrl+T | 全局 thinking | 同上 |
| Ctrl+O | 全局 viewport | 同上 |
| Alt+Shift+E | 无折叠段：静默 | 升细最近折叠段（一级） |
| 收纳键（若 C1） | 无已展开 Activity：静默 | 压回最近 L0 Activity 段 |

**Compaction**：今日与 Alt+E 共用；ActivityFold **不吞** Compaction 块（已拍独立短块）→ 无额外冲突。

**风险**：用户在 L2 行上看见 `▼` 却按 Alt+E——旁注应标 **`(Alt+Shift+E)`**（或段专属提示），避免误导成 Alt+E。

### 深挖 B — 焦点 vs Editor（M2 不需要焦点；预留鼠标缝）

**M2「最近一段」不靠焦点**：启发式 = 距输入框最近、仍处于 L2/L3 的 segment（entries 序上最后一段折叠）。焦点仍在 **Editor**（`EditorSlot::Editor`）；现有槽模型：Tree/Resume/… 互斥替换 editor，Esc 关槽——**scrollback 不是槽**。

若将来 M3「块焦点」：

| 做法 | 与 Editor |
|---|---|
| **不推荐**：把 scrollback 做成 EditorSlot | 无法同时打字；违「贴底输入」心智 |
| **可选 A**：瞬时焦点——按 `Alt+Shift+↑/↓` 在可折行间移动高亮，Enter 切换；任意可打印键 / Esc **立刻还焦 Editor** | 类似「浏览模式」短时 |
| **可选 B**：永不抢焦——只靠最近段启发式 + 鼠标点击标记（M4）；键盘不引入块焦点 | 与 M2 一致，鼠标增强 |

**本波（M2）**：**B 的键盘侧**——无块焦点、不抢 Editor。
**预留缝（为鼠标 / 可选 M3）**：

1. render 时记录 `segment_id → [line_start, line_end)`（或 entry_idx 范围）
2. 日后 mouse click：y → segment_id → bump level（不经 EditorSlot）
3. 点击落在非标记区：保持今日行为（若有选区/粘贴）；**MUST NOT** 因预留缝改变无鼠标时的键路径

引擎今日无 `InputEvent::Mouse`——预留缝可先落在 **产品面数据结构**（行距映射），包侧 mouse 另 change。

### 初步倾向（已升格为拍板）

1. MVP = **M2** + 现有标记。
2. 与 Alt+E：**分层**（上表），旁注区分和弦。
3. 鼠标：映射缝预留，点击实现后置。
4. 块焦点：非本波；若做，优先「瞬时浏览 + 还焦 Editor」，勿新开常驻 EditorSlot。

### 深挖 C — 双向栈（已拍）

| 动作 id（跨面 SSOT） | TUI 默认和弦 | 启发式 | 效果 |
|---|---|---|---|
| `activity.expandNearest` | `Alt+Shift+E` | 最近 **L2/L3** | 升一级（朝 L0） |
| `activity.collapseNearest` | `Ctrl+Alt+Shift+E` | 最近 **L0 Activity** | 降一级，**地板** = 该段默认粗级（未开远段 L3 → 多为 L2；开了则远段地板可为 L3） |

无目标 → 静默。近窗口从未进 ActivityFold 的细账 **不是** collapse 目标。

**「压回默认粗级」解读**：收纳是 **一级一级走向地板**，不是一键瞬移到 L3——与展开对称，避免跳过 L2 摘要。

### `Ctrl+Alt+Shift+E` 支持吗？

| 层 | 结论 |
|---|---|
| **键位解析 / `matches_key_event`** | **支持**：`parse_key_id` 独立认 ctrl/alt/shift；`ctrl+alt+shift+e` 合法 |
| **crossterm + Kitty / modifyOtherKeys** | 现代终端通常能报到三修饰 + 字母 |
| **传统 VT / 部分 tmux / 远程** | **不可靠**：三修饰字母常被吞、变成别的码、或与终端/OS 快捷键冲突 |
| **可配置** | `keybindings.json` 可改绑；旁注必须跟实际绑定走 |
| **Web** | DOM `ctrlKey+altKey+shiftKey+e` **无障碍**；还可点 `▶/▼`（鼠标面） |

**风险**：收纳比展开「多一个 Ctrl」，发现成本更高、TUI 弱终端上可能偶发按了没反应——需在文档/旁注承认，并保证 **Web 与可配键** 不堵死收纳。

### 深挖 D — 跨 TUI / Web：学习成本只要「一套」

用户要学的是 **语义**，不是某个终端和弦：

```text
一套心智（两客户端共同）
  · 旧操作可收成摘要（L2）或 Worked for（L3）
  · 从「靠近输入」一侧：展开 = 往细里剥；收纳 = 往粗里盖
  · ▶ = 还可展开；▼ = 还可收纳（或已细）
  · 最近几轮保持细账；Alt+E 只管细账里的工具块（另一层）

各面只换「怎么触发」
  TUI：默认 Alt+Shift+E / Ctrl+Alt+Shift+E（可改绑）
  Web：同名动作 + 可点标记 + 可选相同快捷键
```

**合约分层（意向）**：

1. **动作语义**（跨面 MUST）：`expandNearest` / `collapseNearest`、段 level、地板规则、与 L1 分层——进 shared 产品说明 / 未来 protocol 或 docs，**不**把 TUI 和弦写进 Web MUST。
2. **默认绑定**（分面 SHOULD）：TUI vs Web 表格式列出；允许用户改。
3. **发现性**：摘要行旁注用 **动作名或短标签** 优于只堆 `(Ctrl+Alt+Shift+E)`；弱终端可依赖 `/` 或设置页看绑定（另案）。

### 这样设计还有什么问题？（审慎清单）

| 问题 | 严重度 | 缓解 |
|---|---|---|
| 三修饰收纳难按、难记、旁注很长 | 中 | 可配键；Web 靠点击；旁注可写「收纳」+ 短和弦 |
| 部分 TUI 终端收不到三修饰 | 中 | 文档说明；默认外提供双修饰备选绑；测 Kitty/主流 |
| 展开易、收纳难（不对称） | 低–中 | 接受「破坏性反向稍难点」；或 Web/可配改为对称双修饰 |
| 只从底操作，想关很旧一段要先盖近段 | 低 | 栈模型的代价；鼠标/焦点后置可点指定段 |
| collapse 地板 ≠ 用户上次停过的 L2 | 低 | 一级步进；地板=默认粗级已拍 |
| 与全局 Alt+E 认知混淆 | 中 | 旁注区分；L2/L3 行不写 `(Alt+E)` |
| Mac：Ctrl vs ⌘ | 中（跨面） | Web 用 Mod（⌘/Ctrl）规范；TUI 仍 ctrl——**心智讲「修饰更多=收纳」**，不讲死 OS 键名 |
| 两客户端快捷键不完全同一物理键 | 低 | 刻意：统一的是动作栈，不是键帽 |

**结论**：语义栈（C1）适合作为 TUI+Web 共同学习模型；`Ctrl+Alt+Shift+E` **库支持、终端有条件**，作 TUI 默认可接受但 MUST 可配 + Web 不依赖它。

## Open Questions（剩余）

1. ~~弱终端官方第二默认~~ → **已拍：只靠用户 json**
2. ~~摘要行旁注~~ → **已拍：先满和弦（形态 A）**；后续可调
3. （暂无）

### 旁注举例（已选 A）

```text
▶ Explored 6 files, 5 searches, ran 9 commands  +22 -19  (Alt+Shift+E)
▼ Worked for 2m 3s  (Ctrl+Alt+Shift+E)
```

## Ethics

- risk_level: low–medium
- prohibited_actions: 静默丢细账；L3 伪造时长；fold 冒充 session compaction；偷渡 delayed 实现；本波抢 Editor 常驻焦点；把 Alt+E 绑成段级展开
- required_evidence: 多级行数/微基准；展开可逆（apply 前）
- escalation_policy: 改 JSONL/模型上下文 → 用户确认
