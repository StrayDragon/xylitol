# Design — c1085-update-agent-skills-runtime

## 边界

| 本变更（c1085） | c1130 |
|---|---|
| Trust 发现 / 重载目录 | `$` 补全 |
| system `<available_skills>` 清单 | 提交时 **读 SKILL.md 注入**模型上下文 |
| 单测验 catalog → system | 单测/集成验 **正文注入或 read 发生** |
| | 用户消息内 **紫色高亮** `$name`（无系统行、无色块） |

**废弃**（相对前一稿）：`/session` Skills 段、`/status skills`、系统消息提醒行作为 skill 观测面。

## 装配 / 重载

```text
bootstrap / reload_skills:
  loader_cwd = trusted ? cwd : temp
  skills = loader.get_skills()
  apply → prompt_opts.skills → rebuild_system_prompt()
  MUST NOT mutate session history
```

`SkillsReloadReport`：`names`、`count`、可选 diagnostics。

## 验收矩阵（不验 TUI）

| 证据 | 如何验 |
|---|---|
| Trust on → 项目 skill 进 system | 临时 SKILL.md + trusted apply；assert system 含 name / `<available_skills>` |
| Trust off → 项目 skill 不进 | untrusted；assert 不含项目名；user 全局仍可 |
| reload | 改盘后 names/system 变；历史 len 不变 |
| A10 本波 | 不改 `/session` dump；不引入 skill 色块 / status 计数 |

**禁止**把 harness 截屏、`/session` 文案、紫色像素当作本变更门禁。

## Trade-offs

- 运行时目录就绪 ≠ 证明某次 `$` 已注入正文；后者在 c1130 用「模型侧消息 / 读文件结果」断言。
- 不在 Host 长持有 ResourceLoader（同 c1100）。
