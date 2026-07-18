# Tasks: c1270-update-ai-bridge-responses-input-align-pi

- [x] 1. Options：`XyGenerateOptions` / `AiBridgeGenerateOptions` 增加 `system_prompt: Option<String>`；infra `to_bridge_options` 透传；ReAct 构造 options 时填入，删除 history `user(system)` 注入
- [x] 2. Responses：`build_body` 前置 developer/system；assistant convert 仅 Text→output_text；有 thinkingSignature 则推 reasoning item，否则省略 Thinking
- [x] 3. Completions / Anthropic：经 options 注入正式 system 通道
- [x] 4. live specs：`pab15` + feature 场景；`ar22`（system 不入 history 首条 user）
- [x] 5. 单测 + BDD；`just lint` / 相关 `cargo test`；`llman sdd validate`
- [x] 6. attach → checkpoint → archive（干净树）
