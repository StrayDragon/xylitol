# xylitol 调研笔记（短索引）

> **非规范。** 稳定边界：根/`src` `AGENTS.md`。高维图：`docs/architecture/`。
> 更新日期：2026-07-11。交接板：`_HANDOFF.md`。轨 P Prompt：`_prompts/track-p-tui-polish.md`。

## 已落盘

| 主题 | 去哪 |
|---|---|
| 产品定调 / Xy* / 包装 / Trust·MCP / cancel·schemars | 根 + `src/AGENTS.md` |
| 架构 mermaid（产品/业务） | [`docs/architecture/`](docs/architecture/README.md) |
| 插话·续跑·中止（产品） | [`docs/architecture/queue-and-interrupt.md`](docs/architecture/queue-and-interrupt.md) |
| 队列运行时实现 | **c525** `design.md` |
| XyEvent 防宽表 | c520 design；产品摘要 [`docs/architecture/events.md`](docs/architecture/events.md) |
| TUI bridge P0（原 readiness） | [`c465/.../design.md`](llmanspec/changes/c465-add-app-tui-bridge/design.md) |

## Track A changes

| ID | 主题 |
|---|---|
| c500 | 精选 pub use + 删死包装 |
| c505 | Provider 单路径 |
| c510 | domain 去 JsonSchema（depends c500） |
| c515 | MCP 配置化 + 重载 |
| c520 | XyEvent 闭集 |
| c525 | 异步可并发队列运行时 |

建议：`c520` ∥ `c500` → `c510`；`c505` ∥ `c515`；**c525 与 c465 开闸前 QueueUpdate 修复强相关**（可先于 TUI apply）。

## 已清理的文档

- 删除：`docs/testing-strategy.md`、`docs/tui-research/*`（对标长文与旧 readiness；**保留** `docs/assets/logo.svg`）
- 测试分层要点已并入根 `AGENTS.md`「提交与测试」
