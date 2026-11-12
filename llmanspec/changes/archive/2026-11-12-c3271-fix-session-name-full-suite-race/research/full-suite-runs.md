# 全量套件验收：连续 5 次零失败（2026-11-12）

命令（每次）：`just test`（cargo nextest run --workspace --all-features --profile agent，排除 package(xylitol-tui)）
环境：`WASI_SDK_PATH=~/.local/share/wasi-sdk-24`、`WASI_SDK` 同路径；分支 `sdd/c3271-fix-session-name-full-suite-race`。
历史失败率：~40-50%（`session_and_host_capabilities_match_in_process_carrier` 读回 None）。

| run | 结果 | 摘要 |
|---|---|---|
| 1 | 通过 | 3777 tests run: 3777 passed (1 slow, 5 leaky), 54 skipped [65.004s] |
| 2 | 通过 | 3777 tests run: 3777 passed (1 slow, 5 leaky), 54 skipped [64.978s] |
| 3 | 通过 | 3777 tests run: 3777 passed (1 slow, 5 leaky), 54 skipped [65.318s] |
| 4 | 通过 | 3777 tests run: 3777 passed (1 slow, 5 leaky), 54 skipped [65.273s] |
| 5 | 通过 | 3777 tests run: 3777 passed (1 slow, 5 leaky), 54 skipped [65.194s] |

修复前基线（同一树、修复前）：3772 tests / 3772 passed [65.093s]（单次通过不代表修复，历史 ~50% 失败）。
修复后用例数 3777 = 3772 + 5（新增：读路径不截断、完整尾保数据、无 id 行收纳、并发读者零丢失、create 后 set→get）。
5 连绿置信：历史 ~50% 失败率下 >97%。

## 根因复核（T1 结论摘要）

原「无 id 行 → 投影丢弃」链不成立；真实根因 = 读路径 `quick_check` 在文件尾无换行时截断（丢弃在写行）+ `append_persisted` 两写窗口 + `materialize_writer` 启动的 automation 首 tick 并发读。修复：读路径只读、写路径单次写入、完整 JSON 尾只补换行、维护命令在持锁时修复。详见 `../design.md` §0。
