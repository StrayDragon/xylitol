---
depends_on: []
---

# c100-add-prompt-templates: Prompt 模板接线 + 命令注册 + 来源标记

## Why（基于现实修正）
原 design 拟新增 minijinja + 声明式 slots 的模板系统。调研发现项目**已有一套完整且不同的**实现：

- `src/agent/templates.rs::PromptTemplate`：位置参数（`$1`/`$2`/`$@`/`${N:-default}`），更贴合 shell 习惯
- `process_prompt` 已拦截 `/template:name` → `PromptResult::Expanded`
- `infra/resource/loader.rs::PromptTemplate` 已从 `.xylitol/prompts/*.md` 发现并解析 frontmatter

**真实差距**（这三点才是缺失的）：
1. **接线断开**：`AgentSession.prompt_templates` 永远为空——`register_templates()` 标 `#[allow(dead_code)]`，CLI `run()` 从未把 ResourceLoader 发现的模板注入 session。
2. **无命令注册**：模板不进命令表，无 `register_prompt_commands()`，c110（slash 命令全集）/c115（RPC `get_commands`）无法发现它们。
3. **无来源标记**：`agent::templates::PromptTemplate` 无 `source_path`/`source_info`，无法追溯。

## What Changes（缩范围为 A 方案）
- `agent::templates::PromptTemplate` 增 `source_path: Option<PathBuf>`（向后兼容，现有构造点填 None）
- 新增 `AgentSession::register_prompt_commands(&mut self, templates: &[crate::infra::resource::PromptTemplate])`：把 ResourceLoader 的模板转换为 agent 内部模板并注册
- CLI `interface/cli/mod.rs::run()`：构建 ResourceLoader 后调用 `register_prompt_commands` 注入（仅当有模型可用时）
- `get_commands()` 覆盖 prompt 来源：每个已注册模板作为 `SlashCommandInfo { source: Skill/Prompt, ... }`（对齐 c110 的 SlashCommandSource，本变更先以 extension_commands 通道占位，c110 统一）

## 明确不做
- **不引入 minijinja / 声明式 slots**：保留并尊重现有位置参数方案（语义更优、已稳定）
- 不改变 `/template:name` 语法（已有 BDD 覆盖）
- 不重写 `templates.rs`

## Capabilities
- prompt-template

## Impact
- 非破坏性：`source_path` 增字段（Option，默认 None）；现有构造点一次性补 None。
- 复用现有位置参数引擎与 frontmatter 解析。
- 触及文件：`src/agent/templates.rs`（source_path 字段）、`src/agent/session.rs`（register_prompt_commands + 命令发现）、`src/interface/cli/mod.rs`（接线）。
