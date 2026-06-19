# Design: Prompt Templates 接线（基于现实修正）

## 为什么大幅改方向（决策记录）

调研 `src/agent/templates.rs` 发现项目**已有一套完整 prompt 模板实现**：
- 位置参数语义（`$1`/`$2`/`$@`/`${N:-default}`），比 minijinja `{{ }}` 更贴合 shell/CLI 习惯
- `process_prompt` 已拦截 `/template:name` → `PromptResult::Expanded`
- `infra/resource/loader.rs` 已发现 `~/.xylitol/prompts/*.md` 与 `<cwd>/.xylitol/prompts/*.md`

原 design 的 minijinja + 声明式 slots 是重复造轮子，且会破坏既有 BDD 覆盖的 `$1` 语义。**采用 A 方案**：尊重现有实现，只补缺失的接线。

## 真实差距与解决

1. **接线断开** → CLI `run()` 构建 ResourceLoader 后调用 `register_prompt_commands(&loader.get_prompts().0)` 注入。
2. **无命令注册** → 新增 `register_prompt_commands()`，并把模板纳入 `get_commands()` 返回（c110 会统一为 `SlashCommandSource::Prompt`）。
3. **无来源标记** → `agent::templates::PromptTemplate` 增 `source_path: Option<PathBuf>`。

## 决策

1. **不引入 minijinja**：现有位置参数引擎稳定、有测试。新引擎无收益。
2. **source_path 用 Option**：现有 `new()`/BDD 构造点传 None 即可向后兼容，无需逐个改。
3. **命令命名沿用 `/template:name`**：已有 BDD 覆盖；c110 统一 slash 表时再评估是否改 `/prompt`。
4. **转换层**：`register_prompt_commands` 接收 `infra::resource::PromptTemplate`（loader 类型）→ 转为 `agent::templates::PromptTemplate`（运行时类型），因两者字段不同（loader 用 `content`，agent 用 `body`）。

## 不做

- 不重写 `templates.rs`、不换模板引擎、不改 `/template:name` 语法。
- 不做来源 scope 区分（user/project）在命令层——source_path 已含足够信息，c110 按需扩展。
