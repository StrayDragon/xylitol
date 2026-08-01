# Tasks: c1780-update-tui-busy-instant-lists

## 测试缝（已对齐 proposal）

- `slash_allowances` Allow/Reject 表
- 产品 TUI harness：busy 开 `/model` `/theme` `/session-resume`
- harness：busy Resume switch / rename / delete → 无写盘 + ScrollNotice 文案 A；bang-busy switch 同拒
- busy `/reload` 拒不回归；`slash_allowances` Reject 全集单测

## 1. Specs landing

- [x] 1.1 Branch binding：`llman sdd change start c1780-update-tui-busy-instant-lists`
- [x] 1.2 改写 live `app-tui-commands`（atm1/10/15/16）与 `app-tui-input`（ati21；必要时 ati29）
- [x] 1.3 `validate c1780-… --strict --no-check`；commit specs landing

## 2. Allow 表 + 开槽

- [x] 2.1 `slash_allowances`：OpenModels / Theme / OpenSessionResume → Allow
- [x] 2.2 `effects/slash`：busy 路径允许挂载 Models / Themes / SessionResume（删「unavailable while busy」拒开）
- [x] 2.3 harness：busy 开三槽

## 3. Resume switch 闸

- [x] 3.1 busy 下 `pending_session_resume_select`（及 rename/delete 确认若会写盘）拒执行 + ScrollNotice A
- [x] 3.2 harness 钉文案与「未 switch」
- [x] 3.3 busy `/reload`/`/trust` 既有拒测不回归

## 4. 文档与校验

- [x] 4.1 `design/keybindings.md` 同步 busy 列表一句（若有）
- [x] 4.2 相关 `cargo test` + `validate --strict`
