---
depends_on: []
title: 配置插件自持与 schema-first 重构
branch: sdd/c3047-update-config-plugin-ownership
base_branch: main
base_sha: 6b92eca85ddbc1782a4ac85a8b518a3c4cc623da
---

# 配置插件自持与 schema-first 重构

## Why

配置体系积压了四类债务：

1. **两套配置面**：`config.yaml`（snake_case）之外还有 `settings.json`（camelCase），键集不对称（`default_thinking_level` / `thinking_budgets` / `steering_mode` / `follow_up_mode` 只在 settings.json）。
2. **静默失效**：顶层未知键被忽略（`AppConfig` 无 `deny_unknown_fields`）；本机配置里的 `token_estimate` / `tokenizers` 死段、`session.storage`、`planning` 都无人察觉；`security.enabled` 无消费方。
3. **所有权错位**：`xylitol-official-plugin-config` 手写 `AppConfig` 字段清单并依赖全部插件；插件无法独立演进自己的配置形状（`lsp` / `dap` 只能单实例）。
4. **产物维护**：`configs/example.yaml` 靠生成器维护，与 schema 漂移；`configs/config.schema.json` 是唯一机器契约。

## What Changes

- **全插件化**：所有配置 section 由其插件声明；core 退化为 loader + 注册表。`AppConfig` 变为 `xylitol` 引导段 + `BTreeMap<String, Value>` section 容器。
- **引导段**：新增 `xylitol:`，承载 `schema_version`、`unknown_keys`（`error` / `warn`）、插件启用清单。
- **注册表**：官方插件 section 静态清单放 kernel；`ConfigSection { key, owner, domain, schema, default }`。
- **未知键（两阶段）**：pre-boot 只校验已知官方 section 与 `xylitol:` / `external_plugins` 形状，未知顶层键先收集；boot 后插件按 rutis 依赖顺序校验自己的 section，剩余未知键再硬失败；`xylitol.unknown_keys: warn` 为显式逃生口；section 内未知键由插件 schema 决定；未知枚举值硬失败。
- **YAML 复用**：保留 anchor/alias；merge key `<<` 由 `Value` 预处理展开；自定义 tag 本期不做。
- **移除 `include`**：本期不再提供通用 include / presets / import。
- **schema-first**：删除 `configs/example.yaml` 与生成器；`configs/config.schema.json` 由注册表生成。
- **CLI**：新增 `xylitol config schema|check|show|init` 四个子命令。
- **外部插件**：允许顶层 section；manifest 声明 `config_key` + `config_schema`，插件 boot 后按 rutis 依赖顺序读取并校验自己的 section。

## Capabilities

- `runtime-config`：加载、section 所有权、引导段、未知键、merge key、schema 来源。
- `cli-entry`：`config` 子命令。
- `infra-plugin`：外进程插件配置与 manifest schema。
- `user-experience`：未知键与迁移错误的可操作性。

## Impact

- 破坏性：`include` 移除、未知键硬失败、`AppConfig` 结构变化。
- 迁移：旧键硬失败并给迁移提示；`xylitol.schema_version` 缺失按当前版本处理；alpha 窗口一次性切换。
- 非目标（后续 change）：`lsp` / `dap` 注册表、magpie `models.d`、settings.json 收口、`models.models` 扁平化、`security.enabled` 处置。

## Further Notes

逐组审计与能力实测：`research/config-audit.md`（随本 change 归档）。
