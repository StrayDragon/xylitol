# Design

## 现状证据

| 事实 | 证据 |
|---|---|
| 描述符非泛型三字段 | `crates/xylitol-plugin-sdk/src/config_section.rs:3-8`（`{key, schema: fn() -> Value, default: fn() -> Value}`，按 key 判等） |
| 11 插件声明方 | `plugins/xylitol-official-plugin-*/src/config.rs` 各 `pub fn config_section()`（compaction/codemode/obs/context-inject/loop-policies/todo/dap/lsp/hooks/mcp/workflow） |
| core sections | `plugins/xylitol-official-plugin-config/src/sections.rs::core_sections()` |
| AppConfig 原始 map | `plugins/xylitol-official-plugin-config/src/types.rs:37-44`（`#[serde(default)]` + `#[serde(flatten)] sections: BTreeMap<String, Value>`） |
| 键示例 | mcp 插件描述符 key = `"mcp_servers"`（其 config.rs 测试 :534 断言）——注意不是 `"mcp"`，恰证明字符串漂移风险真实 |
| 描述符消费面 | kernel `config_sections::all()` → bootstrap.rs:701、boot.rs:143（validate_unknown_keys）/165、reload.rs:196、attach.rs:607、gen-config-schema.rs:6 |

## 目标形状

```rust
// sdk config_section.rs（新增，与既有描述符并存）
pub trait SectionBinding: Sized + serde::de::DeserializeOwned {
    const KEY: &'static str;
    fn schema_value() -> serde_json::Value;          // 优先经 JsonSchema（schemars 已在 sdk deps）
    fn default_value() -> serde_json::Value;
    fn descriptor() -> ConfigSection { ConfigSection { key: Self::KEY, schema: Self::schema_value, default: Self::default_value } }
    fn from_sections(map: &BTreeMap<String, serde_json::Value>) -> Result<Self, SectionBindingError> {
        let v = map.get(Self::KEY).ok_or(SectionBindingError::Missing(Self::KEY))?;
        serde_json::from_value(v.clone()).map_err(|e| SectionBindingError::Parse(Self::KEY, e))
    }
}
```

- 各插件类型化 config 结构体 impl 该 trait：**KEY 全仓只此一处**；原 `config_section()` 函数退化为 `<T as SectionBinding>::descriptor()` 的转发（或直接改调用点，二选一以 diff 最小为准）。
- 读取点改 `T::from_sections(&cfg.sections)`：缺键 / 解析失败显式错误（错误文案进既有 LoadError / 插件装配错误通道，具体归属实施期对齐现状错误类型）。
- `ConfigSection` 描述符与 `config_sections::all()` 聚合面**不变**——schema 生成、未知键校验、两阶段校验全部照旧。

## 删除验证

想象删掉绑定 trait、退回两处写字符串：键漂移重新变成静默 bug（现状即证明）；复杂度在「每个读取点各自处理缺省」处重新冒出来。判：值得收厚。

## 不变量

1. 配置文件格式与兼容性零变化（AppConfig serde 形状不动）。
2. `gen-config-schema` 输出逐字节不变（schema_value 与现状 schema fn 同源生成）。
3. 未知键校验语义与错误文案不变（boot.rs:143 路径不动）。
4. 键唯一性 debug_assert 保留（kernel config_sections.rs）。
5. loud-read 只影响「描述符已注册但读取点键漂移 / 值类型错」的死路径——若普查发现该路径有可观察依赖，STOP 并升级 needs_specs_change。

## 测试边界（seam）

- sdk 单测：绑定 trait 的 descriptor 派生、from_sections 三态（命中 / 缺键 / 解析失败）。
- 迁移护栏：每个插件一条「KEY 关联常量 == 原描述符 key」断言（防迁移改键）；至少一条「键漂移现在报错而非静默默认」的对照测试。
- 既有 `load_with_sections` 系测试（kernel config_sections.rs tests）全绿。

## 决策记录

| 决策 | 选择 | 理由 |
|---|---|---|
| trait 落点 | sdk `config_section` 模块 | 与描述符同居；插件只依赖 sdk 的既有约束 |
| KEY 形态 | 关联常量 | 编译期单源；改键=改一处+编译器跟到 descriptor |
| schema 供给 | 优先 JsonSchema 派生 | schemars 已在 sdk deps；与 gen-config-schema 同源 |
| 声明函数去留 | 实施期按 diff 最小定 | 保函数名则 kernel 聚合面零改动 |
| loud-read | 显式错误 | 本 change 的核心价值；受不变量 5 的 STOP 条款约束 |
