---
version: "alpha"
name: "footer"
description: "Single-line dim footer — cwd · model · thinking · optional used N/~N/? tokens."
tokens_from: "../DESIGN.md"
components:
  footer:
    textColor: "{colors.muted}"
    height: "{spacing.footer-rows}"
---

# Footer

> Token 根源：`{colors.*}` / `{spacing.*}` → [`../DESIGN.md`](../DESIGN.md)。
> **c475 MVP**：`cwd · model`；**c1035**：带 provenance 的 `used N`/`~N`/`?`（无则省略）；**c1150**：thinking 标签（`thinking off` / `{as_str}`）。字段间只用 ` · `，**不再**在 thinking 前加装饰 `•`（避免 `model · · low` 双分隔观感）。

## MUST

1. 恰好 **1 行** dim。字段序：`cwd · model · {thinking}`；有 `ContextTokenEstimate` 时追加 `· used … tokens`（见下表）；可选 `· branch`。
2. Thinking 标签（**c1150**）：level=`off` → `thinking off`；其余用 `ThinkingLevel::as_str`（如 `medium`、`xhigh`）。形如 `~/x · model · thinking off` 或 `· low`。与编辑器 thinking 边框同步；切换经 XyDriver，**MUST NOT** 因 cycle 向 transcript 刷系统行。
3. Token 文案按 `TokenProvenance`（经 `XyDriver::estimate_context_tokens`）：

   | Provenance | 文案 |
   |---|---|
   | Api / RemoteCount / LocalTokenizer | `used N tokens` |
   | Heuristic | `used ~N tokens` |
   | Unknown | `used ? tokens` |

   无估计结果（空会话 / estimate 失败）时 **MUST 省略** 该字段；**MUST NOT** 伪造 `used 0 tokens`。
4. 放不下截断右侧（优先保留 cwd 左端与 model），**MUST NOT** 增高。
5. 快捷键提示：默认**不**写进 footer（勿 `enter submit · double Esc…` 墙）；需要时 `/help` 或旁注括号和弦（见 [`keybindings.md`](./keybindings.md)）。
6. 队列摘要若展示：短前缀 `q:sN|fM ·` 可贴 footer 最左，仍保持单行。
7. 刷新时机：session tree travel 换叶、一轮 turn 结束（AgentEnd / stream close）、compact 成功、thinking cycle / 模型切换；**MUST NOT** 每个 TextDelta 全量 tokenizer.encode。
8. **异步**：footer token 估计 MUST 在后台完成（`spawn_blocking`），**MUST NOT** 阻塞输入 / Tick / 其它渲染；结果落地后再差分刷新 footer 行。
9. **MUST NOT** 常驻多行 debug / 快捷键墙；**MUST NOT** 把 Working 文案塞进 footer（那是 status）；**MUST NOT** 把 Heuristic 显示成无 `~` 的精确值。

颜色：`{colors.muted}`；高度：`{spacing.footer-rows}`。
