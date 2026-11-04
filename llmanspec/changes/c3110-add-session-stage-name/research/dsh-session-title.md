# DSH session title research（一手摘录）

来源：本机 DSH checkout `/Users/l8ng/.bun/install/global/node_modules/@deepseek-ai/`。只读摘录，未改 DSH。

## 包与职责

- `dsh-session-title`：log-backed session title service + provider registry；client-visible title；persist through replay/resume/paging；**never enter model input**；automatic generation never delays main response；newer title requests supersede older work；`refresh()` 显式重生成。
- `dsh-session-title-first-prompt-llm`：first eligible human message → 一次 auxiliary request；只对 fresh session（无 parent、无既有 title）自动；seeded / subagent-origin 排除；fork 继承、不自动重命名。
- `dsh-session-title-all-prompts-llm`：all eligible messages → 每次用户消息后新 revision；若当前 title 仍准确则保留；只在 material change 时更新。
- `dsh-session-title-llm`：共享 LLM execution controls：`maxInputBytes` / `maxOutputTokens` / `timeoutMs` / `provider` / `model`；least supported reasoning effort；provider-selected reasoning。
- `dsh-experimental-session-titles-bundle`：bundle。

## 来源优先级（newest wins）

1. 内置 fallback：first eligible human message 的 leading words，受 `fallbackMaxWords` / `fallbackMaxBytes` 限制。
2. 注册 provider 生成的 title。
3. 用户显式 `rename()`。

用户来源的 latest title **pins** session：后续用户消息不再自动 revision；显式 `refresh()` 是 deliberate unpin。

## 配置（全部必填，库不提供默认）

```yaml
- name: '@deepseek-ai/dsh-session-title'
  config:
    fallbackMaxWords: 8
    fallbackMaxBytes: 96
    maxTitleBytes: 120
```

Provider 配置：`targetWords` / `targetCjkCharacters` / `maxInputBytes` / `maxOutputTokens` / `timeoutMs` / `provider` / `model`。

## Provider 行为（first-prompt）

- 只在 fresh session（no parent、no prior title）自动；first eligible human message 后，fallback 先确保，然后一次 auxiliary request。
- 后续 prompts / renames / inherited forks 不触发。
- 失败 warn，保留 current title；explicit refresh 重试。
- 请求走 marked loop-built request；route 与 logged `request/header` 匹配后才开始；newer revision 中止 older work。
- title request 与 main request 分离；不占 main latency/tokens。

## 归一化（`normalize.js`）

- 去 OSC / CSI / ESC 序列；去 C0/C1 控制字符；去方向 / 不可见控制（U+200B、U+200E/F、U+202A-E、U+2060-64、U+2066-6F、U+FEFF）；`\s+` 折单空格；trim。
- `truncateTitleUtf8` 按码点截断，不拆 UTF-8。
- fallback：清洗后取前 `maxWords` 个空白词，再按 `maxBytes` 截断。

## 对本仓设计的映射（c3110）

- 复用既有 session display name（`SessionInfoEntry.name`）作为阶段名存储；不新建平行 title 系统。
- 触发：首个会话首个 `AgentEnd` 后，TUI 调 `generate_session_name(force=false)`；host 读首条 user + 首条 assistant 正文；默认只一次。
- 回退：模型失败/超时/空 → 首条 user 前导词；保证总有可显示名。
- 钉住：已有名且 `force=false` 直接返回；用户 `set_session_name` 改名即钉住；`force=true` 刷新才覆盖。
- fork 继承父名；subagent 排除；title 不进 `project_for_llm`。
- 长度：`target_cjk_chars=15` / `target_words=20` / `max_title_bytes=120`；输入 `max_input_bytes=8192`；输出 64；timeout 8s；thinking 最低档。
- 与 DSH 的差异：DSH 是 host-side service + log event；本仓用 session name + unary，TUI 触发；DSH first-prompt 只用 user；本仓按用户要求加首个 assistant 正文。

## 未做 / 不照搬

- 不照搬 all-prompts 持续重命名。
- 不做 provider registry / 多 provider 竞争。
- 不做 log-only title event 新变体；复用 session name 存储。
- 不做跨客户端实时广播；MVP 以 unary 返回为准。
