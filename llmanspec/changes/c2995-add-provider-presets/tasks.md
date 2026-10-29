# Tasks

- [ ] 1. **sdk**（model_entry）：`XyModelEntryConfig` 增加 `preset: Option<String>`（`#[serde(default)]`，保持 `deny_unknown_fields`）；新增 `XyModelEntryConfig::preset_id()` 读取入口。测：`cargo test -p xylitol-plugin-sdk`。
- [ ] 2. **config**（presets）：新增 `presets.rs` 内置目录（`ProviderPreset`：`zmodel` / `opencode-zen` / `clinepass` / `commandcode`），声明 `kind / api / compat / base_url / default_model / thinking / thinking_levels / context_window`；不携带密钥。测：`cargo test -p xylitol-official-plugin-config presets`。
- [ ] 3. **config**（resolve，blocked-by: 1,2）：`ModelsConfig::resolve_model_config` / `resolve_model_meta` 先展开预设默认值，再用显式字段覆盖；`api_key` 只取用户配置；未知 `preset` 返回可读诊断（`unknown preset: <id>`），不得静默回落裸档案。测：`cargo test -p xylitol-official-plugin-config`。
- [ ] 4. **schema / example**（blocked-by: 1）：`just gen-config-schema` 刷新 `configs/config.schema.json`；`scripts/gen_config_example.py` 模板增加预设注释示例；文档列预设名、兼容族、`api`、`compat`、默认模型与已知可用基线。测：config crate 的 schema 漂移单测。
- [ ] 5. **specs**（runtime-config / infra-provider）：`runtime-config` 新增预设解析规则（优先级、未知诊断、密钥来源）；`infra-provider` 补预设到 adapter 选择的映射；补场景。测：`llman-sdd validate c2995-add-provider-presets`。
- [ ] 6. **tests**（repo）：预设默认 + 显式覆盖、未知预设诊断、密钥不内置的 BDD/单测。测：`cargo test --all-features --test bdd`。
- [ ] 7. **门禁**（repo）：`just qa` 全绿；收口 PR body 含 `fixed #20`。测：`just qa`。
