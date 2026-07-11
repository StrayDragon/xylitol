# xylitol 调研笔记（短索引）

> **非规范。** 稳定边界：根/`src` `AGENTS.md`。高维产品图：[`docs/architecture/`](docs/architecture/README.md)。
> 更新日期：2026-07-11。交接板：`_HANDOFF.md`。

## 去哪读

| 主题 | 去哪 |
|---|---|
| 产品定调 / Trust·MCP / Provider 范围 | 根 + `src/AGENTS.md` |
| **全部产品架构图（唯一入口）** | [`docs/architecture/README.md`](docs/architecture/README.md) |
| 队列运行时实现 | archive **c525** `design.md` |
| XyEvent 防宽表实现 | archive **c520**；产品摘要见架构目录 |
| TUI bridge P0 | [`c465/.../design.md`](llmanspec/changes/c465-add-app-tui-bridge/design.md) |

勿在本文件复制架构长文；只留指针。

## Track A（已归档 2026-07-11）

| ID | 主题 | 状态 |
|---|---|---|
| c500 | 精选 pub use + 删死包装 | archived |
| c505 | Provider 单路径 | archived |
| c510 | domain 去 JsonSchema | archived |
| c515 | MCP 配置化 + 重载 | archived |
| c520 | XyEvent 闭集 | archived |
| c525 | 异步可并发队列运行时 | archived |
| **c530** | 公开嵌入 API | archived |
| **c535** | Server 统一到 Driver | archived |
| **c540** | 线协议 / RemoteDriver 对齐 | archived |
| **c545** | MCP 配置去泄漏 | archived |
| **c550** | Server 经 dispatch 执行命令 | archived |

路径：`llmanspec/changes/archive/2026-07-11-c5xx-*`。

## 下一波

| ID | 主题 | 状态 |
|---|---|---|
| **c465** | TUI bridge（Track B） | proposed；**本波不处理 4xx** |

产品矩阵与缺口见 [`库与多客户端.md`](docs/architecture/库与多客户端.md)。A3 内核缝已归档；4xx / Track B 搁置。

## 已清理的文档

- 删除：`docs/testing-strategy.md`、`docs/tui-research/*`（**保留** `docs/assets/logo.svg`）
- 测试分层要点已并入根 `AGENTS.md`「提交与测试」
