# Tasks: c1210-refactor-compose-bridge-llm

## 0. Propose（本阶段）

- [x] 0.1 feature 分支 `feat/c1210-refactor-compose-bridge-llm`
- [x] 0.2 proposal / design / tasks
- [x] 0.3 改 live specs + features（dm3/dm6、be4/be6、pab3/pab4、pa20、la25、as45）
- [x] 0.4 `llman sdd change attach`；`validate c1210… --strict --no-check` 绿

## 1. 类型组合（apply）

- [ ] 1.1 domain 依赖 bridge DTO；`AgentMessage::Llm(AiBridgeMessage)`；删除平行 `LlmMessage`（或一期 `pub use` 后删调用点）
- [ ] 1.2 `project_for_llm` → `Vec<AiBridgeMessage>`；Llm passthrough；Env 折叠
- [ ] 1.3 收缩 `map.rs` 消息孪生；`token_estimator` 去重
- [ ] 1.4 更新 `src/AGENTS.md` Provider 适配 + `_TODO` §F

## 2. Session bash + 上下文

- [ ] 2.1 `append_bash` / `record_bash_result` → `Message` + `role:bashExecution`
- [ ] 2.2 读路径：顶层 `BashExecution` → 提升为 Message+Env
- [ ] 2.3 统一 entry→AgentMessage；ReAct/`as45` 播种走该路径；`exclude_from_context`（be6）
- [ ] 2.4 可选：序列化层停止写出顶层 bash 变体（enum 可留读兼容）

## 3. 验证

- [ ] 3.1 单测：project / serde / 旧 JSONL lift / 新 bang 落盘形状
- [ ] 3.2 相关 BDD：`cargo test --test bdd -- --test-threads=1`（触及场景）
- [ ] 3.3 `just qa`；`llman sdd change finalize c1210-refactor-compose-bridge-llm`
