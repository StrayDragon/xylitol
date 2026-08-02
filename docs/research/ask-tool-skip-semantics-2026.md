# Ask-tool skip / decline / cancel semantics（一手调研，2026-08）

> **范围**：结构化「问用户」工具在 skip / decline / cancel / timeout 时，模型收到什么。
> **约束**：仅一手来源（官方文档、带原文字符串的 issue/论坛、协议原文）。
> **非目标**：不改 `llmanspec/specs/**`；不做 xylitol 实现。

## 对比表

| 产品 | 故意 skip/decline 的载荷 | 成功还是错误？ | Esc → decline？ | timeout ≠ skip？ | 显式 Skip UI？ |
|------|--------------------------|----------------|-----------------|------------------|----------------|
| **Cursor `AskQuestion`** | 纯文本：`Questions skipped by the user, continue with the information you already have` | 工具成功结果（非 tool error） | 未找到一手 Esc 映射 | **否**（timeout/空结果与 deliberate skip 字节相同） | **是**（用户提到 Skip / Continue） |
| **Claude Code `AskUserQuestion`** | Decline：`User declined to answer questions`；Esc 取消：`The user doesn't want to proceed with this tool use.`；空成功：`User has answered your questions: . …` | Decline/Esc 多为拒绝/取消路径；空答案却是 **success** | **是**（Esc = reject tool use） | **部分**：AFK 有独立文案；但空 success bug 与 decline 混淆 | 选项里可有 “Skip”；无统一 Skip 控件文档 |
| **Codex `request_user_input`** | timeout：`{"answers":{}}`；cancel：`…was cancelled before receiving a response` | timeout = **成功** JSON；cancel = **RespondToModel 错误** | 未找到一手 Esc 映射 | **是**（空 answers vs cancel 错误串） | 无「Skip」控件证据；有 Snooze；提案 UI 有 Cancel |
| **MCP elicitation** | `{"action":"decline"}` / `{"action":"cancel"}` | JSON-RPC **`result`**（非 error） | Spec：**Escape → cancel**（非 decline） | Spec 三分：accept / decline / cancel（无 timeout 动作） | Clients **MUST** 提供 decline 与 cancel |

---

## 1. Cursor `AskQuestion`

**Deliberate skip 字符串**（用户与 Cursor 员工均引用同一句）：

> `Questions skipped by the user, continue with the information you already have`

