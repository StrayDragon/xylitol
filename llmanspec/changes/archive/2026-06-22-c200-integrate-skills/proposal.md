---
id: c200-integrate-skills
title: "Integrate Skills into Prompt Flow — expansion, sourcing, invocation blocks"
depends_on: [c170-refactor-agent-message-types, c180-rebuild-agent-session]
---

## Why

当前技能系统（`src/infra/skills/loader.rs`）实现了 SKILL.md 发现和校验，但技能从未被实际集成到代理提示流中。pi 中有以下缺失功能：

1. **缺少 `/skill:name args` 展开**：用户消息中的技能引用未被展开为 XML 块
2. **缺少技能调用 XML 格式**：`<skill name="" location="">` 块未在系统提示词中正确渲染
3. **缺少技能来源跟踪**：发现技能时不记录来源元数据（全局/项目/.skills 目录）
4. **缺少 XML 转义**：技能名称/描述在 XML 中未转义（&、"、<、>、'）

## What Changes

1. **技能展开**：在 `AgentSession.prompt()` 中检测 `/skill:name` → 读取技能文件 → 生成 XML 调用块
2. **技能 XML 格式**：系统提示词中的技能区使用 `<available_skills>` XML 格式（对齐 pi 的 `formatSkillsForSystemPrompt()`）
3. **技能来源跟踪**：`loadSourcedSkills()` → 为每个技能附加来源元数据（来源路径、范围类型）
4. **XML 转义**：技能名称/描述中的 XML 特殊字符正确转义
5. **Invocation 块**：`formatSkillInvocation()` → 从技能文件内容生成 `<skill>` XML 块

## Capabilities

- skill-extension

## Impact

- `src/agent/session.rs`：添加 `_expandSkillCommand()` 方法
- `src/agent/prompt.rs`：更新系统提示词中的技能区使用 XML 格式 + 转义
- `src/infra/skills/loader.rs`：添加 `loadSourcedSkills()` 和 XML 工具函数
- `src/infra/resource/loader.rs`：更新技能加载以记录来源

## Definition of Done

- [ ] `/skill:name args` 在 AgentSession.prompt() 中展开
- [ ] 系统提示词中的技能区使用 `<available_skills>` XML
- [ ] 技能名称/描述 XML 转义
- [ ] `loadSourcedSkills()` 实现
- [ ] `formatSkillInvocation()` 实现
- [ ] `cargo test` 通过
