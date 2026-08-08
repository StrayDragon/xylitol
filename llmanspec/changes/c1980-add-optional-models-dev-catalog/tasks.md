# Tasks：c1980-add-optional-models-dev-catalog

## 0. 调研门禁

- [x] 0.1 源仓 TOML / schema 字段对照（`research/`）
- [ ] 0.2 有 catalog.proxy 时拉齐官方 `api.json` / `models.json` / `catalog.json` 与源仓统计交叉校验

## 1. 合约（Specs landing）

- [ ] 1.1 `runtime-config`：`catalog.*`（enabled 默认 false、proxy、paths、cache）
- [ ] 1.2 `cli-entry`：`catalog refresh|status|suggest` ops（早退、非 bootstrap 会话）
- [ ] 1.3 `runtime-model-registry`：catalog 结果 suggestion-only；MUST NOT 自动 register
- [ ] 1.4 架构文档：多厂商模型「后置 models.dev」落地说明

## 2. 实现切片

- [ ] 2.1 配置类型 + 校验 + 示例生成器片段
- [ ] 2.2 cache / ETag / proxy-aware HTTP 拉取（失败保留旧 cache）
- [ ] 2.3 merge（override > pack > cache）+ 小映射表 → suggestion DTO
- [ ] 2.4 CLI `catalog` 子命令
- [ ] 2.5 单测：默认 off、merge 覆盖、未知 npm 不注册、deepseek 建议样例

## 3. 验证

- [ ] 3.1 相关单测 / BDD 绿
- [ ] 3.2 `just qa`（或约定门禁）绿
