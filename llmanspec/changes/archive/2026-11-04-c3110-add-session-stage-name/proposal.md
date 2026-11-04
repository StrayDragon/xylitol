---
depends_on: []
title: 会话阶段名自动生成与持久化
needs_specs_change: true
branch: sdd/c3110-add-session-stage-name
base_branch: main
base_sha: 1d2d21ebec4818af8c83b0fd3f37fc3b0b95eed1
---

# 会话阶段名自动生成与持久化

## Why

产品 TUI 目前只有 footer 的 cwd / model / thinking，会话身份只在已下线的「详情」overlay 里可见；resume / fork / 多会话切换时缺少一个短、稳定、人类可读的显示名。用户要求：首个会话的首条用户消息得到首个结果后，独立请求模型生成一个合适的短名；默认只在第一次做；改名或需要时再触发一次。该短名在 TUI 中统一称 **阶段名**，避免与任务名、计划步骤、todo 项冲突。

## 术语

- **阶段名**：当前会话的短显示名，用于 TUI 编辑区上边框与 resume 列表；底层复用现有 session display name（`SessionInfoEntry.name`），不新建平行标题系统。
- **自动生成**：首个用户消息 + 首个助手正文作为输入，独立 LLM 请求生成；不进入主对话上下文，不阻塞主回复。
- **回退名**：模型失败/超时/输出非法时，从首条用户消息前导词截取；保证总有可显示名。
- **钉住**：用户显式改名后，自动生成不得覆盖；显式刷新才解除并重新生成。

## What Changes

- 新增 runtime 能力 `generate_session_name`：在 fresh 会话首个 turn 结束后，读首条用户消息与首条助手正文，独立调用当前模型（或配置覆盖）生成阶段名；归一化后经现有 `XySessionStore::set_session_name` 持久化为 `SessionInfoEntry`。
- 新增 `XyDriver::generate_session_name(force: bool)` 与协议 unary `generate_session_name`（Writer / Result）；新增 `Command::GenerateSessionName { force }` 与 registry 行；复用既有 `get_session_name` / `set_session_name`。
- 生成策略：目标 12–15 个 CJK 字符或 ≤20 个英文词；`max_title_bytes` 默认 120；输入 `max_input_bytes` 默认 8192；`max_output_tokens` 默认 64；`timeout_ms` 默认 8000；thinking 取当前模型最低可用档（优先 off）。
- 回退：模型失败/超时/空输出/超长归一化后为空 → 用首条用户消息前导词生成回退名并持久化。
- 钉住语义：已有阶段名且 `force=false` → 直接返回既有名；用户 `set_session_name` 改名即钉住；`force=true` 显式刷新，失败保留既有名。
- 作用域：fresh 会话自动一次；fork 继承父会话阶段名且不自动重命名；subagent 会话不生成；resume 读既有名。
- 配置：`session.stage_name`（enabled / target_cjk_chars / target_words / max_title_bytes / max_input_bytes / max_output_tokens / timeout_ms / provider / model）。
- 归一化：去 ANSI / OSC / CSI / 控制字符 / 方向控制；空白折叠为单行；UTF-8 安全截断；拒绝空串。
- 阶段名只落 session log，不进入 `project_for_llm`；不新增 `XyEvent` 变体，复用 `SessionInfoChanged` 作为可选多客户端同步（MVP 以 unary 返回为准）。

## Capabilities（拟）

- `agent-session`：阶段名自动生成 / 回退 / 钉住 / 刷新 / fork 继承 / subagent 排除 / 不进模型上下文。
- `runtime-config`：`session.stage_name` 配置字段与默认值。
- `protocol-app`：`generate_session_name` unary 方法、auth / resp / idem 行；与既有 `get_session_name` / `set_session_name` 同族。

## Impact

runtime（`XyDriver` / in-process / remote）、serve unary 分派、protocol registry、plugin-sdk session store 复用、config 类型与 schema、app-tui host client（c3115 接线）。不改主 ReAct loop 的决策与消息落盘；不新增 crate。

## 非目标

- 不做「所有用户消息持续重命名」（DSH all-prompts 策略）；本 change 只做首轮一次 + 显式刷新。
- 不做跨会话标题服务 / 标题市场 / 多 provider 竞争；单 provider 即当前模型或配置覆盖。
- 不做 TUI 展示与斜杠命令（c3115）。
- 不把阶段名注入模型输入或系统提示。
- 不做 fork 自动重命名；fork 只继承。
- 不改既有 session resume rename 的交互（c3115 可复用）。

## 状态注记

> **designed**：proposal + design 已完成；未写代码、未绑分支、未改 live specs。apply 时按 design 的「实施任务」生成 tasks.md 并绑定分支。
