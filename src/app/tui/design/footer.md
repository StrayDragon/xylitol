---
version: "alpha"
name: "footer"
description: "Single-line dim footer — cwd · model · thinking · optional used N/~N/? tokens · optional derived p%/window."
tokens_from: "../DESIGN.md"
components:
  footer:
    textColor: "{colors.muted}"
    height: "{spacing.footer-rows}"
---

# Footer

> Token 根源：`{colors.*}` / `{spacing.*}` → [`../DESIGN.md`](../DESIGN.md)。
> **c475 MVP**：`cwd · model`；**c1035**：带 provenance 的 `used N`/`~N`/`?`（无则省略）；**c1150**：thinking 标签（`thinking off` / `{as_str}`）；**c1680**：有 `context_window>0` 时追加派生 `p%/W`（仅展示）。字段间只用 ` · `，**不再**在 thinking 前加装饰 `•`（避免 `model · · low` 双分隔观感）。

## MUST

1. 恰好 **1 行** dim。字段序：`cwd · model · {thinking}`；有 `ContextTokenEstimate` 时追加 `· used … tokens`（见下表）；可选 `· branch`。
2. Thinking 标签：可调思考时 level=`off` → `thinking off`，其余用 xylitol 档名。**无可调思考的模型**（支持集空或仅不可调 off）：footer **MUST 省略** thinking 段（`cwd · model`），避免假装可调。**MUST NOT** 展示 provider map 值。与编辑器边框同步；等级经 `/model` picker（[`models-picker.md`](./models-picker.md)）。
2b. **NextTurn pending**（见 [`pending-runtime.md`](./pending-runtime.md)）：存在即将接替的模型/thinking 时，footer 的 model / thinking 字段 MUST 仍显示 **生效中（active）**；接替值 MUST NOT 覆写 footer 主字段，改由 **busy status 行右侧** dim 挂账表达。pending 清除后 footer 可与选中一致（通常已是 active）。
3. Token 文案按 `TokenProvenance`（经 `XyDriver::estimate_context_tokens`）：

   | Provenance | 文案 |
   |---|---|
   | Api / RemoteCount / LocalTokenizer | `used N tokens` |
   | Heuristic | `used ~N tokens` |
   | Unknown | `used ? tokens` |

   无估计结果（空会话 / estimate 失败）时 **MUST 省略** 该字段；**MUST NOT** 伪造 `used 0 tokens`。

3b. **派生占用比（c1680）**：当已展示 used 字段且当前模型 `context_window > 0` 时，MUST 追加 ` · {p}%/{W}`，其中 `p = tokens/window*100`（1 位小数），`W` 为紧凑 window（如 `128k`，对齐 pi `formatTokens`）。Heuristic MUST ` · ~p%/W`；Unknown MUST ` · ?%/W`。`context_window` 为 0 时 MUST NOT 追加。该百分比 **仅展示**，MUST NOT 作为 compaction 触发 SSOT。
4. 放不下截断右侧（优先保留 cwd 左端与 model），**MUST NOT** 增高。
5. 快捷键提示：默认**不**写进 footer（勿 `enter submit · double Esc…` 墙）；需要时 `/help` 或旁注括号和弦（见 [`keybindings.md`](./keybindings.md)）。
6. 队列摘要若展示：短前缀 `q:sN|fM ·` 可贴 footer 最左，仍保持单行。
7. 刷新时机：session tree travel 换叶、一轮 turn 结束（AgentEnd / stream close / TurnEnd）、**CompactionEnd**、turn 进行中有可用 Api usage 更新时（节流）、thinking cycle / 模型切换；**MUST NOT** 每个 TextDelta 全量 tokenizer.encode。
8. **异步**：footer token 估计 MUST 在后台完成（`spawn_blocking`），**MUST NOT** 阻塞输入 / Tick / 其它渲染；结果落地后再差分刷新 footer 行。
9. **MUST NOT** 常驻多行 debug / 快捷键墙；**MUST NOT** 把 Working 文案塞进 footer（那是 status）；**MUST NOT** 把 Heuristic 显示成无 `~` 的精确值。

颜色：`{colors.muted}`；高度：`{spacing.footer-rows}`。
