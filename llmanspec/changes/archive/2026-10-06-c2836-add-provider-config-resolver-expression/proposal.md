---
branch: sdd/c2836-add-provider-config-resolver-expression
base_branch: main
base_sha: 144c6a140223ca30f7561b74e664f55d6924ea9d
---
# c2836 扩展 provider 注册配置值支持 resolver 表达式（r1912）

## Why

ConfigValueResolver（r1821-1823）作为**能力**已实现并被 BDD 真步骤验证，但从未接入
产品装配链——`$VAR` / `${VAR:-default}` / `!command` 表达式在 `models.models` 条目值
中不生效（属上轮 review 报告 P1 缺口的「未接线」标记）。本 change 把该能力接入
provider 注册配置值解析，收掉唯一的产品未接线能力，并让配置支持 shell 习惯的
密钥/地址表达式（如 `api_key: $OPENAI_KEY`、`base_url: !printf %s $GATEWAY`）。

## What Changes

- 新 spec 规则 r1912（test-provider-integration）：`models.models` 条目的
  `api_key`/`model`/`base_url`/`api`/`compat` 字段 MUST 支持 ConfigValueResolver
  表达式；纯字面量保持原样；未绑定变量或命令执行失败时 MUST 以可读错误拒绝装配。
- 实现：`load_from_paths` 生成 `AppConfig` 后对 `model.models` 各条目统一做值解析
  （唯一入口，自动覆盖 `resolve_model` / `resolve_model_meta` / bootstrap 注册）。
  语法触发 = 值以 `$`（`$VAR`/`${…}`）或 `!` 开头；否则原样。失败 → `LoadError`
  带 `alias:field` 上下文。
- 顺序与兼容：minijinja 渲染（`{{ secret.X }}` / `{{ env.Y }}`）先于 resolver，
  语法不重叠；绕过 loader 的直接 serde 路径不解析（保持旧语义）；现有字面量值
  （如 `sk-anchor`）零影响。

## Out of Scope

- `headers` 值（model 条目无 headers 字段；MCP/其它段 headers 已由模板机制覆盖）
- tokenizer / 其它配置段的值（保持现状）

## Tasks

1. 落地 spec r1912 + 场景（test-provider-integration.feature）
2. `infra/config` 提供 `expand_config_value_expr`（包装 resolver::resolve_value，
   lookup = 进程 env，返回值/错误）并接 `load_from_paths`，遍历 models.models
3. 单测：字面量透传 / `$VAR` 展开 / `${VAR:-default}` / `!command` / 未绑定报错
   / 命令失败报错，各带 alias:field 上下文断言
4. BDD 真步骤：临时配置 `models.models` 条目含表达式 → 加载 → resolve_model
   断言解析值；未绑定场景断言装配拒绝
5. 门禁全绿（qa / lint / doc / spec-validate）；verify 后归档
