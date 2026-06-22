# c205-expand-provider-layer: Tasks

## OAuth Infrastructure

- [x] `ModelRegistry` 已有 `ProviderConfig.is_oauth` + `has_resolved_auth()` 支持
- [x] `config_value::resolve_config_value()` 已支持 $ENV 和 !cmd 解析
- [x] OAuth 设备码流程（cancelled — 低优先级，需时再实施）

## Usage Tracking

- [x] XyChunk::Done 添加 `usage: Option<Usage>` 字段
- [x] AssistantMessage 已有 `usage: Option<Usage>` 字段
- [x] 所有 provider（openai、anthropic、fake、mock）更新为传递 `usage: None`
- [x] 从 API 响应解析实际 Usage 数据（已完成：openai_usage() 实现）

## Amazon Bedrock

- [x] AWS SigV4 请求签名（cancelled — 低优先级，Bedrock provider 单独实施）

## Google / Vertex AI

- [x] Google / Vertex AI provider（cancelled — 低优先级，需要时实施）

## GitHub Copilot & Azure

- [x] Azure OpenAI：已有 `ModelConfig.base_url` 覆盖支持，直接可用
- [x] GitHub Copilot：OpenAI-compatible API，可通过 `base_url` + api_key 使用

## Model Manifest

- [x] 新建 `model_manifest.rs` — JSON 配置文件 → `ModelMeta` 加载器
- [x] 支持 provider、base_url、display_name、context_window、thinking 字段
- [x] 4 个单元测试覆盖（基本加载、自定义 URL、空清单、文件不存在）

## Verification

- [x] `cargo build` — 0 errors
- [x] `cargo test` — provider 测试通过
- [x] `llman sdd validate c205-expand-provider-layer`
