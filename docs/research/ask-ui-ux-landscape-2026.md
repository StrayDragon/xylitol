# Ask / questionnaire UI·UX landscape（一手调研，2026-08）

> **范围**：coding-agent「结构化问用户」的布局与视觉层级（TUI + IDE 卡）。
> **约束**：一手来源（官方文档、带 UI 引文的 issue/PR、协议原文）。不改 `llmanspec/specs/**`。
> **相关**：skip 语义 → [`ask-tool-skip-semantics-2026.md`](./ask-tool-skip-semantics-2026.md)；产品意图 → [`src/app/tui/design/ask.md`](../../src/app/tui/design/ask.md)。

## 跨产品对照（摘要）

| 产品 | 选项布局 | 描述 / 例子位置 | 多题导航 | Skip / 提交 | 窄 TUI 可迁移点 |
|------|----------|-----------------|----------|-------------|-----------------|
| Claude `AskUserQuestion` | 垂直选项列表；有 `preview` 时左列表+右预览 | `description` 随选项；`preview` 右侧/旁侧 | header 短标签（≤12）作题签 | Other / notes；timeout 可配；Esc≠Skip | 列表+行下描述；宽时再开右栏 |
| Cursor `AskQuestion` | IDE **问卷卡**（点选） | 社区 schema 多为 `label`；描述常塞进 label | 多题一轮；旧卡易消失 | **Skip** + **Continue** | 显式 Skip；勿超时伪装 skip |
| Warp Agent questions | 对话内 **card**；编号选项 | 文档未细说行下描述；有 **recommended** | ←→ + counter + prev/next | **Skip all** / `Ctrl+C` | 数字快捷键；单选 auto-advance |
| Codex `request_user_input` | TUI **overlay**；一题一屏 | `description` 必填；首项 **(Recommended)** | PageUp/Down；Enter 推进 | Esc 中断；客户端加 **Other**；notes 常在 | 优先保「题+选项」；窄宽压 notes |
| Gemini `ask_user` | 交互 dialog；PR 含 Review tab | `label`+`description` | ←→ / Tab；多题 Review | Esc dismiss；choice 可 Other | header chip + 行下描述 |
| OpenCode `question` | TUI 列表；多选 `[✓]` | `opt.description` 渲染在选项旁 | header tabs + Confirm | Esc reject；单选即时提交 | tabs→Review；数字键 1–9 |
| MCP elicitation | 客户端自定表单 | schema `title`/`description`→标签/帮助 | 协议不规定 | **decline** + **cancel**（Esc→cancel） | flat enum；勿嵌套对象 |

---

## 1. Claude Code `AskUserQuestion`

