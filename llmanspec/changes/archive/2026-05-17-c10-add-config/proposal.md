---
depends_on: []
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

> **⏸️ DAP PAUSED** — DAP 相关配置段已暂停开发（2026-05-17）。配置结构体中 DAP 字段暂不添加，等待 `c85-add-dap-layer` 恢复后引入。

### 配置层级

| 层级 | 路径 | 说明 |
|------|------|------|
| 全局基础 | `~/.config/xylitol/config.yaml` | 用户全局默认 |
| 全局本地 | `~/.config/xylitol/config.local.yaml` | 用户个人覆盖（可选） |
| 项目基础 | `<project>/.xylitol/config.yaml` | 项目团队共享 |
| 项目本地 | `<project>/.xylitol/config.local.yaml` | 开发者个人覆盖（可选，gitignored） |
| 用户会话 | CLI `--config` 参数 | 临时覆盖 |

### 配置覆盖规则

- 后者优先级更高，深层合并（不替换整个 section）
- 安全规则只能收紧不能放宽（§12.3）
- 同一位置的 `.local.yaml` 叠加在其 `config.yaml` 之上

### 模板插值

YAML 文件在解析前先经过模板渲染，支持以下语法：

| 语法 | 来源 | 示例 |
|------|------|------|
| `{{ env.KEY }}` | OS 环境变量 | `{{ env.XYLITOL_LOG_LEVEL }}` |
| `{{ secret.KEY }}` | `secret.env` 文件 | `{{ secret.ANTHROPIC_API_KEY }}` |
| `{{ env.KEY \| default("val") }}` | 带默认值的环境变量 | `{{ env.XYLITOL_MODEL \| default("gpt-4o") }}` |

未设置且无 `default()` 的变量会导致启动失败并提示缺失的 key 及应编辑的文件。

### Secret 管理

| 文件 | 位置 | Git 状态 | 说明 |
|------|------|---------|------|
| `secret.env` | `~/.config/xylitol/` | 不适用（用户目录） | 全局密钥 |
| `secret.env` | `.xylitol/` | gitignored | 项目密钥 |
| `secret.env.example` | `.xylitol/` | committed | 引导模板，列出必需 key（无值） |

密钥解析优先级（高→低）：OS 环境变量 > `secret.env` 文件 > 模板 `default()` 值。

### Local Overlay

每个配置位置支持可选的 `config.local.yaml` 叠加文件，用于在不修改已提交配置的情况下覆盖个人偏好：

- 全局位置：`~/.config/xylitol/config.local.yaml`
- 项目位置：`.xylitol/config.local.yaml`（gitignored）
- CLI `--config`：无叠加（单文件）

### 配置目录发现

支持环境变量控制配置目录位置：

- `XYLITOL_CONFIG_DIR`：覆盖全局配置目录（默认 `$XDG_CONFIG_HOME/xylitol/` 或 `~/.config/xylitol/`）
- `XYLITOL_PROJECT_DIR`：覆盖项目目录（支持从 CWD 向上搜索 `.xylitol/` 或 `.agents/` 目录）

### 项目双根配置

项目级存在两个配置根目录，各自职责不同：

| 目录 | 性质 | 说明 |
|------|------|------|
| `.xylitol/` | xylitol 专属 | 配置文件（config.yaml 等）、密钥（secret.env） |
| `.agents/` | 社区约定（只读） | 技能定义（skills/*/SKILL.md）等，xylitol 不写入此目录 |

`.agents/` 目录支持零配置自动发现。CWD walk 时同时识别 `.xylitol/` 和 `.agents/` 作为项目标记。`ConfigPaths` 结构体暴露 `agents_dir: Option<PathBuf>` 供下游模块（如 c65 技能系统）消费。

#### Skills 解析顺序（低→高优先级）

| 优先级 | 来源 | 性质 |
|--------|------|------|
| 1（最低） | 全局 `config.yaml` `skills:` | 用户默认技能 |
| 2 | 全局 `config.local.yaml` `skills:` | 可选叠加 |
| 3 | `.agents/skills/*/SKILL.md` | **项目约定自动发现**（零配置） |
| 4 | 项目 `config.yaml` `skills:` | 显式配置覆盖同名自动发现 |
| 5 | 项目 `config.local.yaml` `skills:` | 可选叠加 |
| 6（最高） | CLI `--config` `skills:` | 临时覆盖 |

合并语义：按 `name` 去重，高优先级同名 skill 覆盖低优先级的同名字段。`allowed_tools` 未声明时默认 `["*"]`。

> 注：SKILL.md 的实际解析和技能加载在 c65-add-skills-mcp 中实现，c10 仅负责目录发现和路径暴露。

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
    #[cfg(feature = "infra-acp")]
    acp: Option<AcpConfig>,
}
```

## Capabilities

- `config-system`: YAML 配置解析、校验、三级覆盖合并、JSON Schema 生成

## Impact

- 新增 `serde`, `serde_yaml`, `schemars`, `jsonschema`, `minijinja`, `dotenvy`, `dirs` 依赖
- `src/infra/config/` 从占位变为实际实现
- 后续所有 feature 的配置项均在此注册
