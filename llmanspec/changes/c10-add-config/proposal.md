---
depends_on: [c05-init-skeleton]
blocks: [c15-add-cli, c20-add-tools, c25-add-agent-loop, c40-add-hooks, c45-add-lsp-layer, c50-add-security, c55-add-planning-execution, c60-add-model-lock, c65-add-skills-mcp, c70-add-session-snapshot, c85-add-dap-layer]
---

# c10-add-config

## Why

几乎所有后续功能（CLI 模式选择、工具注册、Hook 配置、安全策略、模型路由等）都需要读取 YAML 配置。配置系统是整个项目的"神经系统"，必须最先就绪。

## What Changes

1. 在 `src/infra/config/` 实现 YAML 配置系统
2. 支持 `serde_yaml` 反序列化 + `schemars` JSON Schema 生成
3. 全局/项目/用户三级配置覆盖合并（后者覆盖前者）
4. `jsonschema` 运行时校验
5. 生成 `configs/config.schema.json` 供 IDE 补全
6. 定义完整配置结构体（`AppConfig`），所有模块从此读取配置

### 配置层级

| 层级 | 路径 | 说明 |
|------|------|------|
| 全局 | `~/.config/xylitol/config.yaml` | 用户全局默认 |
| 项目 | `<project>/.xylitol/config.yaml` | 项目级覆盖 |
| 用户会话 | CLI `--config` 参数 | 临时覆盖 |

### 配置覆盖规则

- 后者优先级更高，深层合并（不替换整个 section）
- 安全规则只能收紧不能放宽（§12.3）

### 核心配置结构

配置段分为两层：
- **始终编译**（无 feature gate）：hooks, security, repeat_detection, tools — 始终解析，运行时控制启用/禁用
- **可选编译**（feature-gated）：planning, session, skills, mcp_servers, lsp, review, rtk — 仅当对应 feature 编入时解析

```rust
struct AppConfig {
    model: ModelConfig,
    execution: ExecutionConfig,
    patch_apply: PatchApplyConfig,
    // ── 始终编译（config 层控制）──
    hooks: HooksConfig,           // 空列表 = no-op
    security: SecurityConfig,     // security.enabled = false 可禁用
    repeat_detection: RepeatDetectionConfig,  // enabled = false 可禁用
    tools: ToolsConfig,           // allowlist / blocklist
    // ── 可选编译（feature-gated）──
    #[cfg(feature = "agent-planning")]
    planning: Option<PlanningConfig>,
    #[cfg(feature = "agent-planning")]
    validation: Option<ValidationConfig>,
    #[cfg(feature = "infra-session")]
    session: Option<SessionConfig>,
    #[cfg(feature = "infra-session")]
    compaction: Option<CompactionConfig>,
    #[cfg(feature = "infra-skills")]
    skills: Option<Vec<SkillConfig>>,
    #[cfg(feature = "infra-skills")]
    mcp_servers: Option<Vec<McpServerConfig>>,
    #[cfg(feature = "ui-review")]
    review: ReviewConfig,
}
```

## Capabilities

- `config-system`: YAML 配置解析、校验、三级覆盖合并、JSON Schema 生成

## Impact

- 新增 `serde`, `serde_yaml`, `schemars`, `jsonschema` 依赖
- `src/infra/config/` 从占位变为实际实现
- 后续所有 feature 的配置项均在此注册
