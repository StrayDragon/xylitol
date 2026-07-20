# Tasks: c1390-add-cli-surface-verbs

## 1. 解析与 dispatch

- [x] 1.1 `CliCommand::Tui` / `Print`；`tui run` 可选叶子
- [x] 1.2 默认无子命令行为与今日一致（TTY→TUI；prompt→print）
- [x] 1.3 删除顶层 `--tui` / `--print` / `-p`/`--prompt`/位置 PROMPT（无兼容别名）
- [x] 1.4 help 中 Commands 含 tui/print 与 ops

## 2. Specs / BDD

- [x] 2.1 live `cli-entry`：surface vs ops MUST；默认 TUI 不变
- [x] 2.2 feature 场景：裸跑、`tui`、`print` 需 prompt、ops 顶层仍在
- [x] 2.3 `llman sdd validate` 结构 + full

## 3. 收尾

- [x] 3.1 相关测试绿
- [x] 3.2 `finalize` c1390（c1380 已归档或同分支约定满足 depends_on）
