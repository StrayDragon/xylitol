# Verify — c1760-add-tui-activity-fold

**Date:** 2026-08-12
**Branch:** `sdd/c1760-add-tui-activity-fold`
**base_sha (proposal):** `839990764660a6b9f9014d1a88ee2d35aafeccd6`
**HEAD (pre-verify-doc):** `75bbf0fdfa4fdcbf644f05945ad46f78dec559b4`
**Stage:** `full` · `readyToImplement=true` · `specsLanded=true` · attached

## Gates

| Gate | Result |
|---|---|
| Branch binding | `sdd/c1760-add-tui-activity-fold` |
| `llman sdd show c1760 --type change --output json` | `readyToImplement=true` / `specsLanded=true` |
| `llman sdd validate c1760 --strict --no-check` | **PASS**（INFO: `depends_on` 引用已归档 c1755 / c2070） |
| Focused activity_fold | **11 passed**（7× `activity_fold_att*` + 4× module unit） |
| L1 回归（c2040） | **8 passed**：`harness_mouse_triangle_*`×4、`harness_mouse_drag_across_triangle…`、`thinking_per_id…`、`tools_per_block…`、`scrollback_thinking_fold…` |
| Segment 鼠标 / `FoldTarget::Segment` | **未实现**（见下） |
| Wave 外（c2045 / c2050 点击 / c1505·c1370·c1535） | **未偷渡** |

## Hard constraints (c1760)

| Constraint | Evidence |
|---|---|
| 段阶梯 L0/L2/L3；无段级 L1 | `SegmentLevel` + `activity_fold/`；与 `ScrollbackFold` 分离 |
| L2 计数 / L3 Worked for；无假时长 / 假 +/- | `summary.rs` + `activity_fold_att24_*` |
| C1 `expandNearest` / `collapseNearest`；无目标静默 | keybindings + `slot_input` + `activity_fold_att28_*` |
| 与 L1 分层；L2/L3 不登记块三角 | `render_scrollback` skip middles；`att25` harness |
| auto degrade：keep=2、rebuild/turn-end、流式保护 | `degrade.rs` + host `AgentEnd` / rebuild path；`att26` |
| 行距缝 `segment_id → [start,end)`；**不**进 `FoldHitTable` | `SegmentRowSpans`；`FoldTarget` 仍仅 Tool/Diff/Ask/Thinking |
| 旁注满和弦；禁止 `(Alt+E)` | `format_summary_line` + `att27` |

### Segment 鼠标（显式未交付）

- `FoldTarget` **无** `Segment` 变体（`fold_hit.rs` 未改）。
- `activity.row_spans` 仅 paint 预留；**无** `set_transcript_hit_priority` / hit 接线到段摘要。
- 模块头注释与 att23 MUST NOT 对齐；点击属 c2050（可被 c2045 吸收）。

---

## 合约轴（Spec）

审查对象：live `app-tui-transcript` **att23–att28**（`feature: false` unit）；实现对照 `activity_fold/*`、`widgets/scrollback.rs`、`layout/root/*`、`keybindings.rs`、`host/{mod,session_ops}.rs`。
Diff 范围：`main...HEAD`（base=`83999076`）。

### CRITICAL

（无）

### WARNING

（无行为缺口）

### SUGGESTION

1. **Thinking/Ask-only → L2 `files=1` 占位**（`summary.rs`）：无 files/search/command 计数时仍画「Explored 1 file」，避免空摘要。与 att24「无中间操作可计 → 不生成空摘要」可并存，但类目略牵强；后续可改中性文案（如 `Activity`）而不改阶梯语义。
2. **design 示例 L3 旁注为收纳和弦**，实现 L2/L3 一律画 `expandNearest`（折叠态展开旁注）。att27 要求默认和弦与改绑跟随，**未**强制 L3 画 collapse；与 c2040「折叠行示 expand」一致。归档前可不改；若要对齐 design 图再开小修补。
3. **同源板残留句**：`docs/roadmaps/Web与TUI同源.md` M1b 表已指 c1760，但文末「Activity 折叠草案…**延后**」与能力表「apply 中」未完全收口——archive 时可顺手改成「TUI 已交付 / Web 未兑现」。

### Covered requirements

| Req | Verdict | Notes |
|---|---|---|
| **att23** activity-segment-levels | PASS | 稳定 `seg-{user_idx}`；L2 摘要；User/Assistant/ScrollNotice 外显；行距缝；无段鼠标 |
| **att24** l2-summary-l3-worked-for | PASS | 有戳 `Worked for 2m 3s`；无戳省略时长数字；可靠 Diff 才 +/- |
| **att25** layered-with-l1 | PASS | L2 时 Alt+E 外观不变；升 L0 后 L1 override 恢复；局部 paint misses≤3 |
| **att26** auto-degrade | PASS | keep=2；rebuild crush；`protect_newest` 流式；code-first defaults |
| **att27** markers-and-hints | PASS | fold 标记；`(Alt+Shift+E)`；禁止 `(Alt+E)` |
| **att28** expand/collapse nearest | PASS | 一级可逆；近窗 virgin 静默；焦点不抢 Editor |

### Out-of-scope（正确未做）

- Segment 摘要鼠标 / `FoldTarget::Segment`（c2050 / c2045）
- c2045 剩余块点折 / 广义 FoldTarget
- c1505 切片 / c1370 热缓冲 / c1535 wrap
- runtime-config YAML MUST（att26 允许 code-first）

---

## 标准轴（Standards）

审查对象：`src/app/tui/AGENTS.md` 分层 / 鼠标边界；坏味清单（判断性）。

### CRITICAL

（无）

### WARNING

1. **`scrollback.rs` 继续承载段折叠 paint**（与 c2040 verify 同债）：L2/L3 摘要插入与 L1 hit 同函数。行为正确；c2045 前宜外提 segment paint / hit 缝，避免 Divergent Change。

### SUGGESTION

1. `activity_fold/mod.rs` 上 `#[allow(unused_imports)]` 再导出 `ActivityFoldSettings` / `SegmentRowSpans`——配置面后补可留；否则可改为调用方直路径减少 allow。
2. `ingest_rebuild_clocks` 按 ancestry User 序对齐 segment：空 turn 跳过时依赖 `partition_segments` 过滤，harness 未覆盖「中间空 turn + 乱序戳」边角——人验/后续加固可选。
3. 坏味扫描：无明显 Feature Envy / Speculative Generality（未扩 `FoldTarget`）；`SegmentRowSpans` 预留缝符合提案边界，非过度抽象。

### AGENTS / 分层

| 检查 | 结果 |
|---|---|
| 改动落在 `app/tui`；未穿 `agent`↔`infra` | PASS |
| 鼠标仍仅 L1 三角经 `set_transcript_hit_priority` | PASS（host 未接 segment） |
| 动作 id 与同源板 M1b 一致 | PASS（文档指针已更新主体） |
| 禁止 Wave 外性能实现 | PASS |

---

## Verdict

**PASS — 可 archive**（合约轴 / 标准轴均无 CRITICAL；Segment 鼠标确认未实现）。

建议下一步：`llman-sdd-archive` / `change finalize`（本 verify **不** finalize / **不** push）。
可选：人验最短路径见 [`_HUMAN_CHECKLIST.md`](./_HUMAN_CHECKLIST.md)；顺手收口同源板「延后 / apply 中」措辞。
