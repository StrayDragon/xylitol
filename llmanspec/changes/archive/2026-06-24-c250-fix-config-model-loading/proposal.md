---
depends_on: []
---

# c250-fix-config-model-loading

## Why

用户报告:"似乎没用 .xylitol 配置,默认还是 gpt5.4"。调查发现"配置里的模型选择
**整体不生效**",由三个独立缺陷叠加导致:

### 缺陷 1:兜底默认值是错误占位符(P0)

`src/agent/model/registry.rs:90`:

```rust
const DEFAULT_MODEL_PER_PROVIDER: &[(&str, &str)] =
    &[("openai", "gpt-5.4"), ("anthropic", "claude-opus-4-8")];
```

`gpt-5.4` 和 `claude-opus-4-8` 都是不存在的模型。一旦走到兜底,必然 API 报错。
单元测试 `registry.rs:436` 甚至断言它就是 `gpt-5.4`(锁死了错误值)。

### 缺陷 2:配置的默认模型从不被使用(P0)

`resolve_profile()`(`types.rs:239-258`)**已实现**完整的 model 优先级解析:

```rust
let model_id = model_ref                      // agents.profiles.<default>.model
    .or(self.execution.model.as_deref())      // execution.model
    .or(self.model.default_model.as_deref())  // models.default_model
    .ok_or_else(|| "no model configured...")?;
```

但 `cli/mod.rs` 调用 `resolve_default_profile()` 时**只取了 system_prompt 和
max_iterations,完全忽略返回的 model_id**。`--model` 未传时,从不自动选配置里
的默认模型。即 `default_model: qwen` / `agents.profiles.default.model: qwen` 都是
死配置。

### 缺陷 3:配置加载错误被静默吞掉(P1)

`AppConfig` 有 struct 级 `#[serde(default)]`(`types.rs:16`)但**无
`#[serde(deny_unknown_fields)]`**。用户把 `models:` 误写成 `model:`(单数)时,
serde **静默忽略**该 key → `models` 用 Default(空)→ qwen 条目丢失 → registry 为空
→ 静默走环境变量兜底 → `gpt-5.4`。全程无任何提示。

### 用户实际配置(`.xylitol/config.local.yaml`)

```yaml
model:              # ← 应为 models(复数),被静默忽略
  default_model: qwen
  models:
    qwen:
      provider: openai
      model: Qwen3.6-35B-A3B/UD-Q5_K_XL-think-coding
      base_url: http://tufa:50256/v1
```

三个缺陷叠加 → qwen 丢失 → 死的 default_model → 兜底 gpt-5.4。

## What Changes

| 缺陷 | 修复 | spec |
|------|------|------|
| 1 兜底占位符 | `registry.rs:90` 改成真实模型(`gpt-4o` / `claude-sonnet-4-20250514`),更新对应断言 | modify `model-registry/m3` |
| 2 默认模型不生效 | `cli/mod.rs` 启动时若未 `--model`,用 `resolve_default_profile().model_id` 调 `select_model` | add `cli-entry/ce1` |
| 3 静默吞错 | `cli/mod.rs`:config 文件存在但 registry 加载到 0 个 model 时,报错指向配置文件,而非静默兜底 | add `cli-entry/ce2` |

### 不在本次范围(留 future)

- **本地无鉴权 provider 强制要 API key**:`resolve_api_key` 对 `ModelKind::OpenAi`
  强制 `OPENAI_API_KEY`,指向 `http://tufa:50256/v1` 的本地 provider 本不需要 key。
  当前因用户环境恰好有 key 暂不阻塞,属独立设计问题,留 `future.md`。
- **`deny_unknown_fields`**:全局加会破坏 `#[serde(default)]` 的前向兼容,风险大;
  本次用 ce2 的"加载后校验"替代,更稳妥。
- **settings manager 接入 cli**:`SettingsManager` 有独立 default_model 机制,与
  AppConfig 并行;对齐是更大重构,不在本次。

## Capabilities

- `model-registry`(modify m3)
- `cli-entry`(add ce1, ce2)

## Impact

- **修改**:`src/agent/model/registry.rs`(常量 + 测试)、`src/interface/cli/mod.rs`(启动选模型 + 加载校验)
- **零影响**:TUI、agent loop、session、tools、BDD(无配置相关 BDD feature)
- **用户可见**:配置里的默认模型终于生效;配置写错时给出明确报错
