# Verify — c2040-add-tui-mouse-click-fold-triangle

**Date:** 2026-08-12
**Branch:** `sdd/c2040-add-tui-mouse-click-fold-triangle`
**base_sha (proposal):** `49ce1de618c1ec340ff98a223d72978d1c4f428e`
**HEAD (this verify):** post-human-signoff harness lock-in (drag latch + Thinking mouse + package Down-only hit_priority)
**Stage:** `full` · `readyToImplement=true` · `specsLanded=true` · attached

## Gates

| Gate | Result |
|---|---|
| Branch binding | `sdd/c2040-…` |
| `llman sdd show … --json` | `readyToImplement=true` |
| `llman sdd validate c2040 --strict --no-check` | PASS（INFO: depends_on archived c2020/c2070） |
| `llman sdd validate app-tui-transcript/host --strict --no-check` | PASS；dualWrite=0 |
| `cargo test -p xylitol --lib app::tui` | **352 passed** / 2 ignored |
| Focused c2040 tests | 原 6 + `harness_mouse_drag_across_triangle…` + `harness_mouse_triangle_toggles_thinking…` + `hit_priority_runs_only_on_left_down_not_drag` |
| `just lint` | PASS |
| c2045 / c1760 / c2050 | **not** implemented（out of scope） |
| Human terminal sign-off | **PASS**（2026-08-12 人验） |

## Hard constraints (c2040)

| Constraint | Evidence |
|---|---|
| Glyphs `▸`/`▾`；ascii `>`/`v`；width=1 | `GlyphSet::{fold,unfold}` + `fold_glyphs_visible_width_one` |
| Triangle column only；body no toggle | `FoldHitTable` + `harness_mouse_triangle_toggles_tool_fold` |
| Down swallow via `set_transcript_hit_priority` | `HostSession::install_fold_triangle_hit_priority` |
| Tool/Diff/Ask/Thinking per-id overrides | `ScrollbackFold::{tools,thinking}_overrides` + tests |
| Alt+E / Ctrl+T clear family overrides | `slot_input.rs` clear_* + tests |
| ath25：单块 toggle 不重解析无关 Assistant | fingerprint 含 effective fold；`single_tool_toggle_does_not_miss_unrelated_assistant` |
| Streaming thinking stays expanded | `render_scrollback` streaming `"thinking"` 分支强制 unfold（att8/att21） |

---

## 合约轴（Spec）

审查对象：live `att19`–`att22`、`ath33`；实现对照 `widgets/{glyphs,fold_hit,scrollback}`、`layout/root/*`、`host/mod`、`bridge/{model,session_tree}`。

### CRITICAL

（无）

### WARNING

1. **`base_sha...HEAD` 含无关 `c2080` draft**（规划提交时 main 已 ahead）。不改 c2040 行为；finalize 前确认 ff 合入范围可接受，或 rebase/清理非本票文档。

### SUGGESTION

1. feat commit 含 `Co-authored-by: Cursor`（本地 hook）；仓库规则偏好不暴露 agent——合并前可改写 message（人决定）。
2. Thinking id = `hash(text)-ordinal`：同文多块可区分；跨 travel 与 live 同序同文可复现。若未来 session 有稳定 part id，可升为更强键（属 c2045/后续）。
3. （已补）Diff/Ask 三角鼠标 harness：`harness_mouse_triangle_toggles_{diff,ask}_fold`。

### Covered requirements

| Req | Verdict | Notes |
|---|---|---|
| **att19** fold-glyph-chevron | PASS | Unicode ▸/▾；ascii >/v；width 单测 |
| **att20** per-block-tools-fold-overrides | PASS | Tool/Diff/Ask effective+toggle；Alt+E 清 tools overrides；compaction 连带保留 |
| **att21** thinking-per-id-fold | PASS | id 路径 + Ctrl+T；**+** `harness_mouse_triangle_toggles_thinking_fold` |
| **att22** mouse-fold-triangle-column-only | PASS | 三角/正文 + **拖选划过三角** harness；库 `hit_priority_runs_only_on_left_down_not_drag` |
| **ath33** fold-triangle-hit-priority | PASS | AO begin 安装 hit_priority；命中 toggle+吞按；viewport sync；`take_fold_dirty`→AO stale |

### Out-of-scope（正确未做）

- c2045 FoldTarget 剩余块 / 吸收 c2050
- c1760 L2/L3 activity fold
- 整行可点；拆 Alt+E×compaction

---

## 标准轴（Standards）

### CRITICAL

（无）

### WARNING

1. **`scrollback.rs` 体量继续上涨**——后续 fold 族宜外提 hit/overrides（c2045）。
2. **`ScrollbackFoldDefaultsKey` 四元组**、`FoldTarget::{Tool,Diff,Ask}` 行为折叠到 `toggle_tools`——可读性/扩展债（非行为缺口）。

### SUGGESTION

1. Host 同步 viewport + hit_priority 闭环清晰，无 app→infra 越层。
2. `fold()` clone HashMap、hit emit 重复臂：c2045 前可顺手收。
3. 产品 `AGENTS.md` 鼠标段已改为指向 c2040（不再写「点折叠属后续」）。

---

## Dual-axis agents

- [Verify c2040 spec axis](2d4ad0f1-f250-438a-ab75-ad73567da2dc)：WARNING 验证洞已用本提交 harness 闭合。
- [Verify c2040 standards axis](dc50ef8e-f7dc-4f33-a889-9efd7c33b496)：无 CRITICAL；软顶/命名债记入上方 WARNING。

## Verdict

**PASS — 可归档**（无 CRITICAL；人验 + latch/Thinking 鼠标自动化已齐）。
建议：`llman-sdd-archive` / `change finalize`；留意 WARNING#1（c2080 合入范围）。
