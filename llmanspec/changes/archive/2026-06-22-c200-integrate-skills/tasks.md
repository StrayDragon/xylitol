# c200-integrate-skills: Tasks

## Skill Expansion in AgentSession

- [x] 在 AgentSession 中实现 `expand_skill_command(name, args)` — 查找 + 读文件 + 生成 XML
- [x] `process_prompt()` 检测 `/skill:name args` 模式（在 `/template:` 之后、`/command` 之前）
- [x] 未知技能 → pass through，文件读取错误 → 返回 None
- [x] 接入 prompt 管线：返回 `PromptResult::Expanded(xml)`

## System Prompt XML Format

- [x] 更新 `build_system_prompt()` — `<available_skills>` 含 `<name>`、`<description>`、`<location>`
- [x] 将 `SystemPromptOpts.skills` 从 `Vec<String>` 升级为 `Vec<SkillInfo>`
- [x] 实现 `xml_escape()` 函数（转义 &, <, >, ", '）
- [x] 对所有技能元数据应用转义

## Skill Sourcing

- [x] 添加 `load_sourced_skills(inputs: &[(PathBuf, &str)])` — (path, source) 元组加载
- [x] 已有来源元数据通过 `SkillInfo.source_info` 携带
- [x] `DefaultResourceLoader` 已有的 `load_skills_internal()` 使用来源加载

## Invocation Format

- [x] `expand_skill_command()` 实现完整 XML 调用块格式
- [x] 包含：`References are relative to {baseDir}` 行
- [x] 如果 args 存在，用 `\n\n` 追加
- [x] 所有属性 XML 转义

## Verification

- [x] `cargo build` — 0 errors
- [x] `cargo test` — 技能测试通过
- [x] `llman sdd validate c200-integrate-skills`
