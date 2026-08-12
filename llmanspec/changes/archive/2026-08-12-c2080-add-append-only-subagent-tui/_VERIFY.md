# Verify — c2080-add-append-only-subagent-tui

**Date:** 2026-08-12
**Branch:** `sdd/c2080-add-append-only-subagent-tui`
**Worktree:** `/home/l8ng/Projects/__straydragon__/xylitol.sdd-c2080-add-append-only-subagent-tui`
**base_sha (proposal):** `839990764660a6b9f9014d1a88ee2d35aafeccd6`
**HEAD (pre-verify docs):** `9df0c9b0`
**Stage:** `full` · `readyToImplement=true` · `specsLanded=true` · attached
**CARGO_TARGET_DIR:** per-worktree via `just cargo-wt-env`

## Gates

| Gate | Result |
|---|---|
| Branch binding | `sdd/c2080-add-append-only-subagent-tui` |
| `llman sdd show … --output json --type change` | `readyToImplement=true` · `specsLanded=true` · `attached=true` |
| `llman sdd validate c2080… --strict --no-check` | PASS（INFO: depends_on archived c2071） |
| `llman sdd validate app-tui-append-only --strict --no-check` | PASS；`dualWriteCount=0`；7 unit scenarios |
| `llman sdd validate app-tui-host --strict --no-check` | PASS；`dualWriteCount=0` |
| `cargo test -p xylitol --lib append_only` | **17/17 PASS** |
| ath30 main-session unit | `interaction_mode_defaults_to_application_owned` PASS |
| dual-write / `.feature.delta` | 无；atao* 均为 `feature: false` unit |

### Focused append_only coverage (17)

| Area | Tests |
|---|---|
| atao1 entry | `atao1_explicit_append_only_does_not_steal_tty_default` · `parses_cli_command_append_only` |
| atao2 Inline | `atao2_session_binds_inline` |
| atao3 no fold | `atao3_no_expandable_blocks_api` · `blocks_never_expandable` · `no_fold_flags_on_tool_end` |
| atao4 caps 3/5 | `atao4_caps_strictly_below_mainline` · `write_family_uses_taller_cap` · streaming commit |
| atao5 spill | `session_spill_is_beside_sessions_dir` · `cap_with_spill_writes_and_annotates` · process spill roundtrip |
| atao6 keys | `atao6_closed_set` · `exit_slash_exact` |
| atao7 seam | `atao7_reuses_xydriver_seam_in_host` · `atao7_no_infra_or_capabilities_reach_in` · `atao7_scripted_stream_tool_spill_abort_exit` |

---

## 合约轴（Spec）

审查：live `app-tui-append-only` **atao1–atao7** + `app-tui-host` **ath30** 主语澄清；diff `base_sha...HEAD`（attach + specs + `SurfaceMode::AppendOnly` 实现）。

### CRITICAL

（无）

### Covered requirements

| Req | Verdict | Evidence |
|---|---|---|
| **atao1** | PASS | `SurfaceMode::AppendOnly` + CLI `append-only`；TTY 默认仍 AO TUI（单测钉不抢占） |
| **atao2** | PASS | `AppendOnlySession::new` → `InteractionMode::Inline`；产品文案称 append-only surface |
| **atao3** | PASS | `expandable` 恒 false；Ctrl+O / Alt+E 在 listener 吞掉；无 fold API |
| **atao4** | PASS | 常量 3 / 5，严格小于主线 5 / 10；溢出带 `[Full output: …]` |
| **atao5** | PASS | `{sessions_dir}/{sid}.spill/`；`full_output_path` + tool_spill process root |
| **atao6** | PASS | 闭集：Busy Esc/Ctrl+C abort、`/exit`、空闲双 Ctrl+C；禁 fold/slash/plate |
| **atao7** | PASS | `driver.run` / `abort` + bootstrap 组合根；arch_tests 禁 `infra`/`capabilities` reach-in；ScriptedDriver harness |
| **ath30** | PASS | 主语澄清为**主会话**；主会话仍 AO 默认；AppendOnly 为并列面，非削弱例外 |

### Out-of-scope（正确未做）

- Sub-Agent 编排 M1 / 多 runtime 物化（roadmap 另票）
- AO 内嵌套 append-only 面板
- 专用 spill 打开快捷键 / 完整 slash 目录

---

## 标准轴（Standards）

权威：`AGENTS.md` / `src/AGENTS.md` / `src/app/append_only/AGENTS.md` / write-surface（独立面 + XyDriver）。

### CRITICAL

（无）

### WARNING

1. **`src/app/cli/mod.rs` ~963 行** — AppendOnly 分发堆在组合根；后续可抽 surface bootstrap，不挡本票 archive。
2. **双 Ctrl+C 武装逻辑双写** — `add_input_listener` 与 `handle_crossterm` 各有一份 idle arm；行为一致但维护面双份（Duplicated Code 提示）。
3. **`keys::is_allowed_product_action` 未作运行时闸** — 闭集以单测 + host 硬编码和弦为主；文档表与接线偶有漂移风险。
4. **feat / specs commits 含 `Co-authored-by: Cursor`** — 人决定是否改写后再合默认分支。

### SUGGESTION

1. proposal 顶部「阶段：…未写应用代码」文案已过时，archive 前可擦成 Apply 完成态。
2. `host.rs`（457）与 `harness.rs`（336）体量健康；继续保持面内禁 reach-in，组合根才碰 `infra::session`。
3. 人验：真 TTY 跑 `xylitol append-only`（流式 / 工具 spill / Busy Esc / `/exit`）未强制；自动化已覆盖 Scripted 路径。

---

## Verdict

**合约轴：通过（CRITICAL=0）。标准轴：通过（CRITICAL=0）。**

**Archive recommended: YES**（可走 `llman-sdd-archive` / `change finalize`）。本报告不 inline finalize / 不 push。

下一步：`llman-sdd-archive`。
