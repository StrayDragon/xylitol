# Design: 广义 FoldTarget 剩余可折块

## 目标边界

| 在范围 | 不在范围 |
|---|---|
| 扩展 `FoldTarget` + 同一 `FoldHitTable` 登记剩余可点目标 | 重做 c2040 四类 L1（Tool/Diff/Ask/Thinking） |
| **Wave A**：Compaction 三角；OutputViewport（Ctrl+O）命中 | 复活 keyboard fold-leader / 数字编号 |
| **Wave B**：`Segment` 命中（吸收 c2050）；点摘要标记 = 该段一级 toggle | 实现 c1760 L2/L3 文案 / 段状态机本身 |
| 手势复用 c2040：A+latch；拖选中忽略 fold | 改差分引擎；产品自管 scroll；整行可点（除 Viewport hint 例外） |
| 性能：hit O(可见)；toggle 局部 miss（ath25） | 实现 c1505 / c1370 / c1535 |

## 代码事实（c2040 地基）

| 构件 | 今日 |
|---|---|
| `FoldTarget` | `Tool` / `Diff` / `Ask` / `Thinking`（均 `String` id） |
| `FoldHitTable` | 可见三角列 → screen→content hit；host `set_transcript_hit_priority` |
| `ScrollbackFold` | tools/thinking default+overrides；**全局** `tools_output_expanded` / `compaction_expanded` |
| Bash | **无** L1 三角；输出预览走 Ctrl+O（`tools_output_expanded`） |
| Compaction | **无**三角；`(Alt+E to expand)` 文案；全局 `compaction_expanded`；Alt+E 连带翻 tools |
| att22 | 三角列-only 仅覆盖 att20/att21；**显式不含** L2/L3 / 无三角块 |

## 决策表（已钉）

| # | 议题 | 钉 | 理由 |
|---|---|---|---|
| D1 | c2050 处置 | **(a) 吸收** | 单一 `FoldTarget`/`FoldHitTable` 车道；禁止平行段命中 API。c2050 → **docs-only**（不再独立 design/apply）；Segment 语义与验证迁入本 change Wave B。c2050 proposal 已有「可被 c2045 收薄」注；propose/start 后可把其 status 改 supersede 指针（本设计阶段不改 c2050 目录）。 |
| D2 | 吸收时机 vs c1760 | **Wave B 硬闸 c1760** | 段状态机未落地前禁止假装 Segment 点击。Wave A **不**等 c1760（`depends_on` 仅 c2040）。c1760 归档前：c2050 保持 dormant；Wave B 完成后 docs-only archive c2050。 |
| D3 | Bash | **不补 L1 三角** | 块无折叠态；输出高度属 Viewport 族。命中走 D5。 |
| D4 | Compaction | **补三角列**；点三角 = flip **全局** `compaction_expanded` | 对齐今日单 bool；本波不引入 per-compaction overrides；**不**因点 Compaction 而清 tools overrides / 翻 `tools_expanded`（软拆连带：键盘 Alt+E 仍可连带，鼠标定点只动 compaction）。 |
| D5 | Ctrl+O viewport | **纳入** `FoldTarget::OutputViewport` | 与键盘同构（全局 `tools_output_expanded`）。命中几何 = 可见 expand/collapse **hint 行**上的可点带（hint 文案区；无三角 → att22 三角-only **不适用**；Specs 另开 req）。Tool/Bash/Diff 等凡渲染该 hint 的块均可登记同一 target。 |
| D6 | `FoldTarget` 形状 | **加变体，不硬改名** | 保留 `Tool|Diff|Ask|Thinking`；新增 `Compaction`、`OutputViewport`、`Segment(id)`。MAY 日后收成 `Entry{kind,id}`，本 change **不**强迁。 |
| D7 | Segment 点击语义 | **点摘要标记 = 该段一级 toggle**（升/降朝栈对称） | 对齐 c2050 / c1760「单目标版」；**不是** `expandNearest` 启发式。分层规则继承 c1760 深挖 A（L2/L3 时 L1 全局/覆盖不穿透该段外观）。 |
| D8 | 命中几何（有三角者） | **仅三角列 1 cell** | 与 att22 / c2040 同构；Compaction / Segment 摘要标记同列语义。 |
| D9 | Host 路由 | **复用**现 hit_priority + latch | `app-tui-host` 仅在 toggle 分支扩 match；无新管道。 |

## 状态 / 命中模型

