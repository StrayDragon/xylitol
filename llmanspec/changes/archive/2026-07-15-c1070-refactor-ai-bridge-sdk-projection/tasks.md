# Tasks — c1070-refactor-ai-bridge-sdk-projection

- [x] 1. Delta + design + tasks；`llman sdd validate c1070-refactor-ai-bridge-sdk-projection --no-interactive`
- [x] 2. Domain：`LlmMessage` + `EnvMessage` + `AgentMessage::{Llm,Env}`（untagged 保线格式）+ 投影 `project_for_llm -> Vec<LlmMessage>` + 单测
- [x] 3. 全仓调用点适配；bridge DTO 仅 LLM 角色；`infra` 显式 `LlmMessage → AiBridgeMessage`（无 JSON roundtrip）
- [x] 4. Completions → `async-openai` Client + middleware hooks
- [x] 5. Responses + input_tokens → 同一 Client
- [x] 6. Anthropic Messages + count_tokens → **保留 reqwest 兜底**（官方 Rust SDK 未成熟；DTO 已收窄，hooks 仍经可移植 HeaderBag）
- [x] 7. 同步根/`src`/bridge `AGENTS.md`（组合分层写入；实现落地后再跑满闸 qa）
- [x] 8. `just qa` 相关子集全绿（随 apply）
