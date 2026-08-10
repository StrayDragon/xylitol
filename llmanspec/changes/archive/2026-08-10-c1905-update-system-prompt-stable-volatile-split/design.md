# Design: c1905 system prompt 稳定 / 可变切分

> 一手证据：[`research/stable-volatile-split-2026.md`](./research/stable-volatile-split-2026.md)；主题底稿 [`docs/research/responses-context-layout-and-cache-2026.md`](../../../docs/research/responses-context-layout-and-cache-2026.md) §1。
> **产品钉板（2026-08-10）**：日历日 / cwd **默认不进 system**；以 **`session_env` = 状态栏族特殊类型（bootstrap）** 首轮 Env→user 注入。完整 Lane Runtime → [`c1895`](../c1895-add-agent-status-bar-subsystem/proposal.md)。

## 目标

在 `c1890` 已立的 `ContextPolicy` + Assembler 缝上：

1. system 组装可标注 **stable**（身份、skills 元数据、tools 散文、AGENTS、runtime_policy…）；
2. **`session_env` 作为状态栏族的第一种落地类型**：稀疏 bootstrap（date / clock / cwd），首轮自动构造 Env→user，保持 system 跨日 / resume 前缀稳定；
3. **完整状态栏**（`<agent_status_bar>`、profile、每轮尾插）留给 `c1895`——须能 **扫描 / 合并 / 接管** 本波已持久的 `session_env` 行。

**不**实现完整 StatusBar Lane / tool_search / 布局观测。

## 已钉决策

| ID | 决策 |
|---|---|
| D1 日界 / cache | **否决**默认改写 system 内 date。产品默认：**system 不含 date/cwd**。 |
| D1′ 首轮形状 | 新会话首条 generate 前自动插入一条 **session_env**（状态栏族；Env `CustomMessage`，`project_for_llm` → **user**），再写用户真实输入。用户看到的第一条仍是自己的话；模型侧：`[session_env user][user hello]`。 |
| D2 resume | session_env **持久进 transcript**（append-only）。扫描已有：cwd / 日历日有变则 **再追加**；未变不插。不删旧条。 |
| D3 skills | 元数据仍在 system（stable）；正文 `$skill` → user 投影。 |
| D4 `DatePlacement` | 默认 **`Omit`**。`SystemAsToday` / `SystemPinnedAtSession` 仅消融 / lab。cwd 默认不写 system。pin restore 仅 `#[cfg(test)]`。 |
| D5 instructions | **保持不写**顶栏 `instructions`。 |
| D6 与 c1895 | **`session_env` IS 特殊状态栏类型**（bootstrap），不是临时 hack。c1895 落地时：识别 `custom_type` / XML 根 / `details.status_bar_kind`；决定 off 时保留、replace 合并、或 append 旁路全栏。本波不实现栏 runtime。 |
| D7 TUI | Env `role=custom`（含 session_env）**MUST NOT** 进 scrollback；树 kind=`meta`，标签用 `customType`。 |
| D8 compact / overflow | **本波不修**。`build_context_entries` 常裁掉早期 session_env；下一完整 `run` 会经 `should_append` 再插，但 **overflow 同轮 reload 不注入**。**否决**把 pwd 写回 system（异 cwd resume 仍可能）。Follow-up：**[`c1906`](../c1906-ensure-session-env-after-compaction/proposal.md)**。全栏堆积策略仍归 [`c1897`](../c1897-update-compaction-status-bar-messages/proposal.md)。 |

## 给 c1895 的发现表（实现指针）

| 需要 | 落点 |
|---|---|
| 类型判别 | `CUSTOM_TYPE_SESSION_ENV` = `"session_env"` |
| XML 根 | `SESSION_ENV_XML_ROOT` → `<session_env>`（全栏根名 `<agent_status_bar>`，勿混） |
| 扫最新一条 | `last_session_env` / `session_env_from_message` |
| 是否再追加 | `should_append_session_env`（只比 date/cwd） |
| 注入缝 | ReAct user 落盘前（与日后栏缝同族） |
| TUI | `is_env_custom_message` |
| 模块 | `src/agent/prompt/session_env.rs` |
| 提案 | [`../c1895-add-agent-status-bar-subsystem/proposal.md`](../c1895-add-agent-status-bar-subsystem/proposal.md) |
| Compact / overflow 保证 | → [`../c1906-ensure-session-env-after-compaction/proposal.md`](../c1906-ensure-session-env-after-compaction/proposal.md)（本波不修） |

