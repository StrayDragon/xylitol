# Tasks — c1175-refactor-slash-command-ssot

- [x] 1. Delta + design + tasks 齐；`llman sdd validate c1175-refactor-slash-command-ssot --no-interactive`
- [x] 2. 落地产品 SSOT 模块（name/description[/hint]）；改写 agent `BUILTIN_COMMANDS` / `get_all_commands`；删 dead_code 与过时注释
- [x] 3. TUI `slash_catalog`（及必要处）改读 SSOT；保持 A03 旧短名 unknown
- [x] 4. 测：GetCommands / catalog 名称集一致；`/tree` 等仍 unknown；agent 单测更新
- [x] 5. 同步 `cli-entry` sc3 / `agent-session` a23 语义落地；`just fmt` + lint + 相关 test / app-tui BDD
- [x] 6. `llman sdd validate c1175-refactor-slash-command-ssot --strict --no-interactive`
