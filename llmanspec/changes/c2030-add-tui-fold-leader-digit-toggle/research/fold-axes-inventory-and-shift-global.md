# Research: 产品折叠轴盘点 + Leader/Shift 全局重载意向（c2030）

> 一手代码 + 已归档/活跃提案。终端和弦可达性见同目录 `terminal-chord-alt-shift.md`（并行调研）。

## 1. 今日折叠 / 展开轴盘点（真值）

### 1.1 Live scrollback（`ScrollbackFold` · `scrollback.rs`）

| 轴 | 全局字段 | 默认 | 默认和弦 | 作用对象 | 备注 |
|---|---|---|---|---|---|
| Thinking 块 | `thinking_expanded` | **关** | `Ctrl+T` (`app.thinking.toggle`) | `UiEntry::Thinking` | 与树开时 `Ctrl+T` filter **同键异槽** |
| Tool/Diff/Ask **块**显隐 | `tools_expanded` | **开** | `Alt+E` (`app.tools.blocks`) | Tool、Diff、Ask 正文 | Ask.`expanded` 字段死；渲染跟 tools |
| Tool/Bash/Diff **视口高度** | `tools_output_expanded` | 预览 | `Ctrl+O` (`app.tools.expand`) | 已展开块的 max-height | 与 Alt+E **正交**（DESIGN expandable） |
| Compaction 摘要 | `compaction_expanded` | **关** | **同** `Alt+E`（与 tools 同翻转） | `UiEntry::Compaction` Complete | 无独立 action id |
| Bash 命令行 | — | 常显 | — | Bash | **不**跟 `tools_expanded`；输出只跟 Ctrl+O |

处理真源：`layout/root/slot_input.rs` — Alt+E **同时** `^= tools_expanded` **与** `compaction_expanded`。

### 1.2 会话树 / Resume（非 scrollback）

| 轴 | 模型 | 和弦（节选） | 作用域 |
|---|---|---|---|
| 树节点 fold | `folded_nodes` per-id | Ctrl/Alt+←→ 等（包转发） | **已是 per-id** |
| 树 filter | 多种 | Ctrl+T/U/L/A/O…（树开优先） | 过滤，非块折叠 |
| Resume 子会话折叠 | 面板内 | Ctrl/Alt+←→ | Resume 槽 |

### 1.3 计划中（未落地 · `c1760`）

| 轴 | 动作 | 默认和弦（提案已拍） | 层级 |
|---|---|---|---|
| Activity 段升细 | `activity.expandNearest` | **`Alt+Shift+E`** | L2/L3 |
| Activity 段收纳 | `activity.collapseNearest` | **`Ctrl+Alt+Shift+E`** | L0→地板 |

→ **硬冲突预警**：用户意向「`Alt+Shift+E` = 原 Alt+E 的全局 toggle」与 `c1760` **直接撞车**。须在 Wave 对齐时二选一或改绑其中一方。

### 1.4 非本波 / 勿混

- Editor 折叠、补全 overlay、TooSmall hint、鼠标 capture（c2020）——不是 scrollback fold。
- 包 `ExpandableOutput` 只渲染给定态；产品编排折叠。

---

## 2. 用户意向映射（Q1）

> 「默认把对应的之前全部展开变为 leader；加 Shift+ 作为全部 toggle」

| 今日「全部」和弦 | 意向 leader（定点） | 意向全局 toggle（Shift+） | 冲突 / 注意 |
|---|---|---|---|
| `Alt+E` tools+compaction | `Alt+E` → FoldLeader（编号→digit） | `Alt+Shift+E` → 全局 tools（±compaction？） | **撞 c1760 `expandNearest`** |
| `Ctrl+T` thinking | `Ctrl+T` → ThinkingLeader？ | `Ctrl+Shift+T` → 全局 thinking | 树开时 Ctrl+T 已是 filter；Shift 变体终端/浏览器习惯需查 |
| `Ctrl+O` 视口 | `Ctrl+O` → ?（视口是否适合 leader 存疑） | `Ctrl+Shift+O` | **已被** `app.tree.filter.cycleBackward` 占用（树开） |

