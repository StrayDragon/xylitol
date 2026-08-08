# Tasks: c1940-remove-openai-completions

> Specs landing 须在 `change start` / attach 之后。测试缝见 `design.md`。

## 1. Specs landing（绑定分支后）

- [x] 1.1 `infra-provider`：改写 `pa4`/`pa6`/`pa22`；feature 去掉 Completions 双实现措辞；补「废弃 api 静默→Responses」文档场景（`feature: false`）
- [x] 1.2 `package-ai-bridge`：`pab5`/`pab11`–`pab13`/`pab15`/`pab21` 去掉 Completions；feature 删 `openai-completions-effort`，改写 `openai-via-sdk`
- [x] 1.3 `runtime-model-registry`：改写 `m13`/`m14` 与 Completions 场景为「残留 openai-completions → 装配 Responses」
- [x] 1.4 `agent-hooks`：`h12` 仅 Responses + Anthropic；删 Completions 场景
- [x] 1.5 `package-ai-bridge-accounting`：去掉 Completions-only 措辞/场景（或改为非 Responses）
- [x] 1.6 `llman sdd validate c1940-remove-openai-completions --strict --no-check` 及触及 caps

## 2. 删除 Completions 实现

- [ ] 2.1 bridge：删 `openai_completions` / Completions 传输模块；`AdapterKind`/`factory`/`thinking`/`lib` 去引用；Cargo 去掉 `chat-completion`
- [ ] 2.2 主仓 infra：删 Completions 外壳与选型分支；`resolve`：未知/`openai-completions` → OpenAI kind 默认 Responses
- [ ] 2.3 清理 bootstrap/manifest/example 测例与 `configs/example.yaml` Completions 注释
- [ ] 2.4 文档与 AGENTS：`多厂商模型`/`配置与档案`/bridge `AGENTS.md` 改为 Responses-only（OpenAI 族）+ 保留 Anthropic；注明 ② / 暂无 ①

## 3. 校验

- [ ] 3.1 `cargo test -p xylitol-ai-bridge` + 触及主仓 provider/bootstrap 单测
- [ ] 3.2 `just qa`（或至少 `just lint` + 相关 test）
