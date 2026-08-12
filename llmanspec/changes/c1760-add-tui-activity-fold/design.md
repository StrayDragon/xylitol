# Design: c1760 activity-fold（L0–L3）

## 目标边界

| 在范围 | 不在范围 |
|---|---|
| 段级 `SegmentLevel`：L0 / L2 / L3 | 段级「L1」（L1 = 块级，已由 c2040） |
| L2 计数摘要行 + L3 `Worked for` | 假时长 / 假 `+/-` |
| C1：`expandNearest` / `collapseNearest` | keyboard fold-leader / 数字编号 |
| 配置：`keep_recent_turns`、rebuild/turn-end auto、远段 L3 | c1505 切片 / c1370 热缓冲 / c1535 wrap |
| 与 L1 default+overrides **分层共存** | 重做 c2040 三角 / ThinkingId / 字形 |
| render：`segment_id → 行距` 预留缝 | 段摘要鼠标点击（c2050 / 可被 c2045 吸收） |
| 动作语义进 Web 同源板（文档） | Web UI 实现 |

## 代码事实（设计锚）

| 现状 | 含义 |
|---|---|
| `ScrollbackFold` = 全局 default + `tools_overrides` / `thinking_overrides`（c2040） | L1 已是 per-id；本 change **另加**段 map，勿塞进 overrides |
| `FoldTarget` = Tool/Diff/Ask/Thinking | **本波不扩** Segment 变体；行距缝可先产品私有结构 |
| `UiEntry` **无** timestamp 字段；`session_entry_to_ui_entries` 丢 session base 戳 | L3 MUST 另开时钟源（见下）或投影旁路带戳 |
| `UiRoot` 持 `fold` + `fold_hits` | 段状态挂 root（或等价 UI 状态），与 EditorSlot 无关 |
| 键位目录 `app.thinking.toggle` / `app.tools.blocks` / `app.tools.expand` | 新加 `app.activity.expandNearest` / `app.activity.collapseNearest` |

## 语义阶梯

```text
SegmentLevel（本 change）          BlockFold / L1（c2040，正交）
─────────────────────────          ────────────────────────────
L0  段内细账可见          ←→       tools/thinking default+overrides
L2  一行活动计数摘要               （段内块不进入 render）
L3  一行 Worked for …              （同上）
```

**原则**：越旧可越粗；最近 `keep_recent_turns`（默认 2）保持 L0；流式**当前**回合禁止压段。

### 段（segment）定义

一个 Activity 段 ≈ 一轮用户可见「中间操作墙」：

```text
[User]
  … Tool / Thinking / Diff / Ask / Bash …   ← 可被 L2/L3 收纳
[最终 Assistant 文本]                        ← 外显（与 User）
ScrollNotice / Error / Compaction            ← 永不进 Activity 收纳
```

- **稳定 `segment_id`**：live 与 travel/fork/resume 重建 MUST 同构（可用边界 entry id / 轮次序；具体字段属实现）。
- 近窗口内「从未进 ActivityFold」的细账 **不是** `collapseNearest` 目标。

### 级间可见性

| SegmentLevel | 用户看到 | 段内 L1 |
|---|---|---|
| L0 | User + 中间块 + 最终 Assistant（块服从 L1） | 生效 |
| L2 | User + **摘要行** + 最终 Assistant | 不渲染 → 外观不变 |
| L3 | User + **`Worked for …` 行** + 最终 Assistant | 同上 |

## 与 L1 overrides 分层（深挖 A，规范性）

```text
段 level ∈ {L2, L3}
  → 只画摘要/Worked 行（▸/▾ + 旁注）
  → Alt+E / Ctrl+T / Ctrl+O 与 per-id overrides
       可仍改全局/表，但 MUST NOT 改变该段外观
  → 段内三角 hit MUST NOT 登记（无可点块）

段 level == L0（或近窗口原生细账）
  → c2040 路径全开：default + overrides + 三角 hit
```

| 用户动作 | 近窗口 / 段 L0 | 旧段 L2/L3 |
|---|---|---|
| Alt+E | flip tools default + 清 tools overrides | 外观不变 |
| Ctrl+T | flip thinking default + 清 thinking overrides | 外观不变 |
| Ctrl+O | 全局 viewport | 外观不变 |
| 点 L1 三角 | 单块 toggle（c2040） | 无三角 |
| Alt+Shift+E | 无 L2/L3：静默 | 最近折叠段升一级 |
| Ctrl+Alt+Shift+E | 无 L0 Activity：静默 | 最近 L0 Activity 降一级 |

**旁注**：L2/L3 行标注段栈和弦，**禁止**写 `(Alt+E)` 以免与 L1 混淆。

## 键盘与动作（C1）

| 动作 id（跨面 SSOT） | TUI 默认键 | 启发式 | 效果 |
|---|---|---|---|
| `activity.expandNearest` | `Alt+Shift+E` | 距输入最近的 **L2/L3** | 升一级：L3→L2→L0 |
| `activity.collapseNearest` | `Ctrl+Alt+Shift+E` | 距输入最近的 **L0 Activity** | 降一级，地板=默认粗级 |

