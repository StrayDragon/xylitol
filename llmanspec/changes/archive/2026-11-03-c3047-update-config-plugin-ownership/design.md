# 设计

## 目标形态

```text
AppConfig {
    xylitol: BootstrapConfig,      // schema_version, unknown_keys, plugins
    sections: BTreeMap<String, Value>,  // serde(flatten)
}

ConfigSection {
    key:        &'static str,
    owner:      &'static str,
    domain:     ConfigDomain,
    schema:     fn() -> RootSchema,
    default:    fn() -> Value,
}
```

## 加载流水线

```text
读文件（global → project → --config）
→ {{ }} 渲染（minijinja，不变）
→ YAML 解析为 Value（tag 保留）
→ merge key << 展开（Value 层）
→ 层合并（现有逻辑）
→ 阶段一：校验已知官方 section 与 xylitol / external_plugins 形状；未知顶层键先收集
→ boot：插件按 rutis 依赖顺序读取并校验自己的 section
→ 阶段二：剩余未知顶层键按 xylitol.unknown_keys 策略硬失败或告警
```

## 注册表

- 官方插件：kernel 静态清单（编译期可见），解决「配置加载早于插件 boot」。
- 外进程插件：manifest 声明 `config_key` + `config_schema`；允许顶层 section，插件 boot 后校验自己的 section。
- `gen-config-schema` 遍历静态清单生成聚合 schema；`config schema --domain` 输出片段。

## 未知键策略

- 两阶段：pre-boot 只收集未知顶层键；boot 后插件认领自己的 section；剩余未知键默认硬失败，`xylitol.unknown_keys: warn` 放宽。
- section 内未知键：插件 schema 决定，默认 deny。
- 未知枚举值：硬失败。
- 旧键：硬失败并给迁移提示；`schema_version` 缺失按当前版本处理。

## 迁移

1. 先外放工具类 section（lsp/dap/mcp/hooks/loop_policies/todo/workflow/plan/subagent/external_plugins）。
2. 再迁 core 键（models/agents/execution/tools/tool_batch/session/security/serve/otel/tui）到 owner。
3. 键名保持不变；`AppConfig` 退化为容器。

## 测试 seam

- config crate 单测：加载、校验、未知键、merge key、schema 渲染。
- kernel 单测：注册表聚合、bootstrap 引导段、插件 section 反序列化。
- BDD：`xylitol config check` 子进程、未知键错误文案、外部插件 config schema。
