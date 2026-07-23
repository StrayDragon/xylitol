# Tasks: c1605

- [x] 1.1 `agent/prompt/fragments.rs`：id/body 表 + `fragments_for_batch_mode`
- [x] 1.2 `SystemPromptOpts` + `build_system_prompt` 注入 `<runtime_policy>`
- [x] 1.3 Session 构造/`set_tool_mode` 同步 fragments 并 rebuild
- [x] 1.4 单测：BarrierParallel 含片段；Sequential 不含；APPEND 可并存
- [x] 1.5 live specs：`agent-prompt` pt10（单测覆盖）
