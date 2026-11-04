# Design — 会话阶段名自动生成与持久化

## 目标

在 fresh 会话首个 turn 结束后，用首条用户消息 + 首条助手正文独立生成一个短阶段名；持久化为现有 session display name；默认只自动一次；用户改名钉住；显式刷新可重生成；fork 继承；subagent 排除；阶段名不进入模型上下文。

## 现状（代码事实）

- session display name 已存在：`SessionEntry::SessionInfo(SessionInfoEntry { name })`（`crates/xylitol-plugin-sdk/src/session/entries.rs:183`）；`XySessionStore::get_session_name` / `set_session_name`（`crates/xylitol-plugin-sdk/src/ports/session.rs:120` / `:138`）；写 entry 时 `sanitize_session_display_name`（`:230`）。
- session 列表已投影 `name`（`plugins/xylitol-official-plugin-session/src/manager/store.rs:170`），resume 面板已用（`crates/xylitol-app-tui/src/session_resume/panel.rs`）。
- runtime 已有 `get_session_name` / `set_session_name` / `set_session_name_for`（`crates/xylitol-runtime/src/driver/in_process/session.rs:215` / `:220` / `:228`）；`session_name_fact` 用于 obs（`crates/xylitol-runtime/src/agent/capabilities/session_ops.rs:81`）。
- 协议已有 `get_session_name` / `set_session_name` / `set_session_name_for`（`crates/xylitol-protocol/src/registry.rs:108` / `:109` / `:110`），TUI remote 已接（`crates/xylitol-app-tui/src/driver/remote.rs:1805` / `:1814` / `:1822`）。
- fork 只复制 branch path 的 entries（`plugins/xylitol-official-plugin-session/src/manager/tree.rs:103`），`SessionInfo` 不在 path 时不继承；需要显式复制父名。
- 现有 `sanitize_session_display_name` 只处理换行（`crates/xylitol-plugin-sdk/src/ports/session.rs:230`），不足以做模型输出的终端安全归一化。

## 架构选择

- **复用 session display name**，不新建 title 存储。阶段名 = `SessionInfoEntry.name` 在 TUI 的称谓。
- 生成入口放在 runtime，而不是 TUI：TUI 是 client，模型调用与 session 写必须走 host 操作器角色。
- 新增 `XyDriver::generate_session_name(force: bool) -> Result<String, XyDriverError>`：
  - in-process：读当前 session entries → 组输入 → `ModelManager::build_current_model()`（或配置覆盖）→ 独立 `generate_stream` → 归一化 → `set_session_name` → 返回。
  - remote：经协议 unary `generate_session_name` 调 host。
- 新增协议 unary `generate_session_name`（Writer / Result / PerRpc / Queued），参数 `{ "force": bool }`；新增 `Command::GenerateSessionName { force }` 与 registry 行；复用 `get_session_name` / `set_session_name` 读写。
- 不新增 `XyEvent` 变体：TUI 以 unary 返回更新标题；多客户端同步可选发 `SessionInfoChanged { key: "stage_name", value: {"name": ...} }`，MVP 不做。
- 触发由 TUI 在首个 `AgentEnd` 后发起（c3115），host 幂等：已有名且 `force=false` 直接返回。

## 生成时序

1. TUI 在 fresh 会话首个 `AgentEnd` 后，若本地无阶段名，调用 `generate_session_name(force=false)`（异步，不阻塞输入）。
2. host 读当前 session id；无 session → 错误。
3. host 读 entries：首条 user message 文本、首条 assistant 正文；若 assistant 正文为空（首轮只有工具），只用 user 文本。
4. host 若已有阶段名且 `force=false` → 返回既有名。
5. host 组装系统提示 + 输入 JSON，截断到 `max_input_bytes`；调用当前模型（或配置 provider/model）；thinking 取最低可用档。
6. `timeout_ms` 超时、provider 错误、空输出、归一化后为空 → 回退名。
7. 归一化 → `set_session_name` → 更新 `session_name_fact` / obs → 返回。
8. TUI 收到结果，更新编辑区上边框；失败时 host 仍返回回退名，不向用户抛错。

## 提示词与输入

系统提示（中文/英文随会话语言）：

```text
Create a concise session stage name. Return only the name on one line, no quotes, no markdown.
Aim for 12–15 CJK characters or at most 20 English words. Use the language of the conversation.
Name the main work, not a greeting. Prefer the user's leading words when the result is unclear.
```

用户输入（JSON，便于截断与转义）：

```json
{"user":"<first user message>","assistant":"<first assistant text>"}
```

- 只取首条 user 与首条 assistant 正文；不取 tool args / tool output / thinking。
- 输入超过 `max_input_bytes` 时按 UTF-8 安全截断，不拆码点。
- 生成请求不带 tools、不带历史；`max_output_tokens=64`。

## 长度与归一化

- 目标：`target_cjk_chars=15`、`target_words=20`、`max_title_bytes=120`。
- 归一化：去 OSC / CSI / ESC / C0-C1 控制字符 / 方向控制（U+200B / U+200E/F / U+202A-E / U+2060-64 / U+2066-6F / U+FEFF）；`\s+` 折成单空格；trim；UTF-8 安全截断到 `max_title_bytes`；空串 → 回退。
- 不截断出半个 grapheme；按码点截断即可（与 DSH 一致）。
- 回退名：`fallbackSessionTitle` 等价逻辑——首条 user 清洗后取前 `target_words` 个空白词 / 前 `max_title_bytes` 字节。

