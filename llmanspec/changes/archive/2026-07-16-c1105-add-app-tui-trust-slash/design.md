# Design — c1105-add-app-tui-trust-slash

## 语义钉死（相对 pi）

| 项 | 决定 |
|----|------|
| 写盘 | 是 — `TrustManager::apply_updates` |
| 本会话立即重载项目资源 | **否** — MUST NOT 调用 `reload_runtime` / 等价 |
| 用户如何生效 | 系统块提示 `/reload` 或重启 |

## Slash 形状

```text
/trust                  → Trust 当前 cwd（与 gate「Trust」选项同更新集）
/trust self|this_dir    → 同上（补全冗余默认项，便于空格后 Enter/Tab）
/trust parent           → Trust parent folder（有父路径时；否则 usage/错误提示）
/trust deny             → Do not trust 当前 cwd
/trust <other>          → usage 错误
```

补全：`SlashArgCompletionSource("trust")` 在 `/trust ` 列出 `self`（默认首位）/ `parent` / `deny`，对齐 `/model <id>`；裸 `/trust` 仍走 slash 命令列表。

busy：`agent busy — /trust refused`；MUST NOT 写盘。

## 缝

```text
PendingSlash::Trust { mode }
  → Driver::persist_project_trust(mode)
       → TrustManager（仅 InProcessDriver / app-core）
  → 系统块：saved_path + trusted/denied + 「run /reload or restart to apply」
```

TUI MUST NOT `use crate::infra::trust`。

## 与启动 gate

启动 `trust_gate` / ChoicePrompt **不变**。本 slash 是会话内补写；已信任时再次 `/trust` 可覆盖写盘并同样提示 reload。
