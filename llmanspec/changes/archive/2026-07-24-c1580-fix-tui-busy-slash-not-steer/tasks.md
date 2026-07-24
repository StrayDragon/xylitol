# c1580 Tasks

## 1. BusySlashPolicy

- [x] 1.1 `BusySlashPolicy::{Allow,Reject}` + 穷尽 `busy_slash_policy`
- [x] 1.2 `try_busy_input` 只查表；未知 `/…` 不 steer
- [x] 1.3 effects：Allow 命令去掉 busy 二次拒绝
- [x] 1.4 harness：`/session-name` Allow、`/reload` Reject、未知不 steer
- [x] 1.5 test: `cargo test -p xylitol --lib -- busy_slash harness_busy_slash busy_policy`

## 2. 合约

- [x] 2.1 新增/修订 atm 要求 + feature 场景；同步 atm8/9/11 busy 语义
- [x] 2.2 attach + validate --strict
