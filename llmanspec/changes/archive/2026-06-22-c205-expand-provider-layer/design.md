# Design: c205-expand-provider-layer

## Provider Registration Pattern

```rust
// Registry pattern: each provider registers a factory function
ModelRegistry::register_provider("amazon-bedrock", BedrockProvider::factory);
ModelRegistry::register_provider("google", GoogleProvider::factory);
ModelRegistry::register_provider("vertex", VertexProvider::factory);
ModelRegistry::register_provider("github-copilot", CopilotProvider::factory);
ModelRegistry::register_provider("azure", AzureProvider::factory);
```

## OAuth Flow

```
User runs `/login {provider}`
  1. Initiate Device Authorization flow
  2. Display user_code + verification_uri
  3. Poll for token
  4. Store token securely in auth_storage
  5. Token refresh on expiry (intercept 401 responses)
```

## Usage Tracking

```rust
pub struct Usage {
    pub input: u64,
    pub output: u64,
    pub cache_read: u64,
    pub cache_write: u64,
    pub total_tokens: u64,
}
```
