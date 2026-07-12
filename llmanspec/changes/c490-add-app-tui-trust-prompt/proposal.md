---
change_id: c490-add-app-tui-trust-prompt
title: "app-tui：trust 用 ChoicePrompt 对齐 agent_demo（替换 stdio）"
status: ready
priority: 490
depends_on: ["c460-add-app-tui-host", "c475-add-app-tui-chrome", "c565-add-package-tui-choice-prompt"]
author: agent
track: B
---

# c490-add-app-tui-trust-prompt

## Why

产品 `--tui` 路径仍在 bootstrap 用 `prompt_trust_options_stdio`（`Choice [1-3]`）问信任，**进 raw mode 之前**就是 CLI 问答，与 `agent_demo` 的 **ChoicePrompt 替换 editor 槽**体感严重不一致——用户第一印象就是「不是 demo 那个 TUI」。

## What Changes

1. **交互 trust 推迟到产品 TUI host 内**：需要 Ask 时，用包 `ChoicePrompt`（`Palette::dark().choice_prompt_theme()`）替换 editor 槽；写入 trust store 后继续资源加载 / 会话。
2. **stdio 提示降级**：仅非 TTY / 无 TUI / 测试回调保留；产品 `--tui` MUST NOT 再走 stderr 数字菜单。
3. Bootstrap：`interactive` 仍表示「可提示」，但 TUI 路径的 `on_prompt` 改为 host 可完成的异步/通道缝（或等价：bootstrap 返回 pending-trust，host 决完再 `reload` 项目资源）——见 design。
4. 信任后仍 **yolo**（无逐工具审批）；hook 扩展点保留。

## Capabilities

- `app-tui-trust`（新建）

## Impact

- `src/app/core/bootstrap.rs`、`src/app/tui/{host,ui_root,mod}.rs`、`src/infra/trust/prompt.rs`（保留 stdio 给非 TUI）
- `design/trust-prompt.md` 从草稿升为 SSOT 指针

## Out of scope

- 多 overlay focus-restore（c575）
- Ask 工具 / 多题 ChoicePrompt 产品接线（demo plate 已有）
- c476 live scrollback 富渲染（并行提案）
- `--trust` / `--no-trust` 覆盖语义变更

## Depends note

- 包 `ChoicePrompt`：已归档 **c565**
- chrome / Palette：已归档 **c475**
