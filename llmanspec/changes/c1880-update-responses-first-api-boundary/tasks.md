# Tasks: c1880-update-responses-first-api-boundary

> Specs landing 须在 `change start` / attach 之后。

## 0. Review 门

- [ ] 0.1 Decisions：`defaults.rs` 纯常量；无 YAML 新键；本波无 env；第一语言 vs 方言；extra_policy 三 wire 位；Completions 编译+冒烟
- [x] 0.2 默认板落点 = **`packages/xylitol-ai-bridge`**（infra 仅注入）
- [x] 0.3 默认形态 = **`defaults.rs` 常量**（本波不做 env overlay）
- [ ] 0.4 测试 seam 可接受

## 1. Specs landing（Branch binding 后）

- [ ] 1.1 `package-ai-bridge`：WirePolicy / ExtraPolicy 合约；默认 generic+全 false；适配层 MUST 可读；位关 MUST NOT 假装第一语言
- [ ] 1.2 `infra-provider`：装配注入默认板；**MUST NOT** 要求新 YAML 字段；adapter 选择仍按 `api`
- [ ] 1.3 **跳过** `runtime-config` 新字段 req
- [ ] 1.4 产品文：第一语言 vs 方言；本波策略 code-first

## 2. 默认板实现

- [ ] 2.1 在 `xylitol-ai-bridge` 实现 `defaults.rs`（纯常量）+ `WirePolicy` / `Default`（只读 defaults；测试可字面量覆盖）
- [ ] 2.2 装配路径注入；AdapterKind 不读 WirePolicy；**无** `env::var` 策略路径
- [ ] 2.3 Completions：编译 + 可选冒烟

## 3. 文档

- [x] 3.1 research §5.1 / 术语表：第一语言 vs 方言；flavor→compat；capabilities→extra_policy；`defaults.rs`；本波无 env
- [ ] 3.2 多厂商 / AGENTS 短句对齐（Specs landing / 实现期）

## 4. 校验

- [ ] 4.1 `llman sdd validate c1880-update-responses-first-api-boundary --strict --no-check`
- [ ] 4.2 触及面 fmt/lint + bridge 单测
