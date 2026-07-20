# Tasks: c1420-align-compact-estimate-gate-local-tokenizer

## 0. 分支与合约

- [x] 0.1 `git checkout -b feat/c1420-align-compact-estimate-gate-local-tokenizer`
- [x] 0.2 升 proposal `status: full`；live 改 domain-compaction / package-ai-bridge-accounting / agent-runtime / runtime-config
- [x] 0.3 `llman sdd change attach c1420-…`；live specs `validate --strict --no-check` 绿

## 1. 先修：流式 usage 落盘（Api 锚点前提）

- [x] 1.0 查实：bridge 已传 `XyChunk::Done.usage`；`react.rs` 忽略并硬编码 `usage: None`
- [x] 1.1 流循环捕获 `Done { finish_reason, usage }`；持久化 `AssistantMessage` 写入二者
- [x] 1.2 单测：`test_persist_done_usage`；estimate Api 锚点单测

## 2. LocalTokenizer 闸（默认 off）

- [x] 2.1 `EstimateOpts.allow_local_tokenizer: bool`（默认 `false`）；为 false 时不注入 `tokenizer_estimate`
- [x] 2.2 AppConfig YAML：`token_estimate.local_tokenizer: on|off`（默认 off）接到 Driver / estimate
- [x] 2.3 单测 + rc19 BDD：`off` 不得 LocalTokenizer；非法值加载失败

## 3. Compact 同源估计

- [x] 3.1 `maybe_auto_compact` 用 `estimate_from_session_entries`，去掉阈值路径 `len/4` sum
- [x] 3.2 cut_detector / `tokens_before` 仍 heuristic；design + architecture 注明
- [x] 3.3 单测：有 Api usage 时同源估计 provenance=Api；Driver compact 注入同源 opts

## 4. 文档与校验

- [x] 4.1 `docs/architecture/压缩与上下文.md` 已更新
- [x] 4.2 `just qa` 绿
- [x] 4.3 `change finalize` 单 commit
