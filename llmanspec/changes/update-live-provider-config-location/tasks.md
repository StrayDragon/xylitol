---
depends_on: []
---

# Tasks: update-live-provider-config-location

## 测试边界（seam）

- **路径解析 seam**：`resolve_live_provider` 的路径解析抽成可注入 `global_dir` 的纯函数（`resolve_global_dir` + `resolve_config_path`），在 `packages/xylitol-ai-bridge/tests/live_responses_prompt_cache.rs` 内加 `#[test]` 单测：env override 优先 / 默认全局路径 / 缺失 → Skip / `enabled=false` → Skip / `enabled=true` → Run。
- **BDD seam**：无新 `.feature` 文件；qg07 为 `feature:false` 文档行，仅更新 when/then 描述。
- **集成验证**：`just test-live-provider`（本机有全局配置 → RUN；无 → SKIP 通过）+ `llman sdd validate --all --strict`。

## Tasks

- [x] 1. 测试二进制路径迁移：`packages/xylitol-ai-bridge/tests/live_responses_prompt_cache.rs` 默认路径改为 `<global_dir>/dev/live-provider.yaml`（解析优先级 XYLITOL_CONFIG_DIR → XDG_CONFIG_HOME/xylitol → ~/.config/xylitol）；`XYLITOL_LIVE_PROVIDER_CONFIG` env override 保留；SKIP 文案指向全局路径；抽纯函数并加单测（seam 上表）。
- [x] 2. lab 同源迁移：`packages/xylitol-ai-bridge/examples/lab_resume_prompt_cache.rs` 与 `lab_session_prefix_idempotency.rs` 的 `load_cfg()` 默认路径同步为全局 `dev/live-provider.yaml`，env override 保留；两 example 的 doc 注释更新。
- [x] 3. example 自动生成：新增 `scripts/gen_live_provider_example.py`（维护脚本，不进 qa）+ `just gen-live-provider-example` recipe；运行生成/覆盖 `configs/testing/live-provider.example.yaml`（字段与 LiveProviderFile 对齐：enabled/base_url/model/api_key/max_output_tokens/serial），脚本自校验 YAML 可解析。
- [x] 4. 仓库清理：删除本机 `configs/testing/live-provider.local.yaml`；删除 `.gitignore` 中 `configs/testing/live-provider.local.yaml` 条目；检查 `.config/nextest.toml` 注释是否需要同步。
- [x] 5. 文档指针：根 `AGENTS.md` 命令段（live-provider 专用配置路径）、`docs/research/responses-context-layout-and-cache-2026.md`、`justfile` test-live-provider 注释同步新路径与生成命令。
- [x] 6. 门禁验证：`just fmt` / `just lint` / `just test-live-provider`（本机 RUN 或 SKIP 语义正确）/ `llman sdd validate --all --strict` 全绿。

## 仓库外（用户已授权，apply 完成后执行）

- dotxylitol（`~/.config/xylitol`）：新增 `dev/live-provider.yaml`（内容照搬现有 local 配置：enabled/base_url/model/api_key/max_output_tokens/serial），提交到 dotxylitol 仓库，实现多机同步。
