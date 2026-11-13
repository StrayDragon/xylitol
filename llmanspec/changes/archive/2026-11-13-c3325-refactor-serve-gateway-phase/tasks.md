---
depends_on: []
title: "Consolidate serve gateway phase encoding"
needs_specs_change: false
---

# Tasks — c3325 Serve 网关相位编码收敛

T2→T3→T4→T5 同文件串行接力；T1 独立先行。

- [x] T1 A18 删 ReverseRpcResult::Timeout 变体，host.rs 两处 pattern 改 `Ok(Err(_)) | Err(_)`。验收：`cargo nextest run -p xylitol-serve` + `cargo test --test bdd` + `cargo clippy -p xylitol-serve --all-targets` 绿。
- [x] T2 A8 Phase 增 Stopping + repr(u8) 单一映射（from_u8 未知值显式错误）；mark_shutting_down 上移为 Stopping 转移点（覆盖 shutdown_signal）。blocked_by: []
- [x] T3 A8 status_str() 单点，render_unavailable / healthz 共用；新增 stopping 字符串单测。blocked_by: [T2]
- [x] T4 A9 ServeLifecycle u8 映射承载合一，删下划线兜底。blocked_by: [T2]
- [x] T5 A19 shutdown/Drop 收尾合一（remove_registration 保留收尾路径，30s 提常量）；停机后注册文件删除断言。blocked_by: [T2, T3, T4]
- [x] T6 收口：`llman-sdd validate c3325 --strict` 绿、`just fmt && just lint && just test && just qa` 绿（WASI_SDK_PATH 导出）；healthz BDD 值不变回归。blocked_by: [T5]
