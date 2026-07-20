# Tasks: c1380-add-cli-tokenizer-cache

## 1. Bridge 缓存 + HF 基址

- [x] 1.1 `HfTokenizerCache`：`list_entries` / `remove` / `cache_root`；download 原子写入
- [x] 1.2 `hf_endpoint_base()`：`HF_ENDPOINT` → `HF_HUB_ENDPOINT` → 默认官方域；`build_hf_resolve_url`
- [x] 1.3 单测：列举/清理；URL 前缀随 `HF_ENDPOINT`；默认官方域
- [x] 1.4 估计路径不调用 download（仍走 encode_count_if_cached only）

## 2. 配置与 Registry

- [x] 2.1 顶层 `tokenizers:` + `ModelEntry.tokenizer: string`（短写 / 共享名）
- [x] 2.2 `resolve_tokenizer_ref` + `resolve_tokenizer_with_override`
- [x] 2.3 example.yaml 按 Qwen 推理 id ≠ 词表 repo 示例
- [x] 2.4 runtime-config **rc18**（共享表 + 短写）
- [x] 2.5 Driver estimate 注入 override（`InProcessDriver` ← `AppConfig.tokenizer_override_for`）

## 3. CLI 子命令

- [x] 3.1 `CliCommand::Tokenizer` + `tokenizer.rs`
- [x] 3.2 早退 dispatch
- [x] 3.3 TTY 确认摘要含 repo/file/hf_base/dest；`--yes`
- [x] 3.4 unmapped 错误提示
- [x] 3.5 单测 `run_with`

## 4. Specs / BDD

- [x] 4.1–4.3 live specs（ce15 / paa8–9 / rc18）
- [x] 4.4 BDD step 实现（`tests/bdd.rs`：ce15×6 / paa8–9 / rc18）
- [x] 4.5 `llman sdd validate` change 级（`--no-check` 快闸）

## 5. 收尾

- [x] 5.1 相关单测绿；BDD 场景绿（ce15/paa8–9/rc18）

下一步（不计入本 tasks 勾选，走 verify → finalize）：

- roadmap 收缩（`finalize` / archive 后）
- `llman sdd change finalize c1380-add-cli-tokenizer-cache` + 一次 git commit

## 显式不在本 change

- revision / `HF_TOKEN` / model card 发现
- c1390 `tui`/`print`
- TUI/Web 同意流
