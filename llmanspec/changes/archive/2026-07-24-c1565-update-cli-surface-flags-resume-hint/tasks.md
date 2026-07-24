# c1565 Tasks

## 1. Clap 表面旗标

- [x] 1.1 顶层硬移除 `--session/--model/--list-models/--trust/--no-trust/--config/--no-color`
- [x] 1.2 `xylitol tui` / `tui run` 挂齐上述旗标（parent ≡ run 合并）
- [x] 1.3 `xylitol print` 挂 `--session/--model/--config/--no-color`
- [x] 1.4 单测：parse 成功路径 + 顶层拒绝；更新 e2e `spawn_product_fake` 参数顺序
- [x] 1.5 test: `cargo test -p xylitol --lib cli::`

## 2. Resume 提示

- [x] 2.1 TUI / print 正常退出后若 session 在 `list_sessions` → stderr `Resume by $ xylitol tui --session <uuid>`
- [x] 2.2 未持久化 / 无 session_id → 不打印
- [x] 2.3 test: 单测或 harness 覆盖 hint 判定（可测 `maybe_print_resume_hint` 逻辑 / list 命中）

## 3. 合约与验证

- [x] 3.1 `cli-entry` 增补 requirement + `.feature` 场景（旗标归属 / 顶层拒绝 / resume 行）
- [x] 3.2 `llman sdd change attach` + `validate --strict --no-interactive`
- [x] 3.3 `just fmt` + 触及路径测试绿

## 不做

- Ctrl+C / busy slash / queue drain（c1570/c1580/c1585）
