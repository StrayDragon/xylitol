# Tasks: c3340-refactor-session-slot

## 1. 一次转移

- [x] 1.1 按 `design.md` 把 `release_write_lock` 和 `evict_slot` 收成一个函数。`release_and_evict` 与 `linger_evict_if_idle` 在各自的前置条件之后调用它。水位仍写在 `HostState.seq_watermarks`。

## 2. 现有占用测试

seam：`tests/occupancy/` 里已有的释放、宽限和水位场景。不新增 BDD。

- [x] 2.1 这些场景保持通过。写锁释放、水位保留、回合进行中不驱逐仍然成立。

## 3. 临时评判（deslop-please）

不进 `just qa`。基线是 `.tmp/deslop-please.allow`。测试路径诊断见 `.tmp/deslop-please-deferred-tests.md`，本 change 不改。全仓基线只由不带 crate 参数的 `deslop-please --update-allow .tmp/deslop-please.allow` 重写。只检查一个 crate 时，其它 crate 的条目不在这次输出里，它们仍留在基线中。

`SessionSlot` 上已有的字段泄漏和 `host.rs` 的文件长度留在基线里。本 change 的完成判据是转移收成一处，加上现有占用测试。

- [x] 3.1 `deslop-please xylitol-serve --allow .tmp/deslop-please.allow` 没有基线以外的新诊断。
