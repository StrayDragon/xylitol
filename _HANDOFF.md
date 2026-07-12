# _HANDOFF — 交接板 + 短索引（非规范）

> 最后更新：2026-07-12（agent_demo compact/retry 预览；overlay 非产品主控件；c576 已归档）
> 分支语境：`feat/tui-dev`（相对 `origin/feat/tui-dev` 超前）
> **临时交接 / 进度指针，不是 SSOT。** 稳定边界：各层 `AGENTS.md`、`docs/architecture/`、`llmanspec/`。

---

## 〇、去哪读（短索引）

| 主题 | 去哪 |
|---|---|
| 产品定调 / Trust·MCP / Provider 范围 | 根 + `src/AGENTS.md` |
| **全部产品架构图（唯一入口）** | [`docs/architecture/README.md`](docs/architecture/README.md) |
| TUI bridge / layout 壳 | archive **c465** … **c493**；合约 `app-tui-*`（layout 壳 id 仍为 `app-tui-chrome`） |
| 包 Overlay 引擎（D08） | archive **c575**；**产品默认不用** capturing overlay |
| demo Compacting/Retry | `just demo-tui` · Alt+K / Alt+Y |
| 日常满闸 | **`just qa`** |

### 预览

```bash
just demo-tui
# Alt+K → Compacting；Alt+Y → Retry 1/3
# Ctrl+P → compact-status / retry-status
```

---

## 一、现状

| 轨 | 状态 |
|---|---|
| **B · 产品 TUI** | 至 **c493** 已归档；c491 stub 冻结 |
| **P · 包** | 至 **c576** 已归档（demo compact/retry；overlay 非产品主控件） |

**活跃 change**：见 `llman sdd list`（归档后应为空）。

### 下一焦点

1. 产品 follow-up：`/compact`、真 emit AutoRetry，或活树。
2. 交互继续优先 **editor 槽**（树 / Ask / 板），勿扩 capturing overlay UX。
