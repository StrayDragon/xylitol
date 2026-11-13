---
depends_on: []
title: "Fix trust flag conflict rejection"
---

# Tasks — c3331 --trust/--no-trust 同传显式报错

- [x] T1 spec 落地：cli-entry.feature 增 `trust-flags-conflict` 规则 + 场景（change 分支上提交）。
- [x] T2 解析层：args.rs 3 个 flag 组的 trust 加 conflicts_with；同组同传 clap 报错单测。
- [x] T3 合并层：intent.rs 合并点校验双真 → 显式错误；cli.rs match 消除 (true,true) 静默臂；跨层组合单测。blocked_by: [T2]
- [x] T4 BDD：场景绑定（非零退出 + 报错文案 + 未启动端）；既有单 flag trust BDD 回归。blocked_by: [T1, T3]
- [x] T5 收口：`llman-sdd validate c3331 --strict` 绿、`just fmt && just lint && just test && just qa` 绿（WASI_SDK_PATH 导出）。blocked_by: [T4]
