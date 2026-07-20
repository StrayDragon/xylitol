# Tasks: c1420-align-compact-estimate-gate-local-tokenizer

## 0. 分支与合约

- [x] 0.1 `git checkout -b feat/c1420-align-compact-estimate-gate-local-tokenizer`
- [x] 0.2 升 proposal `status: full`；live 改 domain-compaction / package-ai-bridge-accounting / agent-runtime / runtime-config
- [x] 0.3 `llman sdd change attach c1420-…`；live specs `validate --strict --no-check` 绿（change 实现 tasks 未勾前 `--strict` 会因 pending 失败，属预期）

## 1. 先修：流式 usage 落盘（Api 锚点前提）

- [x] 1.0 查实：bridge 已传 `XyChunk::Done.usage`；`react.rs` 忽略并硬编码 `usage: None`
- [ ] 1.1 流循环捕获 `Done { finish_reason, usage }`；持久化 `AssistantMessage` 写入二者
- [ ] 1.2 单测 / BDD：mock Done 带 usage → session 条目 `usage` 非空；estimate → provenance Api

## 2. LocalTokenizer 闸（默认 off）

- [ ] 2.1 `EstimateOpts.allow_local_tokenizer: bool`（默认 `false`）；为 false 时不注入 `tokenizer_estimate`
- [ ] 2.2 AppConfig YAML：`token_estimate.local_tokenizer: on|off`（默认 off）接到 Driver / estimate
- [ ] 2.3 单测：有映射但 `off` → provenance ≠ LocalTokenizer；`on` + 可算 → 可为 LocalTokenizer

## 3. Compact 同源估计

- [ ] 3.1 `maybe_auto_compact` 用与 footer 同入口估计，去掉阈值路径 `len/4` sum
- [ ] 3.2 评估 cut_detector / `tokens_before`：同波能共用则共用；否则文档注明偏差
- [ ] 3.3 BDD / 单测：有 Api usage 时阈值跟同源估计

## 4. 文档与校验

- [ ] 4.1 `docs/architecture/压缩与上下文.md`：footer↔compact 同源；local 默认 off；usage 落盘
- [ ] 4.2 `just qa` 绿
- [ ] 4.3 `change finalize` 单 commit（用户要求时）

## 明确不做

- Single-flight / 跨进程 IPC；every-N / idle；TUI tokenizer 下载确认
