# 设计：配置节注册表（c3395）

## 目标形状（三层，对齐 LoopMiddlewareRegistry 既有模式）

- 词表 = xylitol-plugin-sdk：`config_section::ConfigSection { key: &'static str, schema: fn() -> serde_json::Value, default: fn() -> serde_json::Value }`。纯数据 + 函数指针，无新增依赖（serde_json 已在词表内）。schemars 不得进 sdk：schema fn 由各声明方 crate 在自己边界用 schemars 生成（符合 src/AGENTS.md「sdk 收纳准则」第 1 条）。
- 声明方 = 各官方插件与 config 核心节：七插件暴露 `pub fn config_section() -> ConfigSection`；config 暴露 `core_sections()`（核心域节 + 过渡节）。
- 聚合 = xylitol-kernel：boot 组装全量清单（编译期可见全部官方插件声明，静态聚合而非装配期注册），注入 ConfigPlugin 构造；未知键校验（loader.rs:252 现查静态表）改查注入清单——全集合等值，行为不变。
- 产物 = gen-config-schema bin 迁至 kernel（kernel 依赖全部官方插件，能取到全量清单，无需装配 runtime）；`render_app_config_schema` 参数化（config 侧）；`sort_object_keys` 已保证键序稳定，聚合顺序不影响产物字节。

## 归属表（24 节）

| 节 | 类型现居 | 归属 | 批次 |
|---|---|---|---|
| xylitol（引导段） | config types | config core | 已是 |
| models / agents / execution / session / tool_batch / tools / security / trust / serve / subagent / external_plugins / tui | config types | config core（决策 D2） | 本 change 定职不搬 |
| otel | config types | config core 过渡 | 2（评估搬 obs） |
| context_inject | config types 与插件双定义 | 插件（收敛双定义） | 2 |
| compaction / codemode | sdk（c3355 先例） | 不动 | — |
| hooks / loop_policies / todo / workflow / mcp_servers / lsp / dap | 插件（config 仅 re-export） | 插件自声明 | 本 change |
| 外进程 claim 键（boot.rs:143-149） | kernel | 保持现状 | 4（评估并入） |

## 本 change 范围

加：sdk 词表；七插件声明；kernel 聚合注入 + row 直连 + bin 迁移。
删：config 七插件 re-export 与 Cargo 依赖；旧 SECTIONS 表（收缩为 core_sections()）；ConfigDomain 死列。

## 后续批次（不在本 change）

- 批次 2：context_inject 双定义收敛（插件胜出）；otel 归属评估（obs 豁免现状）；compaction / codemode 声明挂到各自插件（类型留 sdk）。
- 批次 3：config→xylitol-runtime 依赖（permission 构建，lib.rs to_permission）处置评估；types.rs 瘦身复核。
- 批次 4（可选）：外进程 claim 并入注册表；「未装配插件节」语义收紧（若做，须另起 change 补 r3070 场景——本 change 保持「全量节集合合法」现状语义）。

## 决策记录

- D1 机制形状：静态聚合（kernel 编译期组装），非装配期注册。理由：未启用插件的节仍合法（现状语义），gen bin 无需装配 runtime；r3070「装配后认领」经注入清单等值满足。
- D2 核心域节（约 12 节）落点：留 config，config 定职「核心域节持有者 + 加载 / 合并 / 聚合器」。默认采用，未获用户确认。
- D3 首批搬迁：一把搬 7 个（拆掉 config 全部越界插件依赖）。默认采用，未获用户确认。
- D4 验收缝：既有 runtime-config .feature 场景 + 配置 crate 单测 + 产物逐字节不变。默认采用，未获用户确认。
- D5 compaction / codemode 类型留 sdk（既有先例，不扩大不回搬）。

## 不变式与验收

- configs/config.schema.json 逐字节不变（change 分支上 `just gen-config-schema` 后 git diff 为空）。
- 未知键两阶段校验行为不变；外进程 claim 不变；conf.d / secret.env / 深合并不动。
- BDD runtime-config 场景全绿；sdk / kernel / config / 七插件单测绿。

## 风险

- tests/bdd 若有钉死节清单的断言须同步（未确认；apply 时 rg 查 steps_config）。
- boot row 组装改直连，动 kernel bootstrap / boot 热点文件，回归靠既有 BDD 与 occupancy 装配测兜底。
- models.entries 形状被 x-magpie 连接器跨仓消费：本 change 不改形状，以产物字节一致作证。
