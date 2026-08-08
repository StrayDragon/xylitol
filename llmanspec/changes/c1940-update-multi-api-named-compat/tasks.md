# Tasks: c1940（三协议 × named compat）

## 1. Specs landing

- [x] 1.1 `infra-provider` / `package-ai-bridge` / `runtime-model-registry`：三 api 一等公民（含 openai-completions）
- [x] 1.2 新增/改写：`compat` 命名轮廓、`api_key` 优先于 kind env；deepseek Responses 不发 include
- [x] 1.3 validate change + 触及 caps（`--no-check`）

## 2. 实现
    14|
- [x] 2.1 bridge Completions 一等 + 主仓 `AdapterKind::OpenAiCompletions`；保留 XyChunk alias / MappedBridge
- [x] 2.2 `WirePolicy`/`Compat`：`deepseek` 轮廓；Responses assemble 跳过 include
- [x] 2.3 Completions × deepseek：thinking body（pi 对齐最小集）
- [x] 2.4 `ModelEntry.compat` + `ModelEntry.api_key`；resolve / bootstrap 接线
- [x] 2.5 文档 / example.yaml；用户 `~/.config/xylitol` 只加两模型 + secret 占位

## 3. 校验

- [x] 3.1 `cargo test -p xylitol-ai-bridge`（133）+ factory/manifest/thinking 触及单测
- [x] 3.2 `just lint` + specs validate `--no-check`（package-ai-bridge / infra-provider / runtime-model-registry）
- [x] 3.3 全闸 `just qa`（含 live-provider Ornith；用户已手测 DeepSeek/Zen MCP）
- [x] 3.4 rename：`c1940-remove-openai-completions` → `c1940-update-multi-api-named-compat`；h12/pab13 含 Completions
