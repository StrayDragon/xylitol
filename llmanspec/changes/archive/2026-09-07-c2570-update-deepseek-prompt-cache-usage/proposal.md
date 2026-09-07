---
depends_on: []
branch: sdd/c2570-update-deepseek-prompt-cache-usage
base_sha: 4c883674fb1b17eb5147d8c794fcbf6a1d634991
checkpointed: true
checkpoint_sha: 4c883674fb1b17eb5147d8c794fcbf6a1d634991
---

# DeepSeek 方言诚实映射 Prompt Cache 读数

## Why

`compat=deepseek` 的 WirePolicy 把 `prompt_cache_usage` 设为 false，Responses 路径因此把 usage 标成 `PromptCacheRead::NotApplicable`，丢掉上游已经给出的缓存读数。2026-09 对官方 `deepseek-v4-flash` 的打网：Chat Completions 带 `prompt_cache_hit_tokens` 与 `prompt_tokens_details.cached_tokens`（同值）；Responses 带 `input_tokens_details.cached_tokens`。分叉共享树干时第二枝即可命中。继续当「方言没有缓存字段」会让 footer / 观测把真实命中说成不适用，违反用量诚实。

不能走 quick：外部可观测（DeepSeek 用量从「不适用」变为 Tokens(n)），且 `deepseek_profile_skips_encrypted_include_and_cache_assumptions` 钉死了旧轮廓。

## What Changes

- DeepSeek 命名轮廓：`prompt_cache_usage` 改为 true；仍禁止 encrypted include 与 `previous_response_id`
- Responses：沿用既有三态映射（`input_tokens_details.cached_tokens` → `Tokens(n)`）
- Completions：停止把 usage 硬写成 `Tokens(0)`；映射 `prompt_cache_hit_tokens`（优先）或 `prompt_tokens_details.cached_tokens`
- 单测锁轮廓与映射；不改 Assembler 布局、不改 session 格式

## Capabilities

- `package-ai-bridge`：新增 DeepSeek 缓存读数 MUST（pab28）；pab16/pab22 行为不变（include 仍按轮廓省略；三态仍禁止把缺字段当成命中 0）

## Impact

- YAML `compat: deepseek` 的官方 Responses / Completions 开始透出 cache 读数
- llama.cpp 等 generic 轮廓不受影响（本已 expects true）
- 不保证分叉命中率；只保证「上游报了数字就不要假装没有」

## Further Notes

打网记录（openai SDK，无密钥）：tufa 串行第二枝 `cached_tokens` 升温；DeepSeek 官方第二枝 hit=640/754 量级。稿首 session id 或刷新 `session_env` clock 会把命中打回 0——属会话树不变量，不在本 change。
