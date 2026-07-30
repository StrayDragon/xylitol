# Design: c1220-add-safe-minijinja-system-prompt

## 目标

把**默认** system 正文组装迁到沙箱 minijinja，保留 `build_system_prompt` 对外门面与产品语义（pt1/pt9/pt11 等）；SYSTEM/APPEND/context 仍纯文本。

## 架构

```
SystemPromptOpts (+ optional date)
        │
        ├─ system_prompt | custom_prompt？ → 纯文本正文（不经 Jinja）
        └─ else → sandbox_render(default.j2 + partials, ctx)
        │
        └─ 再纯文本追加：append / context / APPEND_SYSTEM / skills? /
           guidelines / runtime_policy / date+cwd
```

> 精确块顺序以今日 `system.rs` 为准；design 实施时对照代码，**不得**静默改产品顺序。若默认正文含 tools/guidelines，则这些段落进 j2；若当前在「默认 base 之后」追加，则保持该分工（Rust 拼 vs 模板），以「少散落、人类可编」为准微调，但 BDD 可观察语义不变。

## 沙箱（对齐 config template，agent 自建）

- `UndefinedBehavior::Strict`
- ctx **仅** D11 白名单；无 `env`/`secret`
- `{% include %}`：只解析预注册模板名；**无**文件系统 loader
- AutoEscape：纯文本 prompt → None（与 config YAML 同类）

## 模板布局（D10）

- `default_system.j2`（或等价）入口
- **少量** partials（建议 ≤4：如 tools、skills、guidelines、runtime_policy——实施时可合并）
- j2 以散文 + 简单 `{% for %}` / `{% set %}` 为主；过滤（mcp: 剔除、skills 可见性）在 Rust

## API（D9）

- 公开：`build_system_prompt(&SystemPromptOpts) -> String`
- `SystemPromptOpts` 增加 `date: Option<String>`（或等价）；`None` → 今日行为用 Utc 日期
- 内部可有 `render_default(ctx)`；不必公开改名

## 合约

- pt5 → MUST 经沙箱 minijinja 渲染**默认**组装路径；MUST NOT 任意路径 loader；MUST NOT 把 env/secret 注入 prompt ctx
- 废 feature `no-jinja-dep`；改为窄正向或 `feature: false` + 单测（D13）

## 非目标

- 多 profile；用户 SYSTEM.md 当模板；eval YAML；slash prompts
