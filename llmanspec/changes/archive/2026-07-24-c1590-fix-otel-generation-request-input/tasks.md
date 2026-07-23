# c1590 Tasks

## 1. Specs

- [x] 1.1 live `infra-otel`：修订 otel10；新增 otel16 request-body input、otel17 abort finalize + feature 场景
- [x] 1.2 attach + validate --strict

## 2. Bridge trace + adapters

- [x] 2.1 `ProviderRequestTrace`：`capture_request_input`；`finalize`（Completed / Aborted）；Drop 未 finalize → Aborted；Done 解耦 usage
- [x] 2.2 openai-completions / openai-responses / anthropic：HTTP 前 capture 完整 request JSON
- [x] 2.3 单测：capture→Done 有 input；abort Drop 有 ERROR + partial output；none 档不写

## 3. Gate

- [x] 3.1 `cargo test -p xylitol-ai-bridge -- ProviderRequestTrace capture abort observation`
- [x] 3.2 finalize