- **同键跨级**：一对和弦覆盖全部级间步进；不另开 L3↔L2 专用键。
- 产品键位 id：`app.activity.expandNearest` / `app.activity.collapseNearest`（可改绑）。
- 三修饰收纳：库支持、终端有条件；**MUST** 可配；弱终端不另发官方第二默认。
- **无块焦点**：启发式 = entries 序上最靠近输入的合格段；焦点留 Editor。

### 旁注举例

```text
▸ Explored 6 files, 5 searches, ran 9 commands  +22 -19  (Alt+Shift+E)
▾ Worked for 2m 3s  (Ctrl+Alt+Shift+E)
```

（实际旁注跟**当前绑定**走，非写死字符串常量。）

## 配置降级

意向键（落点 `runtime-config` 或产品 TUI 配置节；字段名实现可微调，语义固定）：

| 键 | 默认 | 语义 |
|---|---|---|
| `activity_fold.enabled` | `true` | 总开关；关则全 L0（L1 仍可用） |
| `activity_fold.keep_recent_turns` | `2` | 最近 K 轮保持 L0 |
| `activity_fold.auto_on_rebuild` | `true` | travel/resume/fork 重建后压超窗口段→L2 |
| `activity_fold.auto_on_turn_end` | `true` | 回合结束后压超窗口段→L2（流式中不压当前） |
| `activity_fold.auto_l3_distant` | `false` | 更旧段自动 L3；开时地板可为 L3 |

降级序：**关总开关 → 全 L0**；开则先 L2 策略，再可选 L3。

## L2 / L3 文案

### L2

- 英文计数模板（Cursor 体）：文件 / 搜索 / 命令等可观测类目聚合。
- `+/-`：**仅**当段内 Diff 有可靠统计时附加；否则省略。
- 无中间操作可计 → 不生成空摘要段（保持 L0 或跳过）。

### L3 时长

| 方案 | 本波 |
|---|---|
| 段 wall clock | **采用**：`t(段末/最终 Assistant) − t(段首 User 或首中间操作)` |
| 工具 span 之和 | **不用**（UiEntry 无稳定 per-tool 起止；并行难定义） |

**时钟源（代码债）**：今日 `session_entry_to_ui_entries` **丢** `SessionEntry` base `timestamp`。本 change MUST 任选其一（实现细节）：

1. 投影时旁路保留段边界戳（不强制每个 `UiEntry` 带戳）；或
2. 段状态在 rebuild 时直接从 session 树计算 level + duration。

缺任一端可靠戳 → **停 L2** 或 L3 行省略时长字段；**MUST NOT** 伪造。

## 性能边界

| 要求 | 说明 |
|---|---|
| 少画 | L2/L3 段不 flatten 段内块行（主 ROI） |
| paint | 改段 level → 仅失效该段行范围 / fingerprint；**禁止**全历史 Markdown 重解析（对齐 ath25 精神） |
| hit 预留 | `segment_id → [line_start, line_end)` 只服务日后点击；本波 **不**登记进 `FoldHitTable` |
| 与 c1505 | 未切片时仍全量 traverse entries，但 L2/L3 使单段行数≈O(1)；不替代切片 |
| 与 c1370 | 不放宽热缓冲；少画是副作用 |
| 流式尾 | 不宣称治流式 wrap（c1535） |

## 与 c2045 / c2050 边界

```text
c2040 ✓  L1 三角四类 + per-id + FoldHitTable(Tool|Diff|Ask|Thinking)
   ↓
c1760    段 level + L2/L3 行 + 键盘栈 + 配置 + 行距缝（无 Segment FoldTarget）
   ↓
c2050    段点击 / 与 L1 统一目标（depends 本 change）
   ∥
c2045    广义 FoldTarget + 剩余块；propose 时可吸收 c2050
```

| 本波 MUST | 本波 MUST NOT |
|---|---|
| 段状态与键盘可验收 | 把 `FoldTarget::Segment` 当交付（留给 c2050/c2045） |
| 行距映射数据结构 | 接线 `set_transcript_hit_priority` 到段摘要 |
| 分层表（上）进 specs | 复活 leader 编号 |

## 测试 seam（Specs / Apply 意向）

1. **Harness**：超 `keep_recent_turns` 的旧 turn → L2 行；expand/collapse 一级可逆；无目标静默。
2. **分层**：段 L2 时 `Alt+E` 不改变该段行；升到 L0 后 L1 三角/overrides 恢复有效。
3. **L3**：有双端戳 → `Worked for`；缺戳 → 不出现假时长。
4. **自动**：rebuild / turn-end 压窗；流式中当前回合保持细账。
5. **Paint**：段 level 切换 miss 局部（不对全历史 MD）。
6. **键位**：默认和弦 + 改绑后旁注一致（抽测）。
7. 场景形态：优先 `feature: false` unit + harness（对齐 c2040 att*）；合约句进 `app-tui-transcript`（+ 配置若落 `runtime-config`）。

## 依赖顺序（实现）

```text
Specs landing（段语义 + 分层 + 动作 id）
  → 段划分 / segment_id / 时钟旁路
  → SegmentLevel 状态机 + 自动降级
  → L2/L3 render + 旁注 + 行距缝
  → 键位接线 C1
  → harness / 局部 paint
  → validate（确认未越界进 c2045/c2050 点击）
```
