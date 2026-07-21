# Tasks: c1220-refactor-fuse-domain-runtime-protocol

## 0. Propose（本阶段）

- [x] 0.1 feature 分支 `feat/c1220-refactor-fuse-domain-runtime-protocol`
- [x] 0.2 proposal / design / tasks（status: full）
- [x] 0.3 改 live specs + features（layer-architecture、domain-message、infra-provider、…）
- [x] 0.4 `llman sdd change attach`；`validate c1220… --no-check` 绿

## 1. protocol 三子树 + 收拢 ports

- [ ] 1.1 建立 `src/protocol/{wire,ports,vocab}/`；迁入既有 command/event/transport → wire
- [ ] 1.2 将 `src/runtime_protocol/*` 迁入 `protocol/ports/`；全仓改 import；删 `runtime_protocol` 顶栏
- [ ] 1.3 更新 `src/AGENTS.md` / 根 AGENTS：三子树纪律 + 依赖图

## 2. XyModel 只吃 LLM DTO

- [ ] 2.1 `XyModel::generate_stream` / `generate` 入参改为 `Vec<AiBridgeMessage>`（或 `LlmMessage` 别名）
- [ ] 2.2 agent ReAct / compaction：调用前 `project_for_llm`；infra adapter 去掉 AgentMessage 入口
- [ ] 2.3 收缩/删除 `infra/provider/map.rs` 中对 AgentMessage 的路径

## 3. 消 domain：词汇进 protocol/vocab

- [ ] 3.1 迁 `message` / Env → `protocol/vocab/`；`project_for_llm` → `agent/`；agent `pub use` 消息类型
- [ ] 3.2 迁 `session_types` → `protocol/vocab/session.rs`；`lifecycle`（XyEvent）→ `protocol/vocab/lifecycle.rs`
- [ ] 3.3 迁其余 domain 模块按 design 落点；删 `src/domain/`
- [ ] 3.4 crate 根 `pub use` 与 embed 缝更新

## 4. 验证

- [ ] 4.1 `cargo test --lib` + 相关单测；`cargo test --test bdd -- --test-threads=1`
- [ ] 4.2 `just qa`；`llman sdd change finalize c1220-refactor-fuse-domain-runtime-protocol`
