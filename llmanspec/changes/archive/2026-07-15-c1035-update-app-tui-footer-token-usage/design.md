# Design — c1035 footer token usage

## 文案映射（MUST）

| TokenProvenance | Footer 片段 |
|---|---|
| Api / RemoteCount / LocalTokenizer | `used N tokens`（N 为估计 tokens） |
| Heuristic | `used ~N tokens` |
| Unknown | `used ? tokens` |

无 `ContextTokenEstimate`（estimate 失败或尚未拉取）时：**省略** token 字段（保持 `cwd · model`），MUST NOT 写 `used 0 tokens`。

## 字段序

`[q:…] cwd · model · used … tokens`（queue 前缀仍可选；token 在 model 之后）。

截断：优先保留 cwd 左端与 model；token 字段可被右侧截断。

## 刷新时机

| 事件 | 动作 |
|---|---|
| travel 换叶 | 重新 estimate → 更新 UiModel footer |
| turn 结束（AgentEnd / idle） | 同上 |
| compact 成功 | 同上 |
| 空会话 / 新 session | 省略 token 或 `?`（无消息时倾向省略） |

生成中活估计：可选；若做，节流（例如 ≥200ms）且仅 Heuristic/`~` 或沿用上次 Api 锚点，禁止每 delta encode。

## 接线

```text
effects / host
  → Driver::estimate_context_tokens()
  → ContextTokenEstimate { tokens, provenance, … }
  → footer_token_label(provenance, tokens)
  → UiModel / format_footer_text
```

## 与 design/footer.md

本 change 落地后同步 `src/app/tui/design/footer.md` MUST 条目与上表一致。
