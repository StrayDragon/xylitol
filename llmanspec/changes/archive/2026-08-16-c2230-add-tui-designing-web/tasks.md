# Tasks: c2230-add-tui-designing-web

无 live specs。`skip_specs_landing: true`。分支 `c2230-tui-designing`（禁止 ff 进 main）。

## 1. 调研与提案

- [x] 1.1 写 `research/stack-survey.md`（A/B/C/D 表 + 推荐）
- [x] 1.2 写 `proposal.md`（两源、非真值、拆除条件、atc4 草稿、与 c2200 正交）
- [x] 1.3 `llman sdd change attach` + `validate --no-interactive`

## 2. bun 骨架 + tokens

- [x] 2.1 `src/app/tui/designing/app/`：`package.json` / `bun.lock` / Vite vanilla TS；`bun run dev` 左模块右预览
- [x] 2.2 `sync_tokens.py` 双写 `designing/generated/`；`just check-tui-tokens` 仍绿
- [x] 2.3 `just open-designing`、`just gen-designing-index`；壳无快捷键墙、无「(包)」
- [x] 2.4 `scripts/check_tui_designing.py` 进 `just qa`（`check_*` 经 check-scripts glob）

## 3. activity-fold 首模块

- [x] 3.1 [blocked-by: 2.1] `intent.md` 压缩可观察 MUST + 词表
- [x] 3.2 移植 fixtures `must_contain` / `must_not_contain` 到 `states/*.yaml` + cell lines
- [x] 3.3 预览可切 collapsed / envelope / expanded
- [x] 3.4 playground 闸对该模块改走 YAML states，其它槽仍解析 HTML

## 4. Agent 阅读

- [x] 4.1 `designing/AGENTS.md` supersede `design/AGENTS.md` 过时句
- [x] 4.2 生成 `generated/AGENT-INDEX.md`；qa 抓过期
- [x] 4.3 `src/app/tui/AGENTS.md` 一小段指针

## 5. 验证（切片 1）

- [x] 5.1 `bun run --cwd src/app/tui/designing/app check`
- [x] 5.2 `just check-tui-tokens` + `python3 scripts/check_tui_design_playground.py --check`
- [x] 5.3 旧闸不红；新 check 脚本已接线

## 6. 顶层迁移 + 拆旧轨

- [x] 6.1 `git mv` 到仓库顶层 `designing/`，模块落 `tui/modules/`
- [x] 6.2 迁完其余 TUI 槽；页面右栏展示对齐 / todos / 备注 / 组件键
- [x] 6.3 取消独立快捷键设计；无 keybindings 模块
- [x] 6.4 删除 playground HTML / fixtures / `check_tui_design_playground.py`；token 脚本只写 `designing/generated/`
- [x] 6.5 全部 AGENTS.md 改指向；`design/*.md` 缩成指针；代码为 SSOT
- [x] 6.6 `bun run --cwd designing/app check` + `just check-tui-tokens` + `python3 scripts/check_tui_designing.py --check`
