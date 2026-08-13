---
depends_on:
- c2080-add-append-only-subagent-tui
blocks: []
---

# 移除 append-only 应用面（回滚 c2080 接线）

> **一句话**：撤销 [`c2080`](../archive/2026-08-12-c2080-add-append-only-subagent-tui/proposal.md) 交付的 append-only 应用面全部接线——CLI `append-only` 动词、`SurfaceMode::AppendOnly`、`src/app/append_only/`、session 旁 spill 目录（atao5 专属）与配套 live 合约；**暂时不做双 TUI 维护**。
> **范围**：只回滚**产品行为**（c2080 新增）；不推翻 c2071 的 ath30 AO-only 主会话；不动 xylitol-tui 库 Inline 入口（c2070 交付，先于 c2080 存在）。
> **阶段**：本提案为规划壳（proposal + tasks）；下一步 `llman sdd change start`（Branch binding）→ Specs landing。

## Why

1. **实现与预期相差较多**：append-only 面作为产品，其形态（Inline 载体、闭集键位、无折叠、spill 交互）落地后与预期不符。
2. **维护成本不成比例**：双 TUI（主会话 AO TUI + append-only 旁路面）意味着两条面并行维护 chrome/键位/会话语义；战略上**暂时不做双 TUI 维护**，宁可先把 c2080 接线干净移除，留「多面消费 XyDriver」的验证结论在 c2080 归档里，日后需要时再整体重做。
3. **不伤主线**：c2071 固定主会话 AO-only 是独立战略，不受影响；库 Inline 入口保留（c2070 交付，ath30 原句允许 lab/demo 与其它产品面使用），本票只删除消费它的产品面。

## What Changes

1. **删产品面**：`src/app/append_only/` 整目录（host / bridge / model / keys / caps / spill / harness / arch_tests / AGENTS.md）及 `src/app/mod.rs` 注册。
2. **删 CLI 接线**：`append-only` 子命令与 `AppendOnlySurfaceArgs`；`SurfaceMode` 收敛回 `Tui` / `Print`；`resolve_surface_intent` / `select_surface_mode` / 组合根分发 / trust gate 中 append-only 分支及对应单测全部移除。
3. **删 spill seam**：`src/app/core/tool_spill.rs` 与 `app/core/mod.rs` 注册；`OutputAccumulator` 的进程级 spill dir 接线回退为 c2080 前行为（溢出写系统 tmp）。
4. **live 合约**：
   - 删除 live `llmanspec/specs/app-tui-append-only/`（atao1–7；无 .feature、无 BDD 绑定，随删即净）。
   - `app-tui-host` ath30 回退 c2080 的「主会话主语澄清」措辞（主语从「产品 TUI」= 主会话，无并列面后单主语），并删末句 AppendOnly 例外条款；`ath30-unit` 场景名回退。库 Inline 入口保留句（c2070 原句）不回退。
5. **文档**：`src/app/AGENTS.md` 删 append-only 行。
6. **非目标**：
   - 不改 `packages/xylitol-tui`（Inline / ApplicationOwned 双入口与 Inline 默认不动）。
   - 不改 c2071 ath30 AO-only 战略、不重开主会话双模式。
   - 不删 `infra/tools/accumulator` 本身及其既有 `[Full output: …]` / `full_output_path` 族（c2080 前已有，硬截断溢出继续写系统 tmp）。
   - 不清理 c2080 归档文档（设计史保留）。

## Capabilities

- **删**：`app-tui-append-only`（整个 live spec）
- **触**：`app-tui-host`（ath30 主语/末句回退，AO-only 不变）
- 不动：`cli-entry`（ce16「表面至少含 tui 与 print」从 c2080 前就是该措辞，无 AppendOnly 引用）、`package-tui-interaction-modes` 等库层合约

## Impact

| 层 | 影响 |
|---|---|
| `src/app` | 删 `append_only/` + `tool_spill` seam + `SurfaceMode::AppendOnly`；组合根回 c2080 前分发 |
| `src/infra` | `OutputAccumulator` 进程级 spill dir 接线回退（保留 accumulator 与 Full output 族） |
| `packages/xylitol-tui` | 零改（Inline 入口保留，供 lab/demo） |
| live specs | 删 `app-tui-append-only`；ath30 措辞回退 |
| 验证 | CLI 单测（tui/print 分发、顶层 help 不再列 `append-only`）+ accumulator 溢出回退单测 + BDD（`resolve_surface_intent` 调用点返 3 元组，与 ce16 surface 场景对齐） |

## 后续（本票不承诺）

- 需要旁路观察 / sub-agent 面时，重开独立 change 重新设计；届时再评估双 TUI 维护意愿与形态。
