# Design: c1670-add-session-compact-optional-instructions

## 对照 pi（一手）

| pi | xylitol 现状 | 本 change |
|---|---|---|
| `compact(customInstructions?)` | `force_compact()` / `Command::Compact { id }` 无字段 | ✅ 可选 `instructions` |
| slash `/compact <text>` | `/session-compact` 带参 → usage 错误（A05） | ✅ 带参成功；短名仍无效（A03） |
| `basePrompt += "\n\nAdditional focus: …"` | `generate_summary` 无注入点 | ✅ 追加，不替换骨架 |
| instructions **仅** history `generateSummaryWithUsage` | N/A | ✅；`generateTurnPrefixSummary` **不**带 |
| auto compact 不传 instructions | threshold/overflow 无参 | ✅ 保持 |

## 数据流

```text
/session-compact              → PendingSlash::Compact { instructions: None }
/session-compact   <ws>       → 同上（trim 后空 → None；与现 parse 过滤一致）
/session-compact focus on X   → Compact { instructions: Some("focus on X") }

Command::Compact { id, instructions }
  → dispatch → XyDriver::compact(instructions)
  → AgentSession::force_compact(instructions)
  → CompactionOrchestrator::compact(..., instructions)
  → compact_session(..., instructions)
  → generate_summary(..., instructions)   # history / 非 split
  → generate_turn_prefix_summary(...)     # 永不收 instructions
```

Auto（`maybe_auto_compact` / `maybe_overflow_compact`）调用 `compact_session` 时 **固定** `instructions: None`。

## Wire / Driver

- `Command::Compact { id: Option<String>, #[serde(default)] instructions: Option<String> }`
- `XyDriver::compact(&mut self, instructions: Option<String>)`（同 crate 内一并改 in_process / remote / mock / ScriptedDriver）
- REST / remote：缺字段 = None；有字段则透传（不做版本协商）

## Prompt 拼法（锁定）

与 pi 同构：在选定 `SUMMARIZATION_PROMPT` / `UPDATE_SUMMARIZATION_PROMPT` 之后：

```text
{base_prompt}\n\nAdditional focus: {customInstructions}
```

再写入既有 `<conversation>` / `<previous-summary>` 装配（xylitol 现有顺序可保持：conversation + previous + prompt；**Additional focus 挂在 prompt 常量侧**，与 pi「改 basePrompt」等价）。

## PendingSlash

`PendingSlash::Compact { instructions: Option<String> }`（或等价携带）；busy Allow 列表仍认 Compact。

## PI_DELTAS A05

| 改前 | 改后 |
|---|---|
| 差异：仅无参；带参 usage | **已撤销**：可选 instructions 对齐 pi；命令名仍 `session-compact`（A03） |
| 不得回退 = 是 | 不得回退 = **否**（或移出差异表进「对齐备忘」） |

变更记录加一行日期说明。

## 非目标

| 禁止 | 说明 |
|---|---|
| `/compact` 短名 | A03 |
| `replaceInstructions` | extension 路径不做 |
| auto 带 instructions | 禁止 |
| TUI % | c1680 |
| 超长截断产品策略 | 可不做；日志勿全文 dump |

## 验收 seam

| 锚点 | 覆盖 |
|---|---|
| bare-force | 无参 → force；dispatch Compact instructions=None |
| with-text | 有参 → prompt 含 `Additional focus:` + 文本 |
| whitespace | 仅空格 → 等同无参 |
| auto-clean | threshold/overflow 路径模型输入无 Additional focus |
| a05-doc | PI_DELTAS A05 已更新 |
