---
version: "alpha"
name: "trust-prompt"
description: "Project trust ChoicePrompt before bootstrap (c490)."
tokens_from: "../DESIGN.md"
components:
  trust-body:
    textColor: "{colors.on-surface}"
  trust-warning:
    textColor: "{colors.warning}"
---

# Trust prompt

> Token 根源：`{colors.*}` → [`../DESIGN.md`](../DESIGN.md)。
> **c490**：产品 `--tui` / 默认 TUI 入口在 bootstrap **之前**用包 `ChoicePrompt`（`Palette::dark().choice_prompt_theme()`）完成 Ask；**MUST NOT** `prompt_trust_options_stdio`。

## MUST

1. 需要 Ask 时：raw-mode 内单题 Single ChoicePrompt，选项来自 `TrustManager::get_trust_options`。
2. Esc / Ctrl+C → **取消**：不写 `trust.json`，恢复终端，打印提示后退出（**不**进入产品 TUI）。
3. Enter 提交 **Trust / Trust parent** → 写 trust store，再 `bootstrap` 并进入产品 TUI。
4. Enter 提交 **Do not trust** → 写 deny 到 trust store，打印「denied in trust store」提示后退出（**不**进入产品 TUI）。
5. 信任后 yolo（无逐工具审批 UI）；hook 扩展点保留。
6. ChoicePrompt `prompt` 若含换行，MUST 按行渲染（禁止把 path / 说明 / `· 单选` 粘成一行导致错位）。

## 非目标

- 大 overlay 仪表盘（走全屏/槽内 ChoicePrompt，非命令面板）
- c575 overlay focus-restore
