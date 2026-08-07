---
depends_on: []
branch: sdd/update-live-provider-config-location
base_sha: 263c9aab6b25932b7ad1805e906b82e4068bc4cf
checkpointed: false
---

# live-provider 测试配置迁移到全局共享目录（`~/.config/xylitol/dev/`）

## Why

多机器开发时，仓库内 gitignored 的 `configs/testing/live-provider.local.yaml` 不会跨机同步：新机器 clone 后 `just qa` 的 live-provider 串行闸只能 skip，无法验证 prompt-cache 反例链路。而 `~/.config/xylitol/` 是用户自建 gitea（dotxylitol）同步的共享目录，把专用配置放进其 `dev/` 子目录即可让所有机器 clone 后开箱即用。

同时仓库遵守「代码仓库只能有自动生成的 example」：仓库内不再保留任何手工维护的 local 配置，示例由维护脚本生成。

## What Changes

- live-provider 配置唯一真源改为全局共享目录专用文件 `<global_dir>/dev/live-provider.yaml`；`global_dir` 解析与主 crate 一致（`XYLITOL_CONFIG_DIR` → `$XDG_CONFIG_HOME/xylitol` → `~/.config/xylitol`）。
- 解析优先级：`XYLITOL_LIVE_PROVIDER_CONFIG` env override（保留现有 escape hatch）→ 默认 `<global_dir>/dev/live-provider.yaml` → 缺失或 `enabled=false` 则 skip（qa 通过）。仓库内 `.local.yaml` 路径不再读取。
- `configs/testing/live-provider.local.yaml` 本机文件删除，`.gitignore` 对应条目删除。
- 示例 `configs/testing/live-provider.example.yaml` 保留，改为由维护脚本自动生成（新增 `scripts/gen_live_provider_example.py` + `just gen-live-provider-example`，不进 qa 闸）。
- 测试二进制 `packages/xylitol-ai-bridge/tests/live_responses_prompt_cache.rs` 与两个 lab example（`lab_resume_prompt_cache.rs` / `lab_session_prefix_idempotency.rs`）的 `load_cfg()` 同步迁移路径解析。
- spec `test-qa-gate` qg07 更新 MUST 级措辞：配置位置改为全局共享目录专用文件，并明确 MUST NOT 从全局 AppConfig `config.yaml` 合并 live-provider 参数（保留「测试条件固定」的原意图）。
- 文档指针同步：AGENTS.md 命令段、`docs/research/responses-context-layout-and-cache-2026.md`、justfile 注释。
- dotxylitol（`~/.config/xylitol`）新增 `dev/live-provider.yaml`（本机真配置照搬），其他机器 clone 即用。

## Capabilities

- `test-qa-gate`（qg07 行为合约变更）

## Impact

- `just qa` 行为：有全局配置的机器 RUN 验证，无则 SKIP（通过），与现状语义一致，仅路径来源变化。
- 维护 lab（不进 qa）：`lab_resume_prompt_cache` / `lab_session_prefix_idempotency` 配置来源同源迁移。
- 新机器零配置：clone dotxylitol 即获得 live-provider 配置。
