# xylitol 调研笔记（短索引）

> **非规范。** 稳定边界：根/`src` `AGENTS.md`。高维产品图：[`docs/architecture/`](docs/architecture/README.md)。
> 更新日期：2026-07-11。交接板：`_HANDOFF.md`。主线 Prompt：`_PROMPT.md`。

## 去哪读

| 主题 | 去哪 |
|---|---|
| 产品定调 / Trust·MCP / Provider 范围 | 根 + `src/AGENTS.md` |
| **全部产品架构图（唯一入口）** | [`docs/architecture/README.md`](docs/architecture/README.md) |
| 队列运行时实现 | archive **c525** `design.md` |
| XyEvent 防宽表实现 | archive **c520**；产品摘要见架构目录 |
| TUI bridge（轨 B P0） | [`c465/.../design.md`](llmanspec/changes/c465-add-app-tui-bridge/design.md) |

勿在本文件复制架构长文；只留指针。

## Track A（已归档）

| ID | 主题 |
|---|---|
| c500 · c505 · c510 | 精选导出 · Provider 单路径 · domain 去 JsonSchema |
| c515 · c520 · c525 | MCP 配置化 · XyEvent 闭集 · 异步队列 |
| c530 embed · c535 · c540 | 嵌入 API · Server→Driver · 线协议/Remote |
| c545 · c550 | MCP 配置 seam · Server `dispatch` |

路径：`llmanspec/changes/archive/2026-07-11-c5xx-*`（业务全名；勿与同号段包侧 archive 混淆）。

## Track P（已归档并合入）

包侧 c530–c570（Markdown / demo plate / Diff / Completion / Expandable / playground / Tree / ChoicePrompt / Palette）已归档；可选跟进 **c575** Overlay focus-restore（purpose-draft）。详见 `_HANDOFF.md`。

## 下一波（Track B · 已开闸）

| ID | 主题 | 状态 |
|---|---|---|
| **c465** | TUI bridge（XyEvent→UI + Driver 合流） | proposed；可 apply |
| c475 / c480 / c485 | chrome · input · 垂直切片 | purpose-draft |
| c470 | Codex TranscriptView | **paused**（不做） |
| c490 / c492 / c493 | trust · bash · compaction UI | purpose-draft（后置） |

产品矩阵：[`库与多客户端.md`](docs/architecture/库与多客户端.md)。

## 已清理的文档

- 删除：`docs/testing-strategy.md`、`docs/tui-research/*`（**保留** `docs/assets/logo.svg`）
- 测试分层要点已并入根 `AGENTS.md`「提交与测试」
