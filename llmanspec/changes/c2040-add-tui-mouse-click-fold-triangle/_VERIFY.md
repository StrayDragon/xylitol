# Verify — c2040-add-tui-mouse-click-fold-triangle

**Date:** 2026-08-12
**Branch:** `sdd/c2040-add-tui-mouse-click-fold-triangle`
**base_sha (proposal):** `49ce1de618c1ec340ff98a223d72978d1c4f428e`
**HEAD (this verify):** `ed49b867` (+ docs verify commit)
**Stage:** `full` · `readyToImplement=true` · `specsLanded=true` · attached

## Gates

| Gate | Result |
|---|---|
| Branch binding | `sdd/c2040-…` |
| `llman sdd show … --json` | `readyToImplement=true` |
| `llman sdd validate c2040 --strict --no-check` | PASS（INFO: depends_on archived c2020/c2070） |
| `llman sdd validate app-tui-transcript/host --strict --no-check` | PASS；dualWrite=0 |
| `cargo test -p xylitol --lib app::tui` | **352 passed** / 2 ignored |
| Focused c2040 tests | glyph / FoldHitTable / tools+Alt+E / thinking+Ctrl+T / paint miss / harness mouse triangle+non-triangle **6/6** |
| `just lint` | PASS |
| c2045 / c1760 / c2050 | **not** implemented（out of scope） |
| Human terminal sign-off | 见 `_HUMAN_CHECKLIST.md`（verify 不挡自动化；合并前建议最短路径） |

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
2. **拖选 latch 无专用产品测**：依赖库 `SelectionController` 仅在 Left Down 调 `hit_priority`（拖中为 Drag）。行为符合设计，但缺「拖选中划过三角不 toggle」显式 harness（spec 场景 att22-unit 写了；自动化未单测该路径）。

### SUGGESTION

1. feat commit 含 `Co-authored-by: Cursor`（本地 hook）；仓库规则偏好不暴露 agent——合并前可改写 message（人决定）。
2. Thinking id = `hash(text)-ordinal`：同文多块可区分；跨 travel 与 live 同序同文可复现。若未来 session 有稳定 part id，可升为更强键（属 c2045/后续）。

### Covered requirements

| Req | Verdict | Notes |
|---|---|---|
| **att19** fold-glyph-chevron | PASS | Unicode ▸/▾；ascii >/v；width 单测 |
| **att20** per-block-tools-fold-overrides | PASS | Tool/Diff/Ask effective+toggle；Alt+E 清 tools overrides；compaction 连带保留 |
| **att21** thinking-per-id-fold | PASS | `allocate_thinking_id` live+rebuild；Ctrl+T 清 thinking overrides；流式强制展开 |
| **att22** mouse-fold-triangle-column-only | PASS* | 三角命中/非三角不 toggle harness 绿；拖选 latch 见 WARNING |
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

1. **`scrollback.rs` 体量继续上涨**（本票 +268 行量级）——未拆文件；仍贴软顶策略，后续 fold 族宜外提 hit/overrides（c2045 机会）。

### SUGGESTION

1. `FoldTarget` / `diff_fold_key` / overrides 已是合理领域类型（非 Primitive Obsession）。
2. Host 同步 viewport + hit_priority 闭环清晰，无 app→infra 越层。
3. 可能的 Data Clump：`(scroll_top, transcript_rows)` 已收在 `FoldHitTable`——好。

---

## Verdict

**PASS — 可归档**（无 CRITICAL）。
建议：最短人验签字后 `llman-sdd-archive` / `change finalize`；留意 WARNING#1 合入范围。
