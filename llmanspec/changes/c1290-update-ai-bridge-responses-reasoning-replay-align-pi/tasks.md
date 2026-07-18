# Tasks: c1290-update-ai-bridge-responses-reasoning-replay-align-pi

- [x] 1. Responses `build_body`：`store: false`；tools 每项 `strict: false`；thinking ≠ off 时 `reasoning.summary`（默认 `auto`）+ `include: ["reasoning.encrypted_content"]`；单测锁定 JSON
- [x] 2. 流式 / 非流式：reasoning item 完成 → 完整 JSON 写入 signature；新增 `ThinkingEnd`（或文档化等价）并映射 `AiBridgeChunk` → `XyChunk`
- [x] 3. ReAct：消费 signature，最终 `AgentPart::Thinking` 落盘；回放路径沿用 c1270（单测：有 signature → input 含 `type=reasoning`）
- [x] 4. `set_tools` / bootstrap：收集 `XyTool::prompt_guidelines` → `SystemPromptOpts`；bash/read/edit/write 等补齐短句；单测/组装断言
- [x] 5. live specs：`pab16` + feature；`pt9` + feature；必要时 agent-runtime 场景
- [x] 6. BDD + 相关 unit；`just lint`；`llman sdd validate`；attach →（实现后）checkpoint → archive
