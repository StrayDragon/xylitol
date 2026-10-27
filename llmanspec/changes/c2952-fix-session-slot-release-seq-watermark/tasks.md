# Tasks: c2952

## 1. journal 水位

- [x] 1.1 `EventJournal::new_with_seq` / `with_default_capacity_and_seq` / `next_seq()`。
- [x] 1.2 单测：水位续接、下限 1、空 journal + 水位不谎报遗漏。

## 2. Host 槽释放

- [x] 2.1 `HostState.seq_watermarks` 水位表 + `seq_watermark` / `remember_seq_watermark`。
- [x] 2.2 `SessionSlot::with_seq` 接水位构造。
- [x] 2.3 `evict_slot`：记水位 → 清 driver（断引用环）→ 从 map 移除。
- [x] 2.4 接进 `on_last_subscriber_gone` 的两条路径（clean / linger 到期）。
- [x] 2.5 `slot()` 重建时读水位。
- [x] 2.6 槽单测：释放后重建 seq 续接。

## 3. spec 与 BDD

- [ ] 3.1 `server-core` 新增会话槽释放与序号水位续接规则 + 场景。
- [ ] 3.2 实现 BDD 步骤并绑定。
- [ ] 3.3 既有「重连回放」「journal 环形缓冲」「写占用」场景回归绿。

## 4. 文档与验证

- [ ] 4.1 `crates/xylitol-serve/src/AGENTS.md` 补槽生命周期与水位约束。
- [ ] 4.2 `cargo test --all-features --test bdd` 绿。
- [ ] 4.3 `llman-sdd validate --all --strict` 绿。
- [ ] 4.4 `just qa` 绿。
