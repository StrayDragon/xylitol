# Tasks: c1380-add-cli-tokenizer-cache

## 1. Bridge 缓存 + HF 基址

- [ ] 1.1 `HfTokenizerCache`：`list_entries` / `remove` / `cache_root`；download 原子写入（失败不留坏文件）
- [ ] 1.2 `hf_endpoint_base()`：`HF_ENDPOINT` → 可选 `HF_HUB_ENDPOINT` → 默认 `https://huggingface.co`；`build_hf_resolve_url(repo, file)`
- [ ] 1.3 单测：tempdir 列举/清理；URL 前缀随 `HF_ENDPOINT=https://hf-mirror.com` 变化；无 env 为官方域
- [ ] 1.4 估计路径仍不调用 download（回归 paa6）

## 2. 配置与 Registry

- [ ] 2.1 `ModelEntry.tokenizer` 最小字段（huggingface | local | builtin 互斥）+ schemars；非法组合加载失败
- [ ] 2.2 `resolve_tokenizer`：配置覆盖 → builtin 启发式 → None；单测覆盖
- [ ] 2.3 example / schema 片段：一份 HF repo 示例 + 注释 `HF_ENDPOINT`
- [ ] 2.4 runtime-config live spec：ModelEntry.tokenizer MUST 句（最小）

## 3. CLI 子命令

- [ ] 3.1 `CliCommand::Tokenizer` + `tokenizer.rs`（status / download / clean）
- [ ] 3.2 早退 dispatch；不启会话/MCP；可读配置以解析 `--model` 映射
- [ ] 3.3 TTY 确认摘要含 **repo/file、hf_base、落盘路径**；`--yes` 跳过
- [ ] 3.4 unmapped 错误提示如何写配置或传 `owner/repo`
- [ ] 3.5 单测：`run_with_cache` 风格

## 4. Specs / BDD

- [x] 4.1 `cli-entry` ce15 + feature 场景骨架
- [x] 4.2 `package-ai-bridge-accounting` paa8 + feature 骨架
- [x] 4.3 补强 ce15/paa8/paa9/rc18：`HF_ENDPOINT` 基址；配置映射优先；确认摘要含 base（live specs 已写）
- [ ] 4.4 `llman sdd validate` 相关 cap + change（`--no-check` 结构 → full）

## 5. 收尾

- [ ] 5.1 `just test` / 相关 BDD 绿
- [ ] 5.2 roadmap Tokenizer：M1 兑现后按 `docs/AGENTS.md` 收缩（**归档后再迁 architecture**）
- [ ] 5.3 `llman sdd change finalize c1380-add-cli-tokenizer-cache`

## 显式不在本 change（待完善清单）

- [ ] （另）revision / `HF_TOKEN` / model card 自动发现
- [ ] （另）c1390 CLI 表面 `tui` / `print`
- [ ] （另）TUI/Web 同意流；配置预声明跳过确认
