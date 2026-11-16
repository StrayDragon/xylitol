---
depends_on: []
title: "Core explicit context run"
---

# Tasks — c3344 核心显式上下文运行

- [x] T1 `TurnRequest` 与 Driver 默认入口：新增 `TurnRequest`（parts/context/system_prompt、构造器、`prompt_text`），`XyDriver::run_turn` 返回 `Result<EventStream, XyDriverError>`；纯 prompt 默认回退 `run`，带 context/system 返回 `Unsupported`；`xylitol::embed` 再导出；单测覆盖三种情况与 `TurnRequest::text` 等价。
- [x] T2 AgentRuntime 种子历史：新增 `submit_turn` / 内部 seeded 提交，`start_root_stream` 接受 `Option<Vec<AgentMessage>>`，`freeze_root_config` 接受 system 覆盖参数（不改 `FrozenRootConfig` 字段）；单测覆盖 seed 不写 session、本轮 user/assistant 落盘、system 不写成 user。blocked_by: [T1]
- [x] T3 InProcessDriver 转换：覆盖 `run_turn`，把 `context: Vec<XyLlmMessage>` 转 `Vec<AgentMessage::Llm>`，保留 session 绑定、capability overlay、tool table 冻结；单测覆盖空 context 等价 `run`、非空 context 传给 `submit_turn`。blocked_by: [T1, T2]
- [x] T4 BDD 绑定：为 `agent-run-context` 的 4 个场景补 step definitions，运行 `cargo nextest run --all-features --test bdd`；场景必须与 spec 文本精确匹配。blocked_by: [T2, T3]
- [x] T5 回归与门禁：`cargo fmt --check`、`cargo clippy --quiet`、相关 runtime/agent 测试、`llman-sdd validate c3344-add-core-explicit-context-run --strict`；确认 `run`、`run_with_context`、`submit_parts` 旧行为不变。blocked_by: [T4]