**Schema（官方）**：`questions[]` 含 `question`、`header`（≤12）、`options[]` 的 `label`+`description`、`multiSelect`；TS 可选 `preview`（markdown ASCII / html 片段）([user-input](https://code.claude.com/docs/en/agent-sdk/user-input#question-format))。产品面：选选项，或经 **Other 行 / notes 字段** 自填；自填以中性措辞回灌模型 ([tools](https://code.claude.com/docs/en/tools#askuserquestion-tool-behavior))。SDK 建议 UI 在 Claude 选项后追加 **Other**，答案用用户原文而非字面 “Other” ([user-input § free-text](https://code.claude.com/docs/en/agent-sdk/user-input#support-free-text-input))。插件指南：每调用 1–4 题、每题 2–4 选项 ([interactive-commands](https://github.com/anthropics/claude-code/blob/main/plugins/plugin-dev/skills/command-development/references/interactive-commands.md))。

**右栏预览**：`previewFormat` 打开后，选项可带视觉 mockup，「alongside the label」([user-input § Option previews](https://code.claude.com/docs/en/agent-sdk/user-input#option-previews-typescript))。社区镜像源码 `PreviewQuestionView` 注释写明：**左侧垂直选项列表 + 右侧 preview 面板**；preview 题不挂 Other ([Open-ClaudeCode PreviewQuestionView](https://github.com/LING71671/Open-ClaudeCode/blob/main/src/components/permissions/AskUserQuestionPermissionRequest/PreviewQuestionView.tsx))。同组件 footer 文案含 “Chat about this”、plan 下 “Skip interview…”，以及 `Enter` / ↑↓ / `n` notes 提示（同源文件）。

**Timeout**：默认真等待；可配 `askUserQuestionTimeout`；末 20s 倒计时 ([tools](https://code.claude.com/docs/en/tools#question-auto-continue-timeout)、[settings](https://code.claude.com/docs/en/settings))。

**→ 窄 TUI**：默认 **单栏列表 + 行下 `description`**；仅当宽且有 `preview`/长例子时再开右栏，否则叠在选中项下。

---

## 2. Cursor `AskQuestion`

**触发**：内置 Ask questions 工具 →「special UI… clickable options」（员工 Dean，[forum #152102](https://forum.cursor.com/t/how-can-i-use-clarifying-questions-with-my-skill/152102)）。**Skip / Continue**：用户明确提到未点 Skip/Continue 仍被跳过 ([#158485](https://forum.cursor.com/t/askquestion-tool-can-return-synthetic-skip-string-with-highly-variable-unpredictable-delay/158485))。Skip 载荷成功串：`Questions skipped by the user, continue with the information you already have`（同帖；与 timeout 字节相同问题见 skip 调研）。

**Schema（社区技能稿，非 Cursor 官方 API 页）**：`id` / `prompt` / ≥2 `options`（`id`+`label`）；`allow_multiple`；可选 `title`；记忆偏好可作 *suggested* 选项但仍由用户选 ([baoyu-design cursor.md](https://github.com/jimliu/baoyu-design/blob/HEAD/skills/baoyu-design/references/cursor.md))。第三方命令把说明写进 label：`"Name — brief description"`、`"Other — I'll type…"` ([create-component-cursor.md](https://github.com/Ankish8/myoperator-plugins/blob/main/plugins/myoperator-workflows/commands/create-component-cursor.md))。

**UI 注意**：同时仅最新卡可见，旧卡从历史消失 ([#158485](https://forum.cursor.com/t/askquestion-tool-can-return-synthetic-skip-string-with-highly-variable-unpredictable-delay/158485))。

**→ 窄 TUI**：学其 **显式 Skip + 提交**；避免 IDE 卡消失问题——scrollback 留人话摘要。描述字段若 schema 无，可放选中行下而非依赖宽卡。

---

## 3. Warp Agent questions

官方文档：对话内 **Agent questions card**；单选/多选；可启用 **Other**；可标 **recommended**；多题 prev/next + counter；编号键 `1`…；单选选后 **自动前进**；**Skip all** 或 `Ctrl+C`；完成后会话摘要，可展开复核 ([Agent questions](https://docs.warp.dev/agent-platform/local-agents/interacting-with-agents/agent-questions/))。权限档：Never / Ask unless auto-approve / Always ask（同页）。GUI 卡按内容增高（单题至 ~800px）([PR #11719](https://github.com/warpdotdev/warp/pull/11719))；TUI 共享同一状态机（编号、Other、导航、Ctrl+C skip-all）([PR #13830](https://github.com/warpdotdev/warp/pull/13830))。

**→ 窄 TUI**：编号选择 + 单选 auto-advance + 底栏 Skip all 最省行高。

---

## 4. OpenAI Codex `request_user_input`

**工具 schema**：1–3 题；每题 `header`（≤12）、`question`；选项 2–3，**必填** `label`+`description`；「Put the recommended option first and suffix… `(Recommended)`」；**勿**在列表里放 Other——客户端自动加 free-form Other ([request_user_input_tool.rs](https://github.com/openai/codex/blob/35aaa5d9/codex-rs/tools/src/request_user_input_tool.rs)；早期提案 [PR #9472](https://github.com/openai/codex/pull/9472))。

**TUI overlay（一手 note）**：一次一题；有选项时默认选第一项；**notes 始终可用**且按所选选项存；Up/Down + Space；打字切到 notes；Enter 下一题 / 末题提交；PageUp/Down 跨题；选项模式 Esc **打断 run**；窄宽时优先保证题+选项可见，notes/footer 折叠为单行 `Notes: …` ([docs/tui-request-user-input.md](https://github.com/openai/codex/blob/d47b755a/docs/tui-request-user-input.md))。

**→ 窄 TUI**：Codex 的「挤 notes、保选项」是宽 80–120 的默认策略；`(Recommended)` 用标签后缀比独立徽章省空间。

---

## 5. 其他 TUI

### Gemini CLI `ask_user`

官方：1–4 题；`header` chip（≤16）；`choice` / `text` / `yesno`；choice 需 `label`+`description`；`multiSelect` 时自动加 “All the above”；dismiss 返回模型 ([ask-user.md](https://github.com/google-gemini/gemini-cli/blob/main/docs/tools/ask-user.md)、[geminicli.com](https://geminicli.com/docs/tools/ask-user/))。UI PR：Other、←→、**Review tab**、未答题警告、Esc cancel ([PR #17344](https://github.com/google-gemini/gemini-cli/pull/17344))。

### OpenCode `question`

内置 `question` 工具（非仅插件）([issue #7786](https://github.com/anomalyco/opencode/issues/7786))。TUI 源码：header **tabs** + 末 **Confirm**；选项下渲染 `opt.description`；多选 `[✓]`；默认 “Type your own answer”；单题单选直接 reply；Esc → `reject`；数字键 1–9 ([question.tsx](https://github.com/anomalyco/opencode/blob/5d2dc888/packages/opencode/src/cli/cmd/tui/routes/session/question.tsx))。早期 askquestion PR：`○/●` vs `[ ]/[✓]`、单选 auto-advance ([PR #5958](https://github.com/sst/opencode/pull/5958))。

### Aider / Goose

既有 TUI 景观调研未记录结构化问卷工具；Aider/Goose 以线性 transcript + slash 为主 ([coding-agent-tui-design-landscape-2026.md](./coding-agent-tui-design-landscape-2026.md) 引 [aider commands](https://aider.chat/docs/usage/commands.html)、[goose CLI](https://goose-docs.ai/docs/guides/goose-cli-commands/))。**本调研未找到**一等 `AskUserQuestion` 式 UI。

---

## 6. MCP elicitation（表单模式）

协议不规定具体控件；客户端 MUST：标明请求方、提供 **decline 与 cancel**、form 下允许提交前修改；Escape 等属 **cancel**（非 decline）([spec 2025-11-25](https://modelcontextprotocol.io/specification/2025-11-25/client/elicitation))。Schema：**flat** 原始类型；`title`/`description`；enum 单选/多选 ([同页 Requested Schema](https://modelcontextprotocol.io/specification/2025-11-25/client/elicitation))。Microsoft Cowork：`title`/`description` → 表单标签与帮助；enum → dropdown；外观不可定制 ([cowork-elicitation-forms](https://learn.microsoft.com/en-us/microsoft-365/copilot/cowork/cowork-elicitation-forms))。

**→ 窄 TUI**：enum 列表 ≈ choice；description 作 muted 帮助行；保留 decline≠cancel 语义（与 Cursor 混 skip 对照）。

---

## Transfer：IDE 卡 → 窄终端（~80–120）

| IDE / 宽 UI | 窄 TUI 替代 |
|-------------|-------------|
| 右栏 preview / 大卡 | 选中项下方展开描述或 ASCII preview；宽阈值再两栏 ([Claude preview](https://code.claude.com/docs/en/agent-sdk/user-input#option-previews-typescript)、[Codex layout priorities](https://github.com/openai/codex/blob/d47b755a/docs/tui-request-user-input.md)) |
| 鼠标点选卡 | ↑↓ / 数字键 / `→` 焦点 ([Warp](https://docs.warp.dev/agent-platform/local-agents/interacting-with-agents/agent-questions/)、[OpenCode](https://github.com/anomalyco/opencode/blob/5d2dc888/packages/opencode/src/cli/cmd/tui/routes/session/question.tsx)) |
| 多卡并排 | **一题一屏** + header tabs + Review ([Gemini PR](https://github.com/google-gemini/gemini-cli/pull/17344)、xylitol [`ask.md`](../../src/app/tui/design/ask.md)) |
| 彩色卡片墙 | **左边轨 + semantic color**；Ask 结束后 **勿** `tool-*-bg` 洗底（[`DESIGN.md`](../../src/app/tui/DESIGN.md) c1830；[`ask.md`](../../src/app/tui/design/ask.md)） |

---

## Recommendation for xylitol Ask（<350 字）

对齐已定稿意图：问卷在 **editor 槽**，结束后 scrollback **人话摘要**（非 JSON tool 块）([ask.md](../../src/app/tui/design/ask.md))。

1. **固定左轨 + 语义色**：进行中/成功用 accent/success 轨区分「Ask 等待」与「已答/跳过」；**禁止**默认 `tool-*-bg`（[`DESIGN.md`](../../src/app/tui/DESIGN.md)；对照 Claude/Codex 用结构层级而非整行洗底）。
2. **选项列表优先**：单选 `→`、多选 `[x]`（ask.md）；行下短 `description` 作易懂说明——Codex/Claude/Gemini/OpenCode 均把说明绑在选项上 ([Codex schema](https://github.com/openai/codex/blob/35aaa5d9/codex-rs/tools/src/request_user_input_tool.rs)、[Claude](https://code.claude.com/docs/en/agent-sdk/user-input#question-format)、[OpenCode](https://github.com/anomalyco/opencode/blob/5d2dc888/packages/opencode/src/cli/cmd/tui/routes/session/question.tsx))。
3. **宽时增强**：≥~100 列且有长例子/`preview` 时，右栏或选中项旁显示 AI「易懂例子」；窄宽折叠到选中项下（Claude 左列表+右 preview；Codex 挤 notes）([Claude](https://code.claude.com/docs/en/agent-sdk/user-input#option-previews-typescript)、[Codex TUI note](https://github.com/openai/codex/blob/d47b755a/docs/tui-request-user-input.md))。
4. **字符级强调**：仅对 `(Recommended)`、当前焦点标签、多选勾选做 accent/bold；避免整行反色噪音（Warp recommended；Codex label 后缀）。
5. **Skip 底栏常显** + Esc=skip（产品已拍板）；对照 Cursor 显式 Skip，避免 timeout 复用 skip 串 ([Cursor #158485](https://forum.cursor.com/t/askquestion-tool-can-return-synthetic-skip-string-with-highly-variable-unpredictable-delay/158485)；[ask.md](../../src/app/tui/design/ask.md))。
6. **多题**：header tabs → Review → Enter 提交（Gemini/OpenCode/ask.md）；单选可学 Warp auto-advance 减键次。

---

## Sources（索引）

- Claude: https://code.claude.com/docs/en/agent-sdk/user-input · https://code.claude.com/docs/en/tools · https://github.com/anthropics/claude-code/blob/main/plugins/plugin-dev/skills/command-development/references/interactive-commands.md · https://github.com/LING71671/Open-ClaudeCode/blob/main/src/components/permissions/AskUserQuestionPermissionRequest/PreviewQuestionView.tsx
- Cursor: https://forum.cursor.com/t/how-can-i-use-clarifying-questions-with-my-skill/152102 · https://forum.cursor.com/t/askquestion-tool-can-return-synthetic-skip-string-with-highly-variable-unpredictable-delay/158485 · https://github.com/jimliu/baoyu-design/blob/HEAD/skills/baoyu-design/references/cursor.md
- Warp: https://docs.warp.dev/agent-platform/local-agents/interacting-with-agents/agent-questions/ · https://github.com/warpdotdev/warp/pull/11719 · https://github.com/warpdotdev/warp/pull/13830
- Codex: https://github.com/openai/codex/blob/d47b755a/docs/tui-request-user-input.md · https://github.com/openai/codex/blob/35aaa5d9/codex-rs/tools/src/request_user_input_tool.rs · https://github.com/openai/codex/pull/9472
- Gemini: https://github.com/google-gemini/gemini-cli/blob/main/docs/tools/ask-user.md · https://github.com/google-gemini/gemini-cli/pull/17344
- OpenCode: https://github.com/anomalyco/opencode/blob/5d2dc888/packages/opencode/src/cli/cmd/tui/routes/session/question.tsx · https://github.com/sst/opencode/pull/5958 · https://github.com/anomalyco/opencode/issues/7786
- MCP: https://modelcontextprotocol.io/specification/2025-11-25/client/elicitation · https://learn.microsoft.com/en-us/microsoft-365/copilot/cowork/cowork-elicitation-forms
- xylitol: `src/app/tui/design/ask.md` · `src/app/tui/DESIGN.md`
