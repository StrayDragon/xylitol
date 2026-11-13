# Design — --trust / --no-trust 同传显式报错

## 0. 现状复核（2026-11-13，HEAD b4952e5）

- flag 对在 **3 个参数组**成对出现：`src/cli/args.rs` Tui 组（:60-62）、Print 组（:94-96）、根 CliArgs（:191-192），全部无 `conflicts_with`。
- 静默取信：`src/cli.rs:88-92` `match (surface.trust, surface.no_trust)` 首臂 `(true, _) => Some(true)`。
- 跨层组合路径：`src/cli/intent.rs:39-40` `trust: run.trust || parent.trust`、`no_trust: run.no_trust || parent.no_trust`——root 传一个、子命令传另一个，合并后双真，clap 同组 `conflicts_with` **盖不住**此路径（实施时以实际 clap global/propagation 行为验证）。

## 1. 目标与边界

- 目标：任何方式使两 flag 同时为真 → 显式报错、非零退出、不启动 TUI/print、不触碰 trust store。
- 边界：不改变单 flag 语义与 trust 门禁流程；不新增第三种 flag。

## 2. 设计（双层防线）

1. **解析层（clap）**：3 个 flag 组内 `trust` 各加 `conflicts_with = "no_trust"`——同组同传在 parse 期报错（clap 标准冲突文案、exit 2）。
2. **合并点（单一校验）**：`intent.rs` run/parent 合并后若 `trust && no_trust`，返回显式错误（错误信息含两 flag 与出现层级），`cli.rs` 收口为 eprintln + 非零退出；`cli.rs:88` 的 `(true, true)` 臂不可再静默通过（match 改为穷举报错或提前校验保证不可达）。

两层都必须存在：clap 层给同组好文案，合并层兜跨层组合。

## 3. spec 落点

`cli-entry` 新增规则（中文、产品可观察行为）：

`规则: trust-flags-conflict` —— CLI 同传 --trust 与 --no-trust（含经父子命令合并后同真）时必须显式报错并以非零码退出，不得静默选择任一方向。
`场景: 同传信任标志硬失败` —— 当 命令行同传 --trust 与 --no-trust 那么 报错退出且未进入 trust 门禁。

## 4. 测试边界

- 单测：合并点矩阵（同层 / 跨层 / 单 flag）。
- BDD：cli-entry 场景绑定（非零退出码 + stderr 含报错 + 无 TUI 启动断言）。
- 回归：既有单 flag trust 门禁 BDD（domain-security / app-tui-trust）不变。
