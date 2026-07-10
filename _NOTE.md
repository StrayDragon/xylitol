# xylitol 调研笔记（短索引）

> **非规范。** 稳定边界：根/`src` `AGENTS.md`。高维图：`docs/architecture/`。
> 更新日期：2026-07-11。交接板：`_HANDOFF.md`。轨 P Prompt：`_prompts/track-p-tui-polish.md`。

## 已落盘

| 主题 | 去哪 |
|---|---|
| 产品定调 / Xy* / 包装 / Trust·MCP / cancel·schemars | 根 + `src/AGENTS.md` |
| 架构 mermaid（产品/业务） | [`docs/architecture/`](docs/architecture/README.md) |
| **库嵌入 + 多 client**（矩阵 / 理想 vs 现状） | [`docs/architecture/library-and-clients.md`](docs/architecture/library-and-clients.md) |
| 插话·续跑·中止（产品） | [`docs/architecture/queue-and-interrupt.md`](docs/architecture/queue-and-interrupt.md) |
| 队列运行时实现 | archive **c525** `design.md` |
| XyEvent 防宽表 | archive c520；产品摘要 [`events.md`](docs/architecture/events.md) |
| TUI bridge P0（原 readiness） | [`c465/.../design.md`](llmanspec/changes/c465-add-app-tui-bridge/design.md) |

## Track A（已归档 2026-07-11）

| ID | 主题 | 状态 |
|---|---|---|
| c500 | 精选 pub use + 删死包装 | archived |
| c505 | Provider 单路径 | archived |
| c510 | domain 去 JsonSchema | archived |
| c515 | MCP 配置化 + 重载 | archived |
| c520 | XyEvent 闭集 | archived |
| c525 | 异步可并发队列运行时 | archived |
| **c530** | 公开嵌入 API（`xylitol::embed`） | archived |
| **c535** | Server 统一到 Driver | archived |
| **c540** | 线协议 / RemoteDriver 对齐 | archived |

路径：`llmanspec/changes/archive/2026-07-11-c5xx-*`；c530–c540 见对应 `archive/2026-07-11-c5xx-*` 目录。

## 下一波内核（库 + 多 client）

见 [`library-and-clients.md`](docs/architecture/library-and-clients.md)。A2（c530–c540）已归档。

## 下一波（已开闸 / 已提案）

| ID | 主题 | depends | 状态 |
|---|---|---|---|
| **c545** | MCP 配置去泄漏（embed 缝） | — | proposed |
| **c550** | Server 经 dispatch 执行命令 | c540 | proposed |
| **c465** | TUI bridge（Track B） | c460/c461 ✅ | proposed；**TUI 已开闸** |

建议 apply 顺序：**c545 → c550 → c465**（MCP/dispatch 小、可先稳缝；再进 TUI）。

轨 B 其余（c470+）仍为 draft/paused，按需长大。

## 已清理的文档

- 删除：`docs/testing-strategy.md`、`docs/tui-research/*`（**保留** `docs/assets/logo.svg`）
- 测试分层要点已并入根 `AGENTS.md`「提交与测试」
