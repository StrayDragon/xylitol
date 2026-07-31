# Design: c1218-remove-slash-prompt-templates

## 目标

去掉 pi 式 user prompt templates（磁盘 `prompts/*.md` → `/template:name` / `$1` 展开），使合约与半死实现一致；为 c1220 安全 minijinja system 组装清场。

## 非目标

- 不改 SYSTEM.md / APPEND / skills / `$skill`
- 不引入模板引擎
- 不改默认 `build_system_prompt` 正文语义

## 删除边界

| 层 | 动作 |
|---|---|
| `infra/resource` | 停扫 `prompts/`；去掉 `get_prompts` / 相关诊断（或空实现删除） |
| `protocol::resource::PromptTemplate` | 删除类型（及 SourceInfo 迁移表述中的 PromptTemplate） |
| `agent/prompt/templates.rs` + session `register_prompt_commands` | 删除 |
| `app/core/bootstrap` | 不再发现/注册 templates |
| `app/cli/resources` | 列表/详情不再含 prompts 段（S5） |
| Settings.`prompts` | 删除字段与 YAML 映射（仅服务该能力时）；BDD `prompts-list` 改写/删除 |
| BDD | 见 tasks 测试缝 S1–S5 |

## 合约改写要点

- **废止**：pt3/pt4；a21/a22；a24 中「模板展开器」；a25/a26/rd1/rd8/rd10/rc14/ts03 中 prompts 措辞
- **保留**：a23 产品 slash；a24 **仅**命令处理器拦截；atc18「不列 prompt templates」可改为「无 prompt templates 能力」或保持 MUST NOT 列出（等价）
- **新 SHOULD/MUST（窄）**：System MUST NOT 从 `prompts/` 注册 slash 模板命令；遗留目录 MUST 忽略（单测或 resources 场景钉）

## 风险

- 外部文档/用户若依赖 `/template:`：产品已半死且探索明确不支持；changelog 一句即可
- `blocks: c1220-…`：c1220 升格前须本 change 归档（或依赖边在 rename 后更新）