```text
FoldTarget
  Tool(id) | Diff(id) | Ask(id) | Thinking(id)   // c2040 ✓
  Compaction                                       // Wave A：全局 compaction_expanded
  OutputViewport                                   // Wave A：全局 tools_output_expanded
  Segment(segment_id)                              // Wave B：c1760 段 level ±1

Left Down (screen)
  if selection_dragging → ignore fold
  else if FoldHitTable.hit → toggle_fold_target → swallow
  else → 既有选区 / dock / Editor
```

| 动作 | 效果 |
|---|---|
| 点 Compaction 三角 | `compaction_expanded = !compaction_expanded` |
| 点 OutputViewport hint | `tools_output_expanded = !tools_output_expanded` |
| 点 Segment 标记 | 该段 level 一级 toggle（Wave B；细节随 c1760 API） |
| Ctrl+O / Alt+E（键盘） | 保持今日语义（含 Alt+E×compaction 连带） |
| 拖选中 | 忽略 fold（latch） |

## Wave 切分

```text
Wave A（depends: c2040 ✓）
  FoldTarget += Compaction | OutputViewport
  scrollback 登记 Compaction 三角 + hint 命中
  toggle_fold_target 扩分支
  harness：Compaction / Viewport 点击

Wave B（blocked-by: c1760 归档）
  FoldTarget += Segment(id)
  摘要行标记 → hit；toggle = 段一级
  吸收 c2050 验证矩阵；docs-only 收口 c2050
```

## 与 c1760 / c2050 边界

| Change | 拥有 | 本 change |
|---|---|---|
| c1760 | 段 level、摘要文案、`expandNearest`/`collapseNearest`、行距缝 | 消费段 id↔行映射填 hit |
| c2050（原） | 段鼠标 + 统一目标叙事 | **全部迁入** Wave B；本 id 吸收后不再实现 |
| c2040 | L1 四类三角 + per-id + latch | 不重做；只扩 enum / 表 |

**禁止**：c1760 未有段状态时用全局 bool 假装 Segment；c2050 另开第二套 hit 表。

## Paint / 性能

- hit 表只登记**当前视口可见**头/摘要/hint（与 paint gen 同代）。
- Compaction / Viewport / Segment toggle → 指纹/cache 局部失效；**禁止**全历史 MD 重解析。
- 与 ath25 miss 上界：不因本 change 回退。
- c1505 若已切片：只扫 slice；未切片则扫可见 upper（同 c2050 research）。

## Specs 意向（landing 在 `change start` 之后）

Capability：`app-tui-transcript`（主）；`app-tui-host` 仅当 latch/路由缺口才加（预期 skip）。

| 意向 req | 要点 |
|---|---|
| Compaction 三角 | AO 下点三角 flip `compaction_expanded`；点正文不 toggle；不改 tools overrides |
| OutputViewport 命中 | 点可见 hint 带 flip `tools_output_expanded`；与 Ctrl+O 同构 |
| Segment（Wave B） | 点 L2/L3 标记 → 该段一级；L1 不穿透（深挖 A）；依赖 c1760 |
| 统一表 | 剩余目标 MUST 进既有 `FoldHitTable`；禁止第二管道 |

场景优先 `feature: false` + 产品 harness（对齐 att20–22）。

## 验证 seam

1. Harness Mouse：Compaction 三角 toggle；误点正文不变。
2. Harness Mouse：OutputViewport hint toggle；与 Ctrl+O 同 bool。
3. Harness（Wave B）：同屏 L1 三角 + L2 摘要各打中正确 `FoldTarget`；L2 内 L1 不穿透。
4. ath25：上述 toggle 不回退 miss 上界。
5. 回归：c2040 Tool/Thinking 三角行为不变。

## Start readiness

| 项 | 状态 |
|---|---|
| Open Questions | **全清**（见 proposal） |
| design.md + tasks.md | **有** |
| `depends_on` c2040 | **已归档** |
| c2050 吸收策略 | **已钉 D1/D2** |
| live specs | **未改**（须 `change start` 后 Specs landing） |
| 应用代码 | **未改**（本阶段禁止） |
| Wave B | tasks 标 `[blocked-by: c1760]`；不挡 Wave A start |

**ready_for_start: true** — 可在默认分支干净树上 `llman sdd change start c2045-add-tui-fold-target-remaining`，再 Specs landing（先 Wave A req；Wave B req 可同批写下但 apply 闸 c1760）。

## Ethics

- risk_level: low–medium
- prohibited_actions: 双写第二 hit 表；c1760 前假装 Segment；点 Compaction 误清 tools 覆盖；Viewport 整屏误 toggle
- required_evidence: harness 三类新目标 + c2040 回归；Wave B 另要混合 L1+L2 屏
- escalation_policy: 若要改 Alt+E×compaction **键盘**连带语义 → 另 change / 升级确认（本 change 只软拆鼠标定点）
