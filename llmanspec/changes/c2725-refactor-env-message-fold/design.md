# Design: EnvMessage fold

> Designed / pre-start。无 `depends_on`。

## 1. 目标

对 `EnvMessage` 的语义问题（能否进 LLM、如何投影成 user/assistant 文本、是否 exclude）只在 **一处** match。

## 2. 代码事实

今日至少：

- `src/protocol/message.rs`：`role_name` / `text` / 若干构造
- `src/agent/llm_project.rs`：投影 bash/custom/compaction/branch
- `src/protocol/session/helpers.rs`、`session/mod.rs`：persist bash
- `src/app/core/bang_exec.rs`：bang 结果是否当 Env bash
- `src/agent/prompt/session_env.rs`：custom session_env 折成 user

收口优先放 `protocol::message` impl，因 wire 形状属 protocol；`llm_project` 只做 LLM 方言所需的 `LlmMessage` 转换，调用 message 上的「env 视图」。

## 3. 建议 API（实现时可改名，须穷举）

- `fn llm_projection(&self) -> Option<LlmView>`：`None` = 不送模型
- `fn exclude_from_context(&self) -> bool`（bash 字段；其它变体固定 false 或显式）
- 展示字符串继续 `text()`，避免第三套

新增变体时编译器逼改这些 match。

## 4. 验证

`llm_project` 现有单测平移；bang / session 测不改期望字符串。禁止为「兼容」保留旧 match 与 helper 双路径。