来源：[forum #158485](https://forum.cursor.com/t/askquestion-tool-can-return-synthetic-skip-string-with-highly-variable-unpredictable-delay/158485)（含 Dean Rie 确认：空结果映射到该串）。

| 问题 | 证据结论 |
|------|----------|
| 成功还是错误？ | 作为 **tool return string** 回到模型（成功路径）；员工称 orphaned Promise「resolves with an empty result, which then maps to that exact string」。 |
| Esc？ | 本调研未找到一手 Esc→skip 映射。 |
| timeout ≠ skip？ | **否**。员工 Kevin：timeout「reported to the model as a skip」、与真实 skip「byte-identical」([#159806](https://forum.cursor.com/t/increase-the-timeout-for-ask-questions/159806))。后期宣称移除 auto-skip，但用户仍报同一 skip 串（[#163317](https://forum.cursor.com/t/asktool-timeout-causes-incorrect-behavior-and-leaves-the-chat-in-a-broken-state/163317)）。 |
| Skip UI？ | 有。OP expected：「didn't actually click the **Skip** button」([#158485](https://forum.cursor.com/t/askquestion-tool-can-return-synthetic-skip-string-with-highly-variable-unpredictable-delay/158485))；另有 Continue 提交。 |

---

## 2. Claude Code `AskUserQuestion`

**官方行为**：[Tools — AskUserQuestion](https://code.claude.com/docs/en/tools#askuserquestion-tool-behavior)

- 默认问题保持打开直至作答。
- 可选 `askUserQuestionTimeout`（`60s`/`5m`/`10m`）：到期后关闭对话框、提交已选选项，并告知 Claude「you may be away from your keyboard」，由模型自行判断、可稍后重问。

**运行时字符串（issue 原文）**：

| 情形 | 模型收到 | 来源 |
|------|----------|------|
| Decline / 非答案键误触 | `User declined to answer questions` | [#65392](https://github.com/anthropics/claude-code/issues/65392)（Ctrl-O）；[#69074](https://github.com/anthropics/claude-code/issues/69074)（「Chat about this」） |
| Esc | `The user doesn't want to proceed with this tool use.`（「user rejected tool use」） | [#62905](https://github.com/anthropics/claude-code/issues/62905)；同类 [#56890](https://github.com/anthropics/claude-code/issues/56890) |
| 未选却成功 | `User has answered your questions: . You can now continue with the user's answers in mind.` | [#30552](https://github.com/anthropics/claude-code/issues/30552) JSONL |
| AFK timeout | `No response after 60s - the user may be away from keyboard. Proceed using your best judgment…` + `answers: {}` | [#73408](https://github.com/anthropics/claude-code/issues/73408)；agent 复述 [#73891](https://github.com/anthropics/claude-code/issues/73891) |

| 问题 | 证据结论 |
|------|----------|
| 成功还是错误？ | Esc/拒绝 → **取消/拒绝工具**；AFK/空答案 → **成功 tool_result**（空 answers 危险）。 |
| Esc → decline？ | **是**，映射为 reject tool use（与 vim Esc 冲突）。 |
| timeout ≠ skip？ | AFK 有独立文案（相对 Cursor 更好）；但仍是 success + 鼓励 proceed。官方文档后补可配置 timeout。 |
| Skip UI？ | 问题选项可含 “Skip”（[#30552](https://github.com/anthropics/claude-code/issues/30552)）；无单独全局 Skip 控件文档。 |

Agent SDK：`canUseTool` deny 返回 `{ behavior: "deny", message }`，Claude 看到 deny message（[user-input](https://code.claude.com/docs/en/agent-sdk/user-input)）。

---

## 3. OpenAI Codex `request_user_input`（aka ask-style）

社区常称 `ask_user_question`；落地工具名为 **`request_user_input`**（[#10312](https://github.com/openai/codex/issues/10312) 维护者澄清）。提案 [#9926](https://github.com/openai/codex/issues/9926) 写明：Submit 返回结构化 answers；**Cancel aborts the tool call**。

**Timeout（成功空答案）**：

- PR [#28235](https://github.com/openai/codex/pull/28235)：`autoResolutionMs` 存在时，60s grace + 60s countdown，然后 **「submit an empty answer response」**。
- 会话 JSONL：`{"answers":{}}`（[#29702](https://github.com/openai/codex/issues/29702)）；Desktop 侧 `trackRequest(..., "empty-user-input")`。
- [#34455](https://github.com/openai/codex/issues/34455)：当前 on-timeout 硬编码 `empty_answer`；提议可选 `cancel`。

**Cancel（错误）** — 源码：

```text
request_user_input was cancelled before receiving a response
```

经由 `FunctionCallError::RespondToModel`（[request_user_input.rs](https://github.com/openai/codex/blob/main/codex-rs/core/src/tools/handlers/request_user_input.rs)）。

| 问题 | 证据结论 |
|------|----------|
| 成功还是错误？ | timeout → **成功** `{"answers":{}}`；cancel → **错误串**。 |
| Esc？ | 未找到一手。 |
| timeout ≠ skip？ | **是**（相对 Cursor）：空 JSON vs cancel 错误；但空 answers **不**标注 TIMED_OUT。 |
| Skip UI？ | 无 Skip 控件一手证据；有 Snooze；提案含 Cancel。 |

---

## 4. MCP elicitation `decline` / `cancel`

规范：[Elicitation (2025-11-25)](https://modelcontextprotocol.io/specification/2025-11-25/client/elicitation)

三分动作，均在 JSON-RPC **`result`** 内（非 `error`）：

```json
{ "jsonrpc": "2.0", "id": 1, "result": { "action": "accept" | "decline" | "cancel", "content": { … } } }
```

- **decline**：显式拒绝（Reject / Decline / No）；通常无 `content`。
- **cancel**：未做选择的关闭；示例含 **pressed Escape**、点外部、关对话框。
- Clients **MUST** 提供清晰 decline 与 cancel；servers **MUST** 处理 decline/cancel。

无协议级 timeout action；timeout 需产品层另标。

---

## Recommendation for xylitol

对 builtin `ask`（TUI clarify/decision）：**故意 skip 用结构化成功结果**，例如 `{ "status": "skipped", "answers": {} }`，**不要**用 tool-error；另用 `{ "status": "cancelled" }` / `{ "status": "timed_out" }` 区分 Esc/取消与超时。理由：Cursor 把 timeout 伪装成 skip 导致模型当「用户同意默认」继续，是安全事故；Claude 空 success（`answered: .`）同样伪造成功；Codex cancel 走错误、timeout 走空 answers 可区分但 provenance 仍弱；MCP 用 result 内 `action` 三分最清晰。tool-error 易被当成故障重试而非用户决策。TUI 应有显式 Skip；Esc→`cancelled` 勿与 skip 合并；默认阻塞、超时可选且必须带 `timed_out`。

---

## 来源索引

| ID | URL |
|----|-----|
| C1 | https://forum.cursor.com/t/askquestion-tool-can-return-synthetic-skip-string-with-highly-variable-unpredictable-delay/158485 |
| C2 | https://forum.cursor.com/t/increase-the-timeout-for-ask-questions/159806 |
| C3 | https://forum.cursor.com/t/asktool-timeout-causes-incorrect-behavior-and-leaves-the-chat-in-a-broken-state/163317 |
| C4 | https://forum.cursor.com/t/questionnaire-timeout-falsely-treated-as-skipped-multi-chat-rollback-affects-other-chats/160858 |
| A1 | https://code.claude.com/docs/en/tools#askuserquestion-tool-behavior |
| A2 | https://github.com/anthropics/claude-code/issues/30552 |
| A3 | https://github.com/anthropics/claude-code/issues/62905 |
| A4 | https://github.com/anthropics/claude-code/issues/65392 |
| A5 | https://github.com/anthropics/claude-code/issues/73408 |
| A6 | https://code.claude.com/docs/en/agent-sdk/user-input |
| O1 | https://github.com/openai/codex/pull/28235 |
| O2 | https://github.com/openai/codex/issues/29702 |
| O3 | https://github.com/openai/codex/blob/main/codex-rs/core/src/tools/handlers/request_user_input.rs |
| O4 | https://github.com/openai/codex/issues/9926 |
| O5 | https://github.com/openai/codex/issues/34455 |
| M1 | https://modelcontextprotocol.io/specification/2025-11-25/client/elicitation |
