# Design — c1130-add-app-tui-dollar-skill

## 注入形状（A10）

```text
会话历史 / scrollback:  "Please run $demo and $other"
发模型前展开:
  Please run $demo and $other

  <skill name="demo">
  {SKILL.md body, frontmatter stripped}
  </skill>

  <skill name="other">
  …
  </skill>
```

- Catalog = `prompt_opts.skills`（c1085 Trust 已过滤）
- 未知 `$name`：透传，不注入
- 读盘失败：warn + 透传
- **不** persist 展开正文（resume 仍见 `$`）

## 补全 / 高亮

- `DollarSkillSource` 注册于 `UiRoot::install_completion_sources`
- `scrollback` 用户行：`highlight_dollar_skill_refs` + `Palette.skill_ref`
