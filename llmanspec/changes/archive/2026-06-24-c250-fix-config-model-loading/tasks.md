# c250-fix-config-model-loading — Tasks

- [x] `registry.rs:90`:`DEFAULT_MODEL_PER_PROVIDER` 的 `gpt-5.4`/`claude-opus-4-8` 改为真实模型(`gpt-4o`/`claude-sonnet-4-20250514`),同步更新 `registry.rs:436-441` 断言
- [x] `cli/mod.rs`:启动时若未传 `--model`,用 `resolve_default_profile()` 的 `model_id` 调 `select_model`(核对 `ResolvedProfile.model_id` 实际字段名)
- [x] `cli/mod.rs`:config 文件存在但 registry 加载到 0 个 model 时,报错指向配置文件(而非静默走环境变量兜底)
- [x] 验证编译:`cargo build`
- [x] 单元测试:`cargo test --lib -- model::registry`
- [x] BDD 回归:`cargo test --test bdd -- --test-threads=1`
- [x] 手动验证:修复 `.xylitol/config.local.yaml`(`model:`→`models:`)后 `--list-models` 显示 qwen;故意写错 key 时报错指向配置
- [x] `just fmt && just lint && just test`
- [x] `llman sdd validate c250-fix-config-model-loading --strict --no-interactive`
