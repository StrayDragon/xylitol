# Tasks: c1880-update-responses-first-api-boundary

> Specs landing 须在 `change start` / attach 之后。

## 0. Review 门

- [x] 0.1 Decisions：`defaults.rs` 纯常量；无 YAML 新键；本波无 env；第一语言 vs 方言；extra_policy 三 wire 位；Completions 编译+冒烟
- [x] 0.2 默认板落点 = **`packages/xylitol-ai-bridge`**（infra 仅注入）
- [x] 0.3 默认形态 = **`defaults.rs` 常量**（本波不做 env overlay）
- [x] 0.4 测试 seam 可接受（单测 + toon `feature: false`；不扩 Completions BDD）

## 1. Specs landing（Branch binding 后）

- [x] 1.1 `package-ai-bridge`：pab19–pab21（WirePolicy / extra_policy 范围 / 第一语言 vs 方言）
- [x] 1.2 `infra-provider`：pa4 补充 + pa21–pa22（注入不拥有；api 全称）
- [x] 1.3 **跳过** `runtime-config` 新字段 req
- [x] 1.4 产品文：`多厂商模型.md` / `配置与档案.md`

## 2. 默认板实现（apply）

- [x] 2.1 在 `xylitol-ai-bridge` 实现 `defaults.rs`（纯常量）+ `WirePolicy` / `Default`（只读 defaults；测试可字面量覆盖）
- [x] 2.2 装配路径注入；AdapterKind 不读 WirePolicy；**无** `env::var` 策略路径
- [x] 2.3 Completions：相关包编译通过（factory 单测覆盖显式 `openai-completions` 选型）

## 3. 文档

- [x] 3.1 research §5.1 / 术语表
- [x] 3.2 多厂商 / 配置与档案短句对齐

## 4. 校验

- [x] 4.1 specs `validate --strict --no-check`（package-ai-bridge / infra-provider）
- [x] 4.2 apply：`cargo test`（wire_policy + factory）+ `just fmt` / lint 触及面
- [x] 4.3 verify 建议：assemble/usage 显式消费 WirePolicy；`default_adapter_api` 注释改 API 协议族
