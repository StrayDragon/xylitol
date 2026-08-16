# 调研：session 盘面格式 v6（2026-08-17）

结论与决策依据的原始证据。行号基于 main `31618ae3`；执行时以符号为准复核。

## 1. 两套时间戳的现状

| 位置 | 类型 | 生成 |
|---|---|---|
| 条目壳 `EntryBase.timestamp`、`SessionHeader.timestamp`（`src/protocol/session/entries.rs:27-55`） | String RFC3339 | `rfc3339_now()`（`src/infra/session/manager.rs:15`；调用点 `:279,390,420`） |
| message 内 `timestamp`（bridge DTO：user/assistant/toolResult 三臂，`packages/xylitol-ai-bridge/src/dto/message.rs:26,60,77`） | u64 unix-ms | `now_ms()`（DTO `default`；`src/protocol/message.rs:19` 同名 helper） |
| `streamTiming` 节点 / `thinkingElapsedSecs`（c2240 侧带） | u64 unix-ms | `StreamNodeClock` |

消费点：
- `list_sessions`（`manager.rs:1609-1620`）：`time::OffsetDateTime::parse(Rfc3339)` 解析 header.timestamp → `modified_unix`（秒）；失败回退文件 mtime。语义注意：header 时间戳是**创建时刻**，被当列表时间源用（现状即如此，本票不改此产品语义）。
- HTML 导出标题（`src/app/core/session_export.rs:130`）：`@ {h.timestamp}` 直渲染字符串。
- TUI resume 面用 `modified_unix`（`src/app/tui/session_resume/panel.rs:334`、`search.rs:249-263`），不感知。
- `time` crate 已是 manager 依赖（Rfc3339 解析在用），展示边格式化不需要新依赖。

## 2. alias 全量清单与用途判定

`rg 'serde\(alias|alias ='` 全仓结果（排除测试）：

- `src/protocol/message.rs:40,49,52,58,70,72,74,79,87` — EnvMessage 9 个 snake alias；模块注释（`:30-31`）「snake aliases accept pre-fix JSONL」。
- `packages/xylitol-ai-bridge/src/dto/message.rs:36,50,56,67,69,74,273,275,277,293,295` — 11 个；注释（`:27` 等）「snake aliases read pre-fix JSONL」。

判定为「纯旧盘读兼容」的依据：上游 provider 原始键与这些短名无对应（Anthropic `cache_read_input_tokens` / `cache_creation_input_tokens`；OpenAI `cached_tokens`），adapter 显式映射到 DTO；注释自述用途为 pre-fix JSONL。与 s18「MUST NOT 提供 serde alias」直接冲突 → 删代码对齐 spec（spec 文本除版本号外不动）。

与 spec 的关系：
- s18（`agent-session-store/spec.toon:19`）禁止 alias —— 删除后为真。
- s19 要求 toolCallId 键、MUST NOT toolUseId —— DTO 的 `tool_use_id` alias 让旧盘 `tool_use_id` 键仍可读，删除后彻底对齐。

## 3. streamTiming 侧带机制（c2240 引入，保持行为）

- 注入：`persist_agent_message_with_thought_elapsed`（`src/agent/runtime/react/support.rs:78-110`）先把 `AgentMessage` 序列化为 `Value`，再直注 `thinkingElapsedSecs`（`:89`）与 `streamTiming`（`:97`）顶层键；doc 注释明写「Extra JSON is ignored when history deserializes to AgentMessage for LLM projection」。
- 消费：类型化投影靠 serde 忽略未知字段；TUI resume 经 `src/app/tui/bridge/session_tree.rs` raw JSON 索引（执行时 rg 复核精确行）。
- 结论（2026-08-17 会话）：既非读写模型不一致，也非历史遗留，是刻意的无类型侧带。用户决定：忽略行为保留至首个公共版本前；本票只加形状锁定测试，收紧（正式 schema 字段）另票。

## 4. 版本纪律现状（v6 沿用，只改数字）

- `SESSION_VERSION: u32 = 5`（`entries.rs:12`），版本史在注释（v3 无树 / v4 snake+untagged / v5 camelCase+tagged AgentPart）。
- 读拒绝非当前版本：`enforce_session_version`（`src/protocol/session/parse.rs:43-59`），文案 `unsupported_session_version_msg` 用常量拼「require {SESSION_VERSION}」→ 自动变 6。
- 列表快路径 `peek_session_header_version`（`parse.rs:62-73`）读数字，逻辑不变。
- export JSONL 与存储同构纯转发（`src/app/core/session_export.rs:202-211`）；import 走 `parse_session_jsonl` + 版本强制（`:217-232`）→ v5 导入件自动被拒，符合「不迁移」决定。

## 5. 破坏性影响（用户已确认接受）

- 现有 `~/.xylitol/sessions/`（实测 168 个文件）全部 v5 → 升级后不可读（「require 6」可操作错误）。
- 需要留存的会话先 `/session-export`（jsonl 与存储同构，为未来迁移源）。

## 6. 用户决策记录（2026-08-17 会话）

- 不做任何兼容：一步到位、不迁移、遇处理不了快速报错；尽全力移除兼容内容。
- 时间戳两套基准是问题 → 本票统一为 unix-ms。
- streamTiming 忽略行为正确、保留；公共版本前再收紧。
- s18 与实现的出入以**删代码对齐 spec** 收口。
- 文件名仅 UUID + list 扫描：保持原样（无动作）。
- 顺序约束：先 `c2250-harden-session-write-path`（写入路径加固），后本票。
