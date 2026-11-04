# Codemode 沙箱安全对比（2026-11）

> 性质：change 附属调研，不是 live spec。
> 触发：Boa in-process 是否缺少沙箱、是否应改 WASM。

## 结论

- Boa in-process 是能力沙箱（只暴露 host 函数），不是隔离边界；引擎 bug/panic/OOM 可影响 host；无 heap limit、无 hard interrupt。
- WASM（QuickJS WASM / Eryx）提供线性内存隔离、fuel/memory/timeout、崩溃隔离；host 函数仍受 permission 约束。
- 若 codemode 会执行不可信/被注入的代码，建议 QuickJS WASM；若只对可信模型开放，Boa 的风险可接受但要文档化。
- catalog/exposure/dispatch 设计与 runtime 无关；切 WASM 只改 runtime 段。

## Boa 风险

- 无 FS/network/process/timers；脚本只经 host 函数接触外部；工具权限仍生效。
- in-process：Boa panic/unsafe bug 可能崩溃 host；无 crash isolation。
- RuntimeLimits 只有 loop/recursion/stack；无 heap limit；OOM 可 abort。
- 无 hard interrupt；timeout 靠协作/loop limit；tight loop 可能跑到上限。
- codemode 不扩大工具能力（bash 已可任意命令），但扩大攻击面/DoS。

## WASM 候选

| 方案 | 隔离 | host 调用 | 备注 |
|---|---|---|---|
| jsdet-core | QuickJS WASM + wasmtime；fuel/memory/timeout；零 syscall | Bridge trait；同步 execute | 0.1.x；安全分析向；需 spike |
| quickjs-wasm-rs + wasmtime | QuickJS WASM；wasmtime 限制 | wrap_callback；同步 | Javy/pi 路线；需构建 wasm |
| Eryx | CPython WASI + wasmtime；async callbacks | 异步 | Python；语言切换 |
| Boa subprocess | 进程隔离；无内存隔离 | IPC | 需 OS sandbox；复杂 |
| Boa compiled to WASM | WASM 隔离 | 需自定义 host ABI | 未成熟；spike |

## 建议

- 切 QuickJS WASM（保留 JS）：jsdet-core 先 spike；若 Bridge 不适配，用 quickjs-wasm-rs + wasmtime。
- MVP host bridge 同步（顺序工具调用）；并行 nested calls 后续（promise bridge）。
- WASM 不替代 permission；host 函数仍走 dispatch/prepare_call/run_one。
- 文档明示：WASM 隔离引擎 bug/DoS，不隔离工具误用。

## 来源

- [jsdet-core](https://docs.rs/jsdet-core/latest/jsdet_core/)
- [quickjs-wasm-rs](https://docs.rs/quickjs-wasm-rs/latest/quickjs_wasm_rs/)
- [quickjs-wasm-rs + wasmtime example](https://github.com/seddonm1/quickjs)
- [Eryx](https://github.com/eryx-org/eryx)
- [Boa RuntimeLimits](https://docs.rs/boa_engine/0.22.0/boa_engine/vm/struct.RuntimeLimits.html)
