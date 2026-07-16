# Tasks — c1085-update-agent-skills-runtime

## 1. Loader / app/core

- [ ] 1.1 `discovered_skills` / `reload_skills`（Trust temp-cwd）+ `SkillsReloadReport`
- [ ] 1.2 单测：trusted / untrusted / reload names（断言目录与 report，不验 TUI）

## 2. Agent + Driver

- [ ] 2.1 bootstrap 把 `get_skills()` 写入 `prompt_opts.skills` 并 rebuild
- [ ] 2.2 `apply_skills`（Agent → Runtime → Driver）+ 可查询已加载名（供 c1130，非 /session）
- [ ] 2.3 单测：system 含/不含 skill 名；apply 不改历史

## 3. 校验

- [ ] 3.1 `LLMANSPEC_BASE_REF=main llman sdd validate c1085-update-agent-skills-runtime --no-interactive`
- [ ] 3.2 `just qa`（或 lint + 相关 unit test）；**无** `/session` Skills harness