### 2.1 建议分层（待拍板，非正式决议）

**L1 块显隐（c2030 核心）**

1. `Alt+E` = tools/diff/ask **leader**（视口可折叠头编号）。
2. 全局 tools toggle：**不要默认抢 `Alt+Shift+E`**，除非正式改 `c1760` 和弦；候选：
   - `Alt+Ctrl+E` / `Alt+Shift+W` / 可配 + 旁注；或
   - 保留 `Alt+Shift+E` 给 c1760，全局 tools 用 **`Alt+Shift+B`**（blocks）等。
3. Compaction：从 Alt+E 捆绑中**拆出**独立全局键（否则 leader 与 compaction 心智纠缠）——Open。

**Thinking**

- 首波可：**`Ctrl+T` 仍全局**（改动面小），或对称改 leader + `Ctrl+Shift+T` 全局——与树槽同键问题仍在。
- 不建议本波强行 thinking leader，除非用户坚持「都需要」。

**Ctrl+O 视口**

- 正交高度开关，**不适合**「块编号 leader」隐喻；建议 **保持全局**，不为 Shift 对称而对称。

---

## 3. Q3 编号范围 / 滚动 / 取消复原（设计选项）

代码约束（见 `fold-leader-seams-code-facts.md`）：无现成「视口内折叠头」API；须自扫。

| 方案 | 编号数 | 滚动感知 | 性能 | UX |
|---|---|---|---|---|
| **N1** 仅最近 1 块标 `1` | 1 | 固定「距输入最近」 | 最优 | 几乎不用数字选择；leader 退化成「toggle 最近」——可用但弱于「定点多块」 |
| **N10** 视口内最近 ≤10（`1`–`9`/`0`） | ≤10 | 进 leader 时扫当前视口行距 | 中（O(可见 entry)） | 符合原草案；滚后再按 leader 刷新 |
| **N-scroll** 随滚动持续重标 | ≤10 | 滚动画布时重算 | 差/易闪 | **不推荐**本波；与「瞬时模式」冲突 |
| **N1+scroll hint** | 1 | 旁注提示「滚动后重按 leader」 | 优 | 折中 |

**取消复原（用户已倾向）** — 建议钉：

进入 `FoldLeaderMode` 后，下列任一 **立即退出且不改 fold**（数字键除外）：

- Esc
- 任意**非映射**可打印键 / 编辑键（打字）→ 退出后该键 **照常进 Editor**（或：先退出再投递同一 Key——须测，避免丢首字）
- 再按同一 leader（刷新或退出——二选一；倾向 **退出** 或 **刷新映射**）
- Resize / 打开 overlay / 提交消息

超时：可选 3–5s；非 MUST。

**投递首字**：推荐 listener `Consumed` 仅吞 digit/`Esc`/leader；其它 Key 返回 `Continue` **并**先清 mode，使 Editor 仍收到该键（或 host 显式 re-dispatch）。禁止「取消 mode 却吞掉用户刚打的字」。

---

## 4. 与 c2040 共享

Per-entry override 表（tool/ask `id`；Diff 可能需合成键 / entry index）由 c2030 落地；c2040 点击只写同一表。全局 Shift-toggle：**清空 overrides** 或 **只改 default 保留 overrides**——须另钉（倾向：全局 toggle 改 default **并清空** overrides，避免「按了全局看起来没反应」）。

---

## 5. 待终端调研回答的问题（Q2）

写入 `terminal-chord-alt-shift.md` 后回填：

1. `Alt+Shift+E` / `Ctrl+Shift+T` / `Ctrl+Alt+Shift+E` 在 Kitty / Ghostty / WezTerm / Foot 是否稳定到达应用？
2. 是否与终端自身绑定冲突（新标签、分割等）？
3. 若 `Alt+Shift+E` 可达：仍建议与 c1760 文档冲突优先解决，而非静默双绑。
