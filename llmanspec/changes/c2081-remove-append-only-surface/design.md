# Design: c2081-remove-append-only-surface

## 目标边界

| 在范围 | 不在范围 |
|---|---|
| 删 `src/app/append_only/` 整目录与 `app/mod.rs` 注册 | 改 `packages/xylitol-tui`（Inline/AO 双入口、Inline 默认均不动） |
| 删 CLI `append-only` 动词、`AppendOnlySurfaceArgs`、`SurfaceMode::AppendOnly` | 改 c2071 ath30 AO-only 战略；重开主会话双模式 |
| 删 `app/core/tool_spill` seam；`OutputAccumulator` spill-dir 接线回退系统 tmp | 删 accumulator 本体或其既有 `[Full output: …]` / `full_output_path` 族 |
| 删 live `app-tui-append-only`（atao1–7）；ath30 措辞回退 | 动 `cli-entry` ce16（c2080 前即「至少 tui 与 print」） |
| `src/app/AGENTS.md` 删 append-only 行 | 清理 c2080 归档文档（设计史保留） |

## 回滚判定（逐层）

| 层 | c2080 交付 | 回滚决策 |
|---|---|---|
| `src/app/append_only/` | 新产品面（~1.6k 行 + 7 文件 + AGENTS） | **整目录删除** |
| `src/app/cli` | `append-only` 动词 + `SurfaceMode::AppendOnly` + 分发分支 | **收敛回 `Tui` / `Print`**；`resolve_surface_intent` 返 3 元组 |
| `src/app/core/tool_spill` | 进程级 spill root seam（atao5 专属） | **删除**（仅 append-only 引用） |
| `src/infra/tools/accumulator` | `set_process_spill_dir` / `process_spill_dir` 静态槽 + 溢出路径改判 | **回退该接线**；溢出回系统 tmp（c2080 前行为） |
| `llmanspec/specs/app-tui-append-only` | atao1–7 live spec | **删目录**（无 .feature、无 BDD 绑定，随删即净） |
| `app-tui-host` ath30 | 「主会话」主语澄清 + 末句 AppendOnly 例外 | **回退措辞**（参照 `4b8b736e^`）；库 Inline 保留句为 c2070 原句，不动 |
| `tests/bdd/steps_cli_tokenizer.rs` | `resolve_surface_intent` 4 元组适配 | **回 3 元组**（ce16 surface 场景仍绿） |
| `packages/xylitol-tui` | 无（只消费 Inline） | **零改**；Inline 入口由 c2070 交付，先于 c2080 存在 |

## 关键边界

1. **ath30 回退 ≠ 回退 c2071**：c2071 的 AO-only 默认在 `a41f17db` 已落地；c2080 仅在 4b8b736e 做主语/例外条款澄清。回退只撤销澄清（回到「产品 TUI」单主语），不撤销 AO-only。
2. **库 Inline 保留**：c2070 交付双入口与 Inline 默认，ath30 原句明确「库仍可暴露 Inline 构造入口供 lab/demo」。本票删除唯一产品消费方后，Inline 成为库层遗留入口——与 c2080 前状态一致。
3. **Full output 族保留**：`[Full output: …]` footer 与 `full_output_path` 在 c2080 前已存在（`256f74f7`），硬截断溢出回系统 tmp 是既有行为，不因删面消失。
4. **回滚审计**：实现 diff 对照 `12c7083a`（c2080 实现 commit）逐项反向；唯 accumulator 只反向 spill-dir 部分。
