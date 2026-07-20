# Tasks: c1420-align-compact-estimate-gate-local-tokenizer

> purpose-draft：勾选在升 `full` / feature 分支 attach 后实施。

## 0. 分支与合约

- [ ] 0.1 `git checkout -b feat/c1420-align-compact-estimate-gate-local-tokenizer`
- [ ] 0.2 升 proposal `status: full`；live 改 `domain-compaction` / `package-ai-bridge-accounting`（+ 配置键若需要）
- [ ] 0.3 `llman sdd change attach c1420-…`；`validate --strict`

## 1. 先修：流式 usage 落盘（Api 锚点前提）

- [ ] 1.0 查实：bridge 已传 `XyChunk::Done.usage`；`react.rs` 忽略并硬编码 `usage: None` → session 无 usage（2026-07-20 已确认）
- [ ] 1.1 流循环捕获 `Done { finish_reason, usage }`；持久化 `AssistantMessage` 写入二者（`partial_assistant_message` / MessageEnd 若需展示可跟）
- [ ] 1.2 单测 / BDD：mock Done 带 usage → session 条目反序列化后 `usage` 非空；estimate → provenance Api

## 2. LocalTokenizer 闸（默认 off）

- [ ] 2.1 `EstimateOpts.allow_local_tokenizer: bool`（默认 `false`）；为 false 时不注入 `tokenizer_estimate`
- [ ] 2.2 AppConfig YAML：`token_estimate.local_tokenizer: on|off`（默认 off）接到 Driver / estimate 调用点
- [ ] 2.3 单测：有 HF/builtin 映射但 `off` → provenance ≠ LocalTokenizer；`on` + 可算 → 可为 LocalTokenizer

## 3. Compact 同源估计

- [ ] 3.1 `maybe_auto_compact` 用与 footer 同入口的估计（messages + last_usage + model_id + override + local 闸），去掉阈值路径上的 `len/4` sum
- [ ] 3.2 评估 cut_detector / `tokens_before`：同波能共用则共用；否则文档注明「阈值同源、切点仍 heuristic」并开 follow-up
- [ ] 3.3 BDD / 单测：有可信 Api usage 时 compact 阈值决策跟 Api 数字（或同估计结果），不跟独立 len/4

## 4. 文档与校验

- [ ] 4.1 `docs/architecture/压缩与上下文.md`：写明 footer↔compact 同源；local 默认 off；usage 落盘
- [ ] 4.2 `just qa`（或至少相关 nextest + bdd）绿
- [ ] 4.3 `change finalize` 单 commit（用户要求时）

## 明确不做（本 change）

- Single-flight / 跨进程 IPC 丢弃旧结果
- every-N / idle 等 tokenizer 策略
- TUI tokenizer 下载确认