## 首轮消息心智

```text
generate_options.system_prompt     ← stable only（无 date/cwd）
session history / LLM input:
  Env CustomMessage session_env    → user   # 状态栏族 bootstrap（自动）
  Llm UserMessage（用户输入）       → user   # 用户看见的
```

## 片段标签

| 标签 | 内容 | 规则 |
|---|---|---|
| `stable` | SYSTEM/custom/default、tool snippets、context、APPEND、skills 元数据、Guidelines、runtime_policy | 进 system；会话内只增不改心智 |
| `session_env` | date / clock / cwd | **状态栏族 bootstrap**；默认不进 system；Env→user；可追加 |
| `volatile` / 全栏 | 高频读数、每轮 clock 尾插等 | → `c1895` `<agent_status_bar>`；禁止进 system |

组装顺序（system 内）：

```text
stable body → append/context/APPEND → skills → Guidelines → runtime_policy
# 不再追加 Current date / Current working directory（默认）
```

## session_env 消息形状（已实现）

- `EnvMessage::CustomMessage`，`custom_type = "session_env"`。
- `details` 含 `date` / `clock` / `cwd`，以及 `status_bar_kind: "session_env"`（供栏扫描）。
- `content` XML：

```xml
<session_env>
  <date>2026-08-10</date>
  <clock>2026-08-10T09:56:00Z</clock>
  <cwd>/path/to/project</cwd>
</session_env>
```

- TUI：Custom / meta 过滤，**不**冒充用户打字。
- `project_for_llm`：Custom→user；改模板 = 前缀变更，须显式 change。

## `DatePlacement`（system 侧，消融）

```text
Omit (default):     system 无 Current date
SystemAsToday:      每次 rebuild 写今天（lab / 旧行为对照）
SystemPinnedAtSession: 会话钉死写进 system（lab；产品不默认）
```

cwd：产品路径 **不**经 system；仅 session_env。

## 落点

| 组件 | 层 |
|---|---|
| 片段标签 + `build_system_prompt`（默认无 date/cwd） | `agent/prompt` |
| session_env（状态栏族 bootstrap） | `agent/prompt/session_env.rs` |
| 首轮 / resume 插入缝 | `agent` ReAct（user 落盘前） |
| `DatePlacement` 默认 `Omit` | `agent/context_policy` |
| 全栏 Lane Runtime | out of scope → `c1895` |

## Specs landing

- `agent-prompt`：stable 顺序；默认 system **无** date/cwd；session_env 为首轮 / 变更追加（状态栏族）；单测 + `feature: false`。
- `agent-runtime`：`ar33` date_placement 默认 Omit。

## 测试缝

| 缝 | 方式 |
|---|---|
| 默认 system 无 `Current date:` / `Current working directory:` | unit |
| 空会话首轮：history 投影含 session_env user 再真实 user | unit / react |
| resume 同 cwd+同日：不追加 | unit |
| resume 跨日或 cwd 变：追加一条 | unit |
| body 无顶栏 `instructions` | Assembler 回归 |
| TUI 不展示 session_env 为用户气泡 | unit（bridge / tree） |

## 非目标

- StatusBar Lane / always-on profile（`c1895`）
- Compact / overflow 后强制 ensure session_env（`c1906`）
- 全栏压缩 keep-latest / drop-all（`c1897`）
- tool_search（`c1960`）
- 把 session_env 做成用户可见打字行
- 把 pwd 写回 system（已否决；见 D8）

## Ethics

- risk_level: low
- prohibited_actions: 无标注隐式动态 system 注入；秒级时间进 system；默认日界改写 system
- required_evidence: 默认 system 无 date/cwd；session_env 追加单测；c1895 可发现指针
- refusal_contract: 不把「零动态上下文」写成 MUST（session_env / 栏行仍可变）
- escalation_policy: 若改回默认把 date/cwd 写进 system，须用户确认
