---
depends_on: []
---

# c45-align-skills-system: 对齐 pi 技能系统（SKILL.md）

## Why
当前 xylitol 的技能系统从 YAML config 加载，与 pi 的 SKILL.md 文件发现机制完全不同。

## What Changes
- **完全重写** `src/infra/skills/mod.rs`：SKILL.md 发现、frontmatter 解析、ignore 支持、name/description 验证
- **删除** 旧的 YAML-config-based skill loading 全部代码
- **删除** `AppConfig.skills` YAML 字段和对应的 `SkillConfig` 类型

## Capabilities
- skill-extension

## Impact
- 旧 YAML-based skill loading 完全删除
- 旧 `SkillConfig` 类型删除
- pi 的 SKILL.md 格式成为唯一技能定义方式
