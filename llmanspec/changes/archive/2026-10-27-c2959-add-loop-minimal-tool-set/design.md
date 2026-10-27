# Design

## 方案 A：配置映射（先做，零 core 元数据）

`ToolExposureConfig`：

```rust
pub struct ToolExposureConfig {
    pub enabled: bool,
    pub allow: Vec<String>,
    pub deny: Vec<String>,
    pub read_only: bool,
    pub write_tools: Vec<String>,
}
```

语义：

| 字段 | 作用 |
|---|---|
| `allow` 非空 | 只暴露 allow 内工具（`todo_rewrite` / `todo_update` 始终可见） |
| `deny` | 始终隐藏 |
| `read_only` + `write_tools` | 只读模式下隐藏写工具（身份维度） |
| `enabled=false` 或规则全空 | schema 形状不变 |

风险维度先由 `deny` / `allow` 表达；更细粒度的 step/risk 映射留待 core 元数据（方案 B）。

## 中间件

`ToolExposureMiddleware`：

- 阶段：`before_model`；
- order：50（在 plan-state 200 / progress-reminder 150 之前，后续中间件看到的是收窄后的 schema）；
- 只 `retain`，不删除 ToolSet、不改权限；
- 过滤前后数量与理由写日志（可观测）。

`todo_rewrite` / `todo_update` 始终保留，避免计划无法更新。

## 不做什么

- 不改 `XyToolSchema` 形状。
- 不改 permission 终裁。
- 不删除工具。
- 不按模型自评过滤。
- 不做 step/risk 的细粒度映射（后续 change）。
