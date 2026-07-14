# Design — c716-update-app-tui-agents-layout

## 目标结构（对照 kimi `apps/kimi-code/AGENTS.md`）

| 节 | 写什么 | 不写 |
|---|---|---|
| TUI File Layout | 目录/关键文件一句话 | 行数、进度 |
| Module Responsibilities | host/mod 协调；effects/bridge/layout 下沉 | 实现教程 |
| 硬约束 | Esc 归属指针、bang vs agent abort、reach-in、size budget | 易腐待办 |
| 验证 | harness / bdd / e2e 命令指针 | 完整矩阵长文 |
| HOW | write-tui / PI_DELTAS / DESIGN | 重复根 AGENTS |

## 与现有 AGENTS 关系

- **保留**：现状表可压缩为指针；开闸史勿膨胀；验证表保留并链 c715 BDD。
- **新增**：布局地图 +「新逻辑先下沉」+ Esc/abort 分岔一行表（细节见 c715 `design.md`）。
- **根 AGENTS / src/AGENTS**：不改；本文件只补本目录专属。

## 风险

文档过长 → 违反「短、少变」；用表驱动，细节指 design/PI_DELTAS。
