# Tasks — c1010-add-app-tui-session-io-slash

- [x] 1. Delta + design + tasks 齐；`llman sdd validate c1010-… --no-interactive`（apply 前确认 c1005 已归档；结束后再 `--strict`）
- [x] 2. `PendingSlash` + `parse_slash_command`：compact / export[path] / import path；旧名无效
- [x] 3. effects：`dispatch` Compact / ExportHtml|Jsonl / ImportJsonl；系统行结果
- [x] 4. import：editor 槽 Yes/No 确认（非 Trust Choice）；取消不导入
- [x] 5. SlashCommandSource + argument_hint
- [x] 6. harness：compact / export html+jsonl / import confirm cancel+accept
- [x] 7. `just fmt` + 相关 test / clippy；tasks 全勾后 validate `--strict`
