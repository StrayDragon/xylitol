# Verify — c2045-add-tui-fold-target-remaining（Wave A + B）

**Date:** 2026-08-13
**Branch:** `sdd/c2045-add-tui-fold-target-remaining`
**base_sha (proposal):** `839990764660a6b9f9014d1a88ee2d35aafeccd6`
**HEAD (pre-verify-doc):** Wave B `25591b29` + c2050 docs-only archive
**Stage:** `full` · `readyToImplement=true` · `specsLanded=true` · attached
**Scope:** **Full change**（att29–att32；c2050 docs-only）

## Gates

| Gate | Result |
|---|---|
| Branch binding | `sdd/c2045-add-tui-fold-target-remaining` |
| `llman sdd validate c2045 --strict --no-check` | **PASS**（INFO: depends_on archived c2040；blocks archived c2050） |
| `llman sdd validate app-tui-transcript --strict --no-check` | **PASS** |
| `cargo test --lib harness_mouse` | **8 passed**（含 Segment） |
| `cargo test --lib segment_toggle` / `harness_l2_ignores` | **PASS** |
| `cargo test --lib activity_fold` | **11 passed** |
| c1760 | **archived** `2026-08-12-c1760-…` → Wave B unblocked |
| c2050 | **docs-only archived** `2026-08-13-c2050-…` |

## Wave 切分结论

| Wave | 合约 | 实现 | Verify |
|---|---|---|---|
| **A** | att29 / att30 / att32(A) | ✅ Compaction + OutputViewport | **PASS** |
| **B** | att31 / att32(Segment) / c2050 | ✅ `FoldTarget::Segment` + hit + docs-only c2050 | **PASS** |

**Archive：** **可 archive**（CRITICAL=0）。

---

## 合约轴（Spec）

### CRITICAL

（无）

### WARNING

1. `scrollback.rs` 体量债（同 c1760/c2040）— 非行为缺口。

### Covered requirements

| Req | Verdict | Evidence |
|---|---|---|
| **att29** | **PASS** | `harness_mouse_triangle_toggles_compaction_fold` |
| **att30** | **PASS** | `harness_mouse_hint_toggles_output_viewport` |
| **att31** | **PASS** | `harness_mouse_segment_marker_toggles_one_step`；`harness_l2_ignores_l1_override…`；`segment_toggle_one_step…`（L3→L2；非 nearest） |
| **att32** | **PASS** | Compaction / OutputViewport / Segment 均经同一 `FoldHitTable` + hit_priority |
| c2040 回归 | **PASS** | Tool / Thinking / Diff / Ask / drag latch harness 仍绿 |
| c2050 吸收 | **PASS** | `archive/2026-08-13-c2050-…`；`status: superseded` / `absorbed_by: c2045` |

### Out-of-scope（正确未做）

- c1760 段状态机本身（已归档）
- 复活 keyboard fold-leader
- Bash L1 三角；整行可点（Viewport 仅 hint）

---

## 标准轴（Standards）

### CRITICAL

（无）

### WARNING

1. 折叠 hit / summary 仍堆在 `scrollback.rs`（体量）。

### PASS notes

- 分层：改动落在 `app/tui`；未穿 `agent`↔`infra`
- 无第二 hit 管道；Segment toggle 走 `ActivityFoldState::toggle_one_step`
- 未复活 fold-leader

---

## Verdict

**PASS — 可 archive**（合约轴 / 标准轴均无 CRITICAL；Wave A+B + c2050 docs-only 闭合）。
