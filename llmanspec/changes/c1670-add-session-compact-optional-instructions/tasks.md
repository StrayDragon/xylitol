# Tasks: c1670-add-session-compact-optional-instructions

> 验收一致：可选 instructions 仅 force；auto 干净；A05 撤销；不越界 c1680 / replaceInstructions。

## 1. 合约

- [x] 1.1 修订 live `domain-compaction`：c24 force 可选 instructions + Additional focus；auto MUST NOT；修订 `app-tui-commands` atm8：有参成功 / 空白=无参；`.feature` 锚点（bare-force / with-text / whitespace / auto-clean）
- [x] 1.2 design 数据流 / prompt 拼法 / A05 台账已定稿（本 change `design.md`）

## 2. 传输与摘要

- [ ] 2.1 `Command::Compact` + `XyDriver::compact` + `force_compact` / orchestrator / `compact_session` 透传 `Option<String>`；serde default；auto 调用点显式 `None`
  `[blocked-by: 1.1]`
- [ ] 2.2 `generate_summary` 注入 `Additional focus:`；`generate_turn_prefix_summary` MUST NOT；Fake/捕获单测证明文本进入模型输入
  `[blocked-by: 2.1]`

## 3. TUI 与台账

- [ ] 3.1 `parse_slash_command` / `PendingSlash` / `effects/slash`：无参与空白 → None；非空 → Some；去掉 usage 拒绝路径
  `[blocked-by: 2.1]`
- [ ] 3.2 更新 `PI_DELTAS.md` A05 + 变更记录；harness `h28`（或后继）三路径断言
  `[blocked-by: 3.1]`

## 4. 验收

- [ ] 4.1 单测/BDD/harness：bare-force、with-text、whitespace、auto-clean、a05-doc
  `[blocked-by: 2.2]` `[blocked-by: 3.2]`
- [ ] 4.2 `llman sdd validate c1670-add-session-compact-optional-instructions --strict`；确认无 `/compact` 短名、无 replaceInstructions、无 TUI%
  `[blocked-by: 4.1]`
