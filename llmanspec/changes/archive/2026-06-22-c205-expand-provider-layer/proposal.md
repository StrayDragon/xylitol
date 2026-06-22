---
id: c205-expand-provider-layer
title: "Expand Provider Layer — Amazon Bedrock, Google/Vertex, GitHub Copilot, OAuth, usage tracking"
depends_on: [c170-refactor-agent-message-types, c185-upgrade-agent-loop]
---

## Why

当前 provider 层仅支持 OpenAI 和 Anthropic，缺少 pi 支持的其他关键 provider：

1. **缺失 Amazon Bedrock**：通过 AWS SigV4 认证访问 Anthropic/Amazon 模型
2. **缺失 Google/Vertex AI**：Gemini 模型 + 图片支持 + 思考签名验证
3. **缺失 GitHub Copilot**：OpenAI Responses API 模式的 Copilot 集成
4. **缺失 Azure OpenAI**：Azure 托管的 OpenAI 端点
5. **缺失 OAuth 流程**：GitHub Copilot 和 OpenRouter 的 OAuth 设备码流程
6. **缺失每个消息的使用量跟踪**：provider 返回的 token 使用量未记录到消息中
7. **缺失 provider attribution**：多个 provider 返回消息时标明来源

## What Changes

1. **Amazon Bedrock**：实现 AWS SigV4 签名、Converse API、模型 ID 解析
2. **Google/Vertex AI**：Gemini API、思考内容块、签名验证
3. **GitHub Copilot**：OpenAI Responses API 兼容层 + OAuth 设备码流程
4. **Azure OpenAI**：Azure 端点 + 自定义域名配置
5. **OAuth 流程**：设备码授权 + token 刷新存储
6. **使用量跟踪**：从 provider 响应中提取 usage 字段 → 填充到 AssistantMessage.usage
7. **Provider attribution**：`ProviderAttribution` 结构体，标明消息来源 provider

## Capabilities

- provider-integration

## Impact

- `src/agent/provider/`：新增 bedrock.rs、google.rs、vertex.rs、copilot.rs、azure.rs、oauth.rs
- `src/agent/provider/mod.rs`：注册新 provider
- `src/agent/types.rs`：添加 Usage 字段到 AssistantMessage
- `src/agent/registry.rs`：ModelRegistry 支持 OAuth 认证发现
- `src/agent/auth_storage.rs`：OAuth token 持久化

## Definition of Done

- [ ] Amazon Bedrock provider 实现（至少一个模型测试通过）
- [ ] Google/Vertex AI provider 实现（至少一个模型测试通过）
- [ ] GitHub Copilot provider 实现（使用 OpenAI Responses API）
- [ ] Azure OpenAI provider 实现
- [ ] OAuth 设备码流程 + token 存储
- [ ] 使用量跟踪：每个 AssistantMessage 携带 usage 信息
- [ ] Provider attribution：消息来源可追溯
- [ ] `cargo test` 通过
