---
version: "alpha"
name: "skill-ref"
description: "Inline $skill highlight in user messages (A10); no skill blocks or system lines."
tokens_from: "../DESIGN.md"
components:
  skill-ref:
    textColor: "{colors.skill-ref}"
---

# Skill ref（`$name` 高亮）

> Token：`{colors.skill-ref}` → [`../DESIGN.md`](../DESIGN.md)。呈现合约：[`../PI_DELTAS.md`](../PI_DELTAS.md) **A10**。

## MUST

1. 用户消息正文中，已识别的 `$skill` token（含 `$`）MUST 用 `{colors.skill-ref}` 前景高亮；其余正文仍用 user-message 默认色。
2. **MUST NOT** 为每个 `$` 另开 skill 色块、系统消息行、footer/`/session`/`/status skills` 清单。
3. 多 `$` 同条消息：各自高亮，**一条**用户消息块。
4. 产品验收以 **SKILL.md 注入 / read** 为准（c1130）；本文件只钉呈现色。高亮可有 demo/组件测，**不是**注入门禁的替代。

## Demo（`agent_demo`）

Plate `completion-dollar` / 提交含 `$demo`：transcript 用户行高亮 token；注入用 stub 正文记录（供 harness 断言），**不**加系统「已加载」行。
