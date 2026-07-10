---
change_id: c450-revise-app-tui-contract
title: "修订 app-tui 合约：废弃 ratatui 遗留，引入 app-tui-*，锁定第四次 TUI 边界"
status: proposed
priority: 450
depends_on: []
author: agent
track: B
---

# c450-revise-app-tui-contract

## Why

第四次重做产品 TUI 前，单体 `llmanspec/specs/app-tui`（39 req）仍混有已删除的 ratatui / `StyledLine` / `Viewport::Inline` / idle 常驻 Ready 等合约，与现行 `packages/xylitol-tui` + `src/app/tui/DESIGN.md` 严重冲突。若在腐烂合约上叠实现，会重蹈前三次失控。

本变更**只修订规范与命名规则**，不实现 `src/app/tui` 产品代码（`run()` 可仍返回占位错误，直到 c460）。

## What Changes

1. **退役冲突 req**：旧 `app-tui` 中绑定 ratatui、`StyledLine`、`Viewport::Inline`、always-on Ready、旧 markdown/TestBackend 实现史的 requirement **已从 main spec 直接移除**（保留 6 条跨切面不变量 + capability 索引）；本变更 delta 将保留项改为中文并对齐 Remote 预留 / `app-tui-*` 索引。引擎与五层 harness 真值在 `package-tui-*`（尤其 `package-tui-testing`），不在单体 `app-tui` 重复。
2. **引入 `app-tui-*` capabilities**（最小壳 + 关键 MUST，中文 purpose/statement）：
   - `app-tui-host` — 入口、host 循环、终端生命周期、日志
   - `app-tui-bridge` — XyEvent→UI、流生命周期、Remote 预留
   - `app-tui-transcript` — 消息/可展开/diff 呈现
   - `app-tui-chrome` — theme/glyph/status/footer
   - `app-tui-input` — editor、slash、steer/follow-up 键位
   - `app-tui-commands` — `/exit` `/model` 与 `protocol::Command` 映射
3. **更新 `llmanspec/config.yaml` rules**：强制 `app-tui-*` / `package-tui-*`；声明 specs 正文（purpose/statement）**强制中文**；记录其它域前缀意向（`core-*` / `infra-*` 等由并行清理任务落地）。
4. **锁定已决议产品边界**（写入 delta + design.md）：
   - 引擎：host 驱动 `xylitol-tui`；禁止产品路径 `TUI::start()`；禁止 ratatui
   - 布局：transcript → status(0|1) → editor(边框) → footer(1)；idle **不占** status
   - 信任后 **yolo**（无逐工具审批 UI）；保留 hook 扩展点
   - 键位：Esc=abort（流中）；Ctrl+C 清输入 / 空再退；流中 Enter=steer；Alt+Enter=follow-up
   - MVP slash：`/exit` + `/model`；可扩展
   - `tui` feature **并入 default**（本变更改合约；Cargo 落地可在 c460）
   - 默认 InProcess；**保留** RemoteDriver 类型供日后远控
5. **标明清理项**（本变更可只改规则/指针；大文件搬迁交给并行提示词任务）：旧 `diff-review` ratatui 审批、腐烂 BDD 挂靠、单体 `app-tui` 最终删除或缩成索引。

## Capabilities

- `app-tui`（modify / 收缩为索引或退役说明）
- `app-tui-host`（新建）
- `app-tui-bridge`（新建）
- `app-tui-transcript`（新建）
- `app-tui-chrome`（新建）
- `app-tui-input`（新建）
- `app-tui-commands`（新建）

## Impact

- `llmanspec/specs/**`、`llmanspec/config.yaml`
- 根 / `src/app/tui` / `packages/xylitol-tui` 的 AGENTS 命名指针（短更新）
- **不改** `src/app/tui/mod.rs` 实现（仍占位）
- **不改** `packages/xylitol-tui` 代码

## 不在范围

- App Shell 实现（c460+）
- Diff/Tree/InputListener 包实现（c451/c454/c455）
- DESIGN 文件拆分正文（c449）
- specs 全域重命名（`core-*`/`infra-*`）— 见 `_tmp_prompts/` 并行任务

## 验证

```bash
llman sdd validate c450-revise-app-tui-contract --strict --no-interactive
```
