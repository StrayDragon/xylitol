# Tasks: c1890-add-responses-context-policy-assembler

> Specs landing 须在 `change start` / attach 之后。本文件为 Designed 规划壳。

## 0. Review 门

- [x] 0.1 书引用 / WirePolicy 用语对齐（`flavor`→`compat`；姊妹仓 Ch2 指针；research §7）
- [x] 0.2 深挖收束：Policy 仅全局 defaults；system 信道保持 prepend；日界默认≡现状
- [x] 0.3 测试缝：Assembler golden + Policy 默认单测；`feature: false`；不扩 BDD step
- [x] 0.4 Branch binding：`change start`（干净 main）后再 Specs landing（注意：llman 用 `origin/main` 算 base 时会偏旧；本 change 手动钉本地 main tip）

## 1. Specs landing（绑定分支后）

- [x] 1.1 `package-ai-bridge`：Assembler 唯一构造 Responses body + 消费 `WirePolicy`（新 req + `feature: false` 场景）
- [x] 1.2 `agent-runtime`：ContextPolicy 钩子默认档 + Responses 路径经 Assembler（`ar33`）
- [x] 1.3 **跳过** YAML / `runtime-config` 新键（code-first）
- [x] 1.4 校验：`llman sdd validate` change + 触及 specs `--strict --no-check`

## 2. 实现（apply）

- [ ] 2.1 `ContextPolicy` + `defaults.rs`（tools_mode / status_bar_mode / date_placement 占位；默认≡现状）
- [ ] 2.2 `ResponsesAssembler`：收敛现有 `assemble_responses_body` / prepend system；注入 `WirePolicy`
- [ ] 2.3 调用点：ReAct / infra Responses 路径只经 Assembler；禁止 adapter 二次业务布局
- [ ] 2.4 Golden：默认 ≡ 旧路径；至少两套 WirePolicy 字段集 diff
- [ ] 2.5 文档指针：research / architecture 一句「Assembler 缝已立」；不改书

## 3. 校验

- [ ] 3.1 `cargo test -p xylitol-ai-bridge`（assembler / wire_policy）+ 触及 agent 单测
- [ ] 3.2 `just qa`（含串行 live-provider；本 change 不改其行为）
