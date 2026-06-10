---
llman_spec_valid_scope:
  - src/
  - tests/
llman_spec_valid_commands:
  - cargo test
llman_spec_evidence:
  - "Archived from change c10-add-config"
---

```toon
kind: llman.sdd.spec
name: "runtime-config"
purpose: "TBD - created by archiving change c10-add-config. Update purpose after archive."
requirements[17]{req_id,title,statement}:
  r1,"yaml-parse","System MUST parse YAML configuration from three layers (global/project/user) and deep-merge them with later layers overriding earlier ones."
  r2,"schema-gen","System MUST generate JSON Schema from Rust config types via schemars for IDE auto-completion."
  r3,"runtime-validate","System MUST validate merged config against JSON Schema at runtime and report human-readable errors."
  r4,"template-render","System MUST render {{ env.KEY }} and {{ secret.KEY }} template expressions in YAML files before parsing, using minijinja."
  r5,"secret-env",System MUST load secret.env dotenv files from each config location and inject their values as the secret.* namespace in template rendering.
  r6,"local-overlay","System MUST support an optional config.local.yaml overlay at global and project config locations, merged on top of the base config.yaml at the same location."
  r7,"config-dir-env",System MUST respect XYLITOL_CONFIG_DIR and XYLITOL_PROJECT_DIR environment variables for config directory resolution.
  r8,"agents-dir-discovery",System MUST discover .agents/ directory alongside .xylitol/ during project root detection and expose its path via ConfigPaths for downstream consumers.
  r9,"model-resolution","System MUST resolve the effective model ID from user-provided sources (CLI override, agent profile, execution config, model.default_model). System MUST NOT embed environment-specific local model IDs as code defaults. If no model is configured, system MUST return a clear error describing how to configure one."
  r10,"provider-tag","System MUST accept `openai` (not `open_a_i`) as the provider tag for OpenAI-compatible models in configuration. System MUST validate provider tags via JSON Schema and return a descriptive error for unknown values."
  rc1,"three-tier","Config MUST merge three tiers in priority order: user (~/.xylitol/config.yaml) > project (./.xylitol/config.yaml) > global (built-in defaults)."
  rc2,"model-config","Config MUST support model entries with: provider, model-id, base_url override, api_key env reference, thinking support, context_window."
  rc3,"provider-config","Config MUST support provider entries with: base_url, api_key, headers, api (openai-compatible or anthropic-messages)."
  rc4,settings,"Config MUST support settings: max_iterations, compaction_threshold, default_model, thinking_level, tools (allow/deny list)."
  rc5,"env-interpolation","Config MUST support ${ENV_VAR} and $ENV_VAR interpolation in api_key and base_url fields."
  rc6,"secret-backend",Config MUST support external secret resolution via !command prefix for api_key values.
  rc7,"bdd-config",BDD tests under tests/features/config.feature MUST all pass.
scenarios[22]{req_id,id,given,when,then}:
  r1,happy,a project config overrides model field,config is loaded,merged config contains project model value
  r2,happy,AppConfig struct is defined,"schemars::schema_for is called",valid JSON Schema is generated to configs/config.schema.json
  r3,happy,a config file has invalid field,config is loaded,a descriptive error is returned with field path
  r4,happy,"a config file contains {{ env.MODEL_NAME }} with MODEL_NAME set",config is loaded,rendered config contains the env var value
  r4,sad,"a config file contains {{ env.MISSING_KEY }} with no default",config is loaded,a descriptive template error is returned listing the missing key and file to edit
  r5,happy,".xylitol/secret.env contains ANTHROPIC_API_KEY=sk-xxx and config references {{ secret.ANTHROPIC_API_KEY }}",config is loaded,"rendered config contains sk-xxx"
  r6,happy,project config.local.yaml overrides model field,config is loaded,merged config contains local model value overriding base config
  r7,happy,XYLITOL_CONFIG_DIR=/custom/dir is set,config is loaded,global config loaded from /custom/dir/config.yaml
  r7,sad,XYLITOL_PROJECT_DIR is not set and no .xylitol/ directory found in CWD ancestry,config is loaded,"system proceeds with global config only (zero-config works)"
  r8,happy,project root contains .agents/ directory,project root is discovered,ConfigPaths.agents_dir is set to the .agents/ path
  r8,sad,project root has no .agents/ directory,project root is discovered,ConfigPaths.agents_dir is None and system proceeds normally
  r9,happy,a config provides model.default_model,a resolved profile is built,the model id resolves to model.default_model
  r9,sad,no model is configured in CLI or config,a resolved profile is built,a descriptive configuration error is returned
  r10,happy,config provider tag is openai,config is loaded,"provider is parsed as ProviderKind::OpenAI"
  r10,sad,config provider tag is invalid,config is loaded,a schema validation error is returned pointing to the provider field
  rc1,merge,"user config has default_model: gpt-4o and project config has default_model: claude",config is loaded,"default_model is gpt-4o (user wins)"
  rc2,"model-entry","a model entry with provider anthropic and model claude-sonnet-4-20250514",model is resolved,"provider and model-id are available"
  rc3,"provider-override","a provider entry overrides base_url to https://proxy.example.com",model builds client,requests go to the overridden URL
  rc4,settings,compaction_threshold is set to 0.7,agent session starts,compaction triggers at 70% context usage
  rc5,"env-interpolation","api_key: $OPENAI_API_KEY with OPENAI_API_KEY=sk-abc",config loads,"resolved api_key is sk-abc"
  rc6,"secret-command","api_key: !pass show api/openai",config loads,api_key is resolved from external command
  rc7,"bdd-pass",BDD runner invoked,"cargo test --test bdd",all config scenarios pass
```
