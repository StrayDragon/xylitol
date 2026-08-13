# Tasks: c2081-remove-append-only-surface

> **前置**：[`c2080`](../archive/2026-08-12-c2080-add-append-only-subagent-tui/) 已归档（实现已合 main）。
> **本阶段**：Propose（规划壳）；下一步 Branch binding → Specs landing。

## 进度总览

| 块 | 状态 | 合约 |
|---|---|---|
| 0 规划壳 Designed | ✅ | proposal + tasks |
| 1 Branch binding | ✅ | `sdd/c2081-…` attached |
| 2 Specs landing | ✅ | 删 `app-tui-append-only`；ath30 回退 |
| 3–4 实现回滚 | ✅ | Apply backlog |
| 5 validate / verify | ⬜ | apply 门禁绿；建议 `llman-sdd-verify` |

---

## 0. Designed — ✅

- [x] 0.1 确认 c2080 实现 commit `12c7083a` 为 main 祖先；范围 = 该 commit 新增的产品行为
- [x] 0.2 写 proposal.md + tasks.md（回滚范围 / 非目标已钉）
- [x] 0.3 库 Inline 入口（c2070）不动、ath30 AO-only（c2071）不回退 —— 已钉

---

## 1. Branch binding — ⬜

- [x] 1.1 `llman sdd change start c2081-remove-append-only-surface`（干净树 + 默认分支）
- [x] 1.2 确认 attached / stage full

## 2. Specs landing — ⬜

- [x] 2.1 删 live `llmanspec/specs/app-tui-append-only/`（atao1–7；无 .feature / BDD 绑定，随删即净）
- [x] 2.2 `app-tui-host` ath30 回退：主语从「主会话产品 TUI」回「产品 TUI」；删末句 AppendOnly 例外；`ath30-unit` 场景名回退；库 Inline 保留句（c2070 原句）不动（已按 `4b8b736e^` 精确回退验证）
- [x] 2.3 `llman sdd validate` 相关 live specs `--strict --no-check`；commit Specs landing → `readyToImplement=true`

## Apply backlog（`llman-sdd-apply`）

### 3. 删产品面 + CLI 接线

- [x] 3.1 删 `src/app/append_only/`（host/bridge/model/keys/caps/spill/harness/arch_tests/AGENTS.md）与 `src/app/mod.rs` 注册
- [x] 3.2 CLI：删 `append-only` 子命令、`AppendOnlySurfaceArgs`；`SurfaceMode` 收敛为 `Tui` / `Print`；`resolve_surface_intent` 返 3 元组；`select_surface_mode`、组合根分发、trust gate 回 c2080 前
- [x] 3.3 删 append-only 相关单测（`atao1_*` / `parses_cli_command_append_only`）；`help_lists_surface_and_ops_commands` 不再断言 `append-only`
- [x] 3.4 `src/app/AGENTS.md` 删 append-only 行；清理 c2080 注释引用

### 4. 回退 spill seam

- [x] 4.1 删 `src/app/core/tool_spill.rs` 与 `app/core/mod.rs` 注册
- [x] 4.2 `OutputAccumulator`：删 `set_process_spill_dir` / `process_spill_dir` 与静态槽，溢出回系统 tmp；相关单测回退（含「Path may be system tmp or session spill dir」断言）

### 5. 校验

- [x] 5.1 `cargo test`（app/cli、infra/tools/accumulator 相关）+ `tests/bdd/steps_cli_tokenizer.rs`（`resolve_surface_intent` 3 元组）
- [x] 5.2 `llman sdd validate c2081-… --strict`（可带 `--check`）；verify → archive

## 实现顺序

```text
0 Designed ✅
  → 1 Branch binding
  → 2 Specs landing → readyToImplement
  → 3 删产品面 + CLI 接线
  → 4 回退 spill seam
  → 5 校验 → 建议 llman-sdd-verify → archive
```

## 回滚审计要点（apply 时核对）

- `git diff 12c7083a^ 12c7083a --stat` = c2080 全量清单；本票实现 diff 应对其逐项反向（唯 `infra/tools/accumulator` 只反向 spill-dir 部分，保留 c2080 未触碰的 accumulator 本体）。
- 库 Inline 入口（`InteractionMode` / `TUI` 默认 Inline）为 c2070 交付，不得出现在本票 diff。
- ath30 回退参照 commit `4b8b736e^`（c2080 Specs landing 前文本）。
