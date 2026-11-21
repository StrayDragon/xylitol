---
depends_on: []
needs_specs_change: false
title: config section 键与类型单源化：SectionBinding 关联常量收编双真值源
---

## Why

config section 注册制（c3395）落地后，每个 section 的**键字符串存在两份独立真值**，且类型化读取静默回落：

- **描述符侧**：各插件 `config.rs` 的 `config_section() -> ConfigSection { key: &'static str, schema: fn, default: fn }`（11 个插件 + config 插件 `sections.rs::core_sections()`；例：mcp 插件 key 为 `"mcp_servers"`）。
- **读取侧**：`AppConfig`（`plugins/xylitol-official-plugin-config/src/types.rs:37`）以 `#[serde(flatten)] sections: BTreeMap<String, Value>` 持原始值，各消费方按**字面量字符串** `sections.get("…")` 取值再 serde 解析，取不到/解析失败按缺省静默回落。
- 结构体上 `#[serde(default)]` 使缺失 section 静默为空——键漂移（描述符写 `mcp_servers`、读取点写 `mcp`）不会有任何报错，只表现为配置「不生效」。

这是架构审查 C14 的原始判定：键与类型错位只静默回落 default。后果：改一个 section 名要全局搜字符串；错位是运行时静默 bug 而非编译错误。

## What Changes

- `xylitol-plugin-sdk` 的 `config_section` 模块新增类型化绑定 trait（形状见 design）：`const KEY` 关联常量 + schema/default 供给 + `from_sections` 类型化读取（缺键 / 解析失败**显式报错**，不再静默回落）。
- 11 个插件声明方与 core_sections 迁到绑定 trait：KEY 只写一处，描述符由绑定派生（`descriptor()`），`config_sections::all()` 聚合面与未知键校验 / schema 生成行为不变。
- 类型化读取点改经 `from_sections`（loud error）；`AppConfig` 的 serde 形状与既有配置文件兼容性不变。
- **loud-read 条款**：若实施普查发现「静默回落」是既有可观察行为且被场景依赖，则本 change 升级为行为变更（`needs_specs_change: true`）并停下重审——预判不成立，因 loader 已有 schema-first 与未知键校验兜底（boot.rs:143 / gen-config-schema）。

## Capabilities

无 live spec 变更（纯实现层绑定形状重构；`runtime-config` 的分层加载 / 深合并 / 未知键校验语义不变）。

## Impact

- 代码：`crates/xylitol-plugin-sdk/src/config_section.rs`、`plugins/xylitol-official-plugin-config/src/sections.rs|types.rs`、11 个插件 `config.rs` 及其类型化读取点、`crates/xylitol-kernel` 消费面（bootstrap.rs:701 / boot.rs:143,165 / reload.rs:196 / attach.rs:607 / gen-config-schema.rs）。
- 不改配置文件格式、不改 schema 生成输出、不改未知键校验语义。

## Open Questions

- 类型化读取点的完整清单由实施普查产出（rg `sections.get(` / 按插件类型结构体的消费路径）；本提案只钉「键单源 + loud read」两个不变量。
