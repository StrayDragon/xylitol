# Verify — c2071-update-app-tui-host-mode-b-only

**Date:** 2026-08-12
**Branch:** `sdd/c2071-update-app-tui-host-mode-b-only`
**base_sha (proposal):** `1dd5de3a53e28365099c2351abd0a767bbfab688`
**HEAD (this verify):** `28071747` + follow-up BDD notch fix (see Gates)
**Stage:** `full` · `readyToImplement=true` · `specsLanded=true` · attached

## Gates

| Gate | Result |
|---|---|
| Branch binding | `sdd/c2071-update-app-tui-host-mode-b-only` |
| `llman sdd show … --output json --type change` | `readyToImplement=true` |
| `llman sdd validate c2071… --strict` | PASS (INFO: depends_on archived c2070) |
| `llman sdd validate --specs --strict --no-check` | PASS 64/64 |
| `cargo test --test bdd` | PASS 276/276（含 `wheel-sticky-viewport`；step 已跟 `WHEEL_NOTCH=1`） |
| BDD `package_tui_interaction_modes` | PASS 6/6 |
| Product ath30 unit | `interaction_mode_defaults_to_application_owned` 等既有 harness |
| Click-after-wheel regression | `application_owned_click_stays_aligned_after_repeated_wheel_scrolls` PASS |
| fold / c2040 / c1760 | **not** implemented (out of scope) |
| Human terminal sign-off | Ghostty/tmux A0–A8 已 PASS；SSH `not test`；业务面 B1–B8 清单未填满 |

## Hard constraints (c2071)

| Constraint | Evidence |
|---|---|
| Product default ApplicationOwned (ath30) | `TuiRunOptions::default` / `new_product_ui*` → AO |
| No mid-session mode switch | No `apply_interaction_mode` |
| No product Inline escape (UX) | No CLI/env mode switch; Inline only explicit test/lab ctor |
| Dump on exit | Library dump-on default; product does not expose opt-out |
| Keep Driver/slash/key business semantics | Harness paths retained; AO viewport/mouse/dump deltas OK |
| Do NOT implement fold/c2040/c1760 | No product fold-click wiring |

---

## 合约轴（Spec）

审查：live `ath30` / `ath29` / `ath31` / `avs1` / `ptim10`；`main...HEAD` 含 AO 默认 + 后续 perf（超 proposal「库无影响」字面，但未改 ath30 语义）。

### CRITICAL

（无 — 已修）

- ~~BDD `wheel-sticky-viewport` 期望 `L11`（旧 ±3）~~ → step 改为 `L13`（`WHEEL_NOTCH=1`），276/276 绿。

### Covered requirements

| Req | Verdict | Notes |
|---|---|---|
| **ath30** | PASS | 启动绑定 AO；无热切；无 `XYLITOL_TUI_MOUSE` 模式开关 |
| **ath29** | PASS* | 映射/Moved/teardown 单测 + agent_demo/AO PTY ignore；产品专用 mouse PTY 仍偏薄（见 WARNING） |
| **ath31** | PASS | Copied chrome，非 Error toast |
| **avs1** | PASS | H1–H9 harness |
| **ptim10** | PASS | wheel sticky BDD 绿 |
| **ptim14** | PASS | 产品经库 seam 接入 |

### Out-of-scope

- fold click / c1760 / c2040：未实现（正确）。

---

## 标准轴（Standards）

权威：`AGENTS.md` / `src/app/tui/AGENTS.md`。

### CRITICAL

（无）

### WARNING

1. **`tui.rs` 体量超硬顶**（~2700 行）— AO reproject/wheel cadence 堆叠；后续拆分，不挡本票 archive。
2. **AO 绘制编排跨 host / mod / 库** — 能下沉的 wheel 合流仍可再收。
3. **`src/app/tui/PI_DELTAS.md` 未同步 D16** — package 侧已改；产品台账缺口。
4. **人验**：SSH 未测；清单 B1–B8 未填满（A 面 Ghostty/tmux 已 PASS）。
5. **超 proposal 库面 perf** — 正确性/体验债偿还；合入前知悉 diff 大于「仅默认翻转」。

### SUGGESTION

1. `HostSession::new` Inline 默认 rustdoc 再钉「非产品路径」。
2. `scripts/lab_ao_session_cpu.py` 去掉硬编码绝对路径。
3. wheel 手感另开实验 branch（勿复活 DECSTBM shift）。

---

## Verdict

**合约轴：通过（CRITICAL=0）。标准轴：通过（CRITICAL=0）。**

**Archive recommended: YES**（可 `llman-sdd-archive` / `change finalize`），合并默认分支前建议补齐人验 B 面 / SSH（WARNING #4）。

下一步：`llman-sdd-archive`（本报告不 inline finalize）。
