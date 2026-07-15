# Tasks — c1156-update-app-tui-paste-text-fallback

- [x] 1. infra `read_clipboard_text`（c8）+ 单测（可 mock 命令路径或纯 MIME/解析辅助）
- [x] 2. Driver 缝 + InProcess / Scripted / Remote stub
- [x] 3. host drain：无图 → 文本回退；皆无才 Error
- [x] 4. harness：有文本插入 / 皆无 Error；`llman sdd validate` + `just lint`
