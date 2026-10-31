# 调研：配置 include（2026-10-31）

## 动机

magpie 连接器插件（x-magpie）会把网关模型目录写成独立文件（例如 `~/.config/xylitol/magpie-models.yaml`），不触碰用户主配置。xylitol 当前只加载固定层（全局 `config.yaml|yml` → 项目 `.xylitol/config.yaml|yml` → CLI `--config`），没有读取外部文件的入口。

## as-is

- `plugins/xylitol-official-plugin-config/src/paths.rs`：`ConfigPaths { global_dir, project_dir }`；`XYLITOL_CONFIG_DIR` / `XYLITOL_PROJECT_DIR` 可覆盖。
- `loader.rs`：`merge_config_layers` 依次 `merge_first_layer` → `merge_layer` → `load_and_render`（渲染 `{{ secret.* }}` / `{{ vars.* }}`）→ `deep_merge`；最后 `serde_json::from_value::<AppConfig>`（`deny_unknown_fields`）。
- 无 include 字段；未知顶层键直接拒绝。

## 方案比较

| 方案 | 优点 | 缺点 |
|---|---|---|
| A. 顶层 `include: [path]`（低优先级层） | 通用、可链式、用户可控；主配置可覆盖 | 需要环检测与缺失文件策略 |
| B. 约定目录自动加载（如 `models.d/*.yaml`） | 零配置 | 隐式、与「配置后置」原则冲突；难以表达顺序 |
| C. 仅 `models.include` | 面窄 | 插件若还要注册 external_plugins 就不够 |

选 A：显式、通用、与现有分层合并同构；缺失文件 warn 跳过（首次 refresh 前文件不存在是常态），成环报错。

## 路径与格式

- 相对路径相对**包含文件所在目录**解析；`~` 展开为 home。
- 文件按 YAML 解析（YAML 是 JSON 超集，插件写 JSON 也可读）。
- 被包含文件同样渲染 `{{ secret.* }}`。

## 风险

- 合并顺序错误会让主配置覆盖不了 include → 单测锁定。
- 环检测依赖规范化路径（`std::fs::canonicalize`）；文件不存在时无法规范化，按原路径计入集合。
