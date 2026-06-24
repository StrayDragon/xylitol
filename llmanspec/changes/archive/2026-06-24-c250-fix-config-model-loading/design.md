# c250-fix-config-model-loading — Design

## Context

三个缺陷叠加导致"配置里的模型选择不生效"。本设计分别给出最小修复路径。

## 修复 1:兜底默认值(model-registry/m3)

### 现状

```rust
// registry.rs:90
const DEFAULT_MODEL_PER_PROVIDER: &[(&str, &str)] =
    &[("openai", "gpt-5.4"), ("anthropic", "claude-opus-4-8")];
```

测试 `registry.rs:436-441` 断言这俩值。

### 修复

```rust
const DEFAULT_MODEL_PER_PROVIDER: &[(&str, &str)] =
    &[("openai", "gpt-4o"), ("anthropic", "claude-sonnet-4-20250514")];
```

同步更新 `registry.rs:436-441` 的断言。选择依据:`gpt-4o` 和
`claude-sonnet-4-20250514` 是 OpenAI / Anthropic 当前主力且稳定可用的模型,
也是 `configs/example.yaml` 里示范的值。

## 修复 2:启动自动选配置默认模型(cli-entry/ce1)

### 现状

`cli/mod.rs` 用 `resolve_default_profile()` 只取了两个字段:

```rust
let system_prompt = app_config.as_ref()
    .and_then(|cfg| cfg.resolve_default_profile().ok())
    .and_then(|p| p.system_prompt) ...;
let max_iterations = ... .map(|p| p.max_iterations) ...;
// ❌ 从不读 p.model_id
```

然后只在 `--model` 时 `select_model`:

```rust
if let Some(ref mid) = args.model {
    match resolver::resolve_model(mid, &available, None) {
        Ok(resolved) => { let _ = agent_session.select_model(&resolved.model.id); }
        ...
    }
}
```

### 修复

`resolve_default_profile()` 已返回含 `model_id` 的 `ResolvedProfile`(types.rs:255-258
已实现优先级)。cli 启动时,若未传 `--model`,用 resolved 的 model_id 自动选:

```rust
// 解析一次,复用
let resolved_profile = app_config.as_ref()
    .and_then(|cfg| cfg.resolve_default_profile().ok());

let system_prompt = resolved_profile.as_ref().and_then(|p| p.system_prompt.clone());
let max_iterations = resolved_profile.as_ref().map(|p| p.max_iterations).unwrap_or(50);

// ... 构建 agent_session ...

// 模型选择:--model 优先,否则用 resolved profile 的默认模型
let target_model: Option<&str> = args.model.as_deref()
    .or(resolved_profile.as_ref().and_then(|p| p.model_id.as_deref()));

match target_model {
    Some(mid) => {
        match resolver::resolve_model(mid, &available, None) {
            Ok(resolved) => {
                if let Some(ref warning) = resolved.warning { eprintln!("Warning: {warning}"); }
                let _ = agent_session.select_model(&resolved.model.id);
            }
            Err(msg) => eprintln!("Warning: {}\n{}", msg, auth::format_no_model_selected_message()),
        }
    }
    None => { /* 无 --model 也无配置默认:沿用 registry 顺序或环境兜底,保持现状 */ }
}
```

注意:`ResolvedProfile.model_id` 字段名需在实现时核对(`core/model` 中定义);
若字段名不同,按实际调整。`resolve_profile` 的 model 优先级(profile > execution >
default_model)已在 types.rs:255-258 正确实现,无需改动。

## 修复 3:加载到 0 model 时报错(cli-entry/ce2)

### 现状

```rust
// cli/mod.rs 兜底逻辑
if model_registry.is_empty() {
    // 走环境变量发现 openai/anthropic,用 default_model_id_for_provider
    // → gpt-5.4(缺陷1)
}
if model_registry.is_empty() {
    eprintln!("Error: {}", auth::format_no_models_available_message());
    return Err("no models available".into());
}
```

问题:当**配置文件存在但解析出 0 个 model**(key 写错)时,这段走环境变量兜底,
不报配置错误。用户根本不知道配置没生效。

### 修复

区分两种"registry 空"的情况:

```rust
let config_present = app_config.is_some()
    && app_config.as_ref().is_some_and(|c| !c.model.models.is_empty()
        || c.model.default_model.is_some());

// 构建 registry 后:
if model_registry.is_empty() && config_present_but_empty {
    // 配置文件存在、声明了 models、但一个都没加载成功 → 大概率拼写/key 错误
    return Err(
        "config file present but no models loaded. \
         Check the top-level key is 'models:' (plural), \
         and each entry has provider+model. See configs/example.yaml."
    );
}
if model_registry.is_empty() {
    // 无配置,环境变量也没 → 原有报错
    eprintln!("Error: {}", auth::format_no_models_available_message());
    return Err("no models available".into());
}
```

实现细节("config 声明了 models 但加载失败"的判定)在 apply 时细化:可比较
`app_config.model.models.len()` 与最终 registry 中来自配置的 model 数。

## 测试策略

### 单元测试

- `registry.rs`:更新 `default_model_id_for_provider` 断言为新值,加一条"非占位符"
  断言(如 `assert!(id != "gpt-5.4")` 泛化保护)。

### 集成/手动验证

```bash
# 1. 修复用户的 .xylitol/config.local.yaml(model: → models:)后:
./target/debug/xylitol --list-models    # 应显示 qwen,不是 gpt-4o
./target/debug/xylitol --print "hi"     # 应用 qwen + 本地 base_url

# 2. 故意写错 key(model: 单数):
./target/debug/xylitol --list-models    # 应报错指向配置,而非静默 gpt-4o

# 3. 无 --model,配置有 default_model:
./target/debug/xylitol --print "hi"     # 自动用配置默认,不报 "no model selected"
```

### 回归

```bash
cargo test --lib                              # 含 registry 测试
cargo test --test bdd -- --test-threads=1     # 无配置 BDD,须保持通过
just fmt && just lint && just test
```