## 钉住 / 刷新 / fork / subagent

- 已有名且 `force=false` → 返回既有名，不调用模型。
- 用户改名（既有 `set_session_name`）→ 名字存在即钉住；自动生成不会覆盖。
- `force=true`（显式刷新）→ 忽略既有名重新生成；失败保留既有名并返回既有名；没有既有名则回退。
- fork：`fork_session` 成功后读父 session name；若子会话无 `SessionInfo`，`set_session_name` 复制父名；不自动重命名。
- subagent：host 在生成入口检查 session 来源；subagent 会话直接返回既有名或空，不调用模型。
- resume：TUI 读既有名；不触发自动生成。
- aborted / error 的首轮：不自动生成；保留空或既有名；用户可 `/stage-name refresh` 显式重试。

## 配置

```yaml
session:
  stage_name:
    enabled: true
    target_cjk_chars: 15
    target_words: 20
    max_title_bytes: 120
    max_input_bytes: 8192
    max_output_tokens: 64
    timeout_ms: 8000
    provider: null   # 可选；缺省当前模型
    model: null      # 可选；与 provider 成对
```

- 非法值（0 / 负数 / max_title_bytes < fallback）→ 配置加载失败或回落默认并 warn；以 `runtime-config` 的校验风格为准。
- 配置由 `plugins/xylitol-official-plugin-config/src/types.rs` 承载，schema 自动再生。

## 协议

- `METHOD_GENERATE_SESSION_NAME = "generate_session_name"`；registry row：`Auth::Writer`、`Idem::PerRpc`、`Resp::Result`、`CMD`、`Exec::Queued`（与 `set_session_name` 同族）。
- 参数：`{ "force": bool }`，`force` 缺省 false。
- 返回：`{ "name": string }`，与 `set_session_name` 返回形状一致。
- TUI remote 新增 `exec_generate_session_name`；in-process 直接调 `XyDriver`。

## 安全 / 隐私

- 阶段名只落 session log（`SessionInfoEntry`），`project_for_llm` 不读；不进入系统提示 / 上下文。
- 生成请求的输入只含首条 user / assistant 正文；不含 tool args、output、thinking、文件内容。
- 模型输出按终端安全归一化，避免 ANSI / 方向控制注入。
- 生成请求失败只 warn，不向用户抛错；回退名保证可用。

## 实施任务（apply 时转 tasks.md）

1. plugin-sdk：新增 `normalize_stage_name` / `fallback_stage_name`（或放 runtime），复用 `sanitize_session_display_name`；单测覆盖 ANSI / 控制字符 / UTF-8 截断 / 空串。
2. runtime：新增 `agent/session/stage_name.rs`；`XyDriver` 加 `generate_session_name`；in-process 实现读 entries + 调模型 + 写名；测试用 fake model / MemoryStore。
3. runtime fork：`fork_session` 后复制父名；测试 fork 继承。
4. protocol：新增 `METHOD_GENERATE_SESSION_NAME` + registry row；registry 单测；wire 无新事件。
5. serve：`dispatch_session_unary` 处理 `generate_session_name`；auth / busy / 幂等测试。
6. config：`session.stage_name` 类型 / 默认 / 校验 / schema；配置单测。
7. specs：`agent-session` 新规则；`runtime-config` 字段；`protocol-app` 方法；BDD 场景（首轮自动 / 回退 / 钉住 / 刷新 / fork / subagent / 不进上下文）。
8. TUI 接线留给 c3115；本 change 只到 driver + unary + 配置 + specs。

## 验证

- 单测：归一化、回退、长度、UTF-8、控制字符。
- runtime 单测：fake model 返回正常 / 超时 / 空 / 超长；已有名不调用；force 重生成；fork 继承；subagent 跳过。
- serve 单测：unary auth / 参数 / 返回；busy 时拒绝或排队。
- BDD：首轮自动生成、回退、用户改名钉住、刷新、fork 继承、阶段名不进模型上下文。
- 真机：tmux 下首轮后标题出现；改名 / 刷新；resume 恢复。

## 风险 / 待确认

- 生成请求在主回复结束后异步；TUI 标题可能延迟数秒；回退名可先显示（c3115 可选择先显示回退名，再由 auto 覆盖；本 change 只返回最终名）。
- 阶段名与 session resume rename 共用存储；自动生成必须尊重用户改名，靠「名字存在即钉住」实现；若未来要区分 auto / user，需加 metadata（本 change 不做）。
- `set_session_name` 的 sanitize 较弱；生成路径用独立归一化，避免改动既有用户改名行为。
- 多客户端同步不在 MVP；其他客户端需重新 `get_session_name`。
- 模型选择：缺省当前模型；如果当前模型不可用，回退名，不尝试其他模型。

## 参考

- 本 change 一手摘录：[`research/dsh-session-title.md`](./research/dsh-session-title.md)。
- DSH `@deepseek-ai/dsh-session-title`、`dsh-session-title-first-prompt-llm`、`dsh-session-title-llm`：log-backed title service、fallback、first-prompt provider、长度目标、归一化、钉住 / refresh。
- opencode session title / tool details visibility：见 c3120 与 c3130 的 research。
