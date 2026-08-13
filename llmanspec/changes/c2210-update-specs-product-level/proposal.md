---
depends_on: []
---

# Specs 收敛为产品级合约

> **一句话**：live spec 只限制用户/下游可观察行为；代码组织、路径、行数、测试文件行号迁出 spec。
> **目的地**：重构改名/挪文件不必先改一堆 toon；新 req 无歧义才准入。

---

## Why

仓库政策已要求产品级优先（`llmanspec/AGENTS.md`），但 65 个 capability 里约 54 个 `spec.toon` 提到 `src/`。`layer-architecture` 钉目录树；`ath12` 钉 `host/mod.rs` 与复杂度脚本；`atc4` 把 `DESIGN.md` 写成视觉 SSOT；`adp*` 把 playground lint 写成产品合约；`tt02` 甚至引用测试文件行号。

结果：扩展形态或拆模块时，spec 变成第二套架构文档，且与 `src/AGENTS.md` 双写。Agent 按 MUST 路径实现 → 与「代码是真值源」冲突。另有大量 `feature: false` 空 GWT，阅读差。

## What Changes

- **分层尺子**（写入本 change，确认后可回写 `llmanspec/AGENTS.md`「spec 约束层级」加检查清单）：对外行为 → spec；分层方向/crate 边界可留；路径/类型/行数/迁移清单 → AGENTS 或删。
- **审计**：每个 req `keep | rewrite-product | move-to-AGENTS | delete`。优先 `layer-architecture`、`app-tui-host`、`app-tui-chrome` `atc4`、`app-tui-design-playground`、`package-tui-testing`。
- **精简**：合并过碎 capability；删除无观察者的「防复活」；单测覆盖的 req 不要假 GWT 墙。
- **歧义闸**：新 req 必须附「唯一解释」场景或明确非目标；有争议停在 proposal。

Live specs 的实际编辑 **须** Branch binding 后在非默认分支进行（本草案不改 `llmanspec/specs/**`）。

## Capabilities（意向）

本票主要改 **spec 文本与政策**，可能触及几乎所有 capability 的措辞，但 **不**改产品运行时行为。可 `skip_specs_landing` 若只动 AGENTS + 直接编辑政策；若改 live toon 则按「组织过期可直接编辑 spec」条款，仍建议单独分支以免默认分支脏。

## 非目标

- 关掉 BDD / 废除 llman SDD
- 本票实现 TUI kind 表（c2200）
- 把架构不变量从 `src/AGENTS.md` 删掉（那是迁入处）

## Further Notes

调研：[`research/spec-rigidity-and-product-level.md`](./research/spec-rigidity-and-product-level.md)
派工：[`research/PROMPT-spec-audit.md`](./research/PROMPT-spec-audit.md)
三线索引：[`../c2200-refactor-tui-kind-catalog-verify/research/TRACKS.md`](../c2200-refactor-tui-kind-catalog-verify/research/TRACKS.md)
