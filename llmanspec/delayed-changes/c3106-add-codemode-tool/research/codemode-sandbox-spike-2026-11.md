# Codemode sandbox spike (2026-11)

## 结论

本轮 spike 范围内：

- **JS codemode 首选 rquickjs WASI + Wasmtime 50 stable**：rquickjs 0.14 编译到 `wasm32-wasip1`，Wasmtime 50 提供 fuel / epoch / memory；guest 1.1MB，eval 与 fuel trap 已验证。它替代已 deprecated 的 `quickjs-wasm-rs`，且比 `jsdet-core` fork 更可维护。
- **QuickJS WASM（jsdet-core fork）** 可作 fallback / 参考：WASM 隔离与 fuel 已验证；但发布版 crate 编译失败，Bridge 同步，timeout 非硬中断。
- **Boa 0.22** 适合 trusted / prototype：async host call 最省事，但没有 heap limit，进程内执行，不是安全边界。
- **Python codemode**：**Monty 1.4** 纯 Rust、启动快、host 函数 pause/resume；进程内执行，crash isolation 用 `monty-pool`。**Eryx 0.14** 是 Python WASM，async callback、resource limits、硬取消；但 runtime 较重。
- **Wasmtime 50 stable** 是默认 sandbox substrate：fuel / epoch / memory + async host call；**Wasmi 2.0** 是纯 Rust interpreter fallback；**Wasmer** 适合 WASIX；**WasmEdge** 适合 AOT/AI；**wasm3** 不适合安全关键 codemode。
- **MicroQuickJS** 是 ES5 子集、10KB RAM 级引擎，不适合现代 codemode。
- `quickjs-wasm-rs` 已 deprecated；`rquickjs` 是高层绑定与 WASI 方向的继任者。

## 环境

- rustc 1.101.0-nightly；cargo 1.101.0-nightly。
- 临时目录 `/tmp/xylitol-codemode-spike`，独立 Cargo 项目：`boa` / `quickjs` / `wasmtime` / `rquickjs-guest` / `rquickjs-host` / `wasmi-host` / `monty` / `eryx`；不写主仓。
- `boa_engine` 0.22.0（default-features off）；`tokio` 1.54.1。
- `jsdet-core` 0.1.1（local fork；wasmtime 43.0.2；embedded `quickjs.wasm` 1.2MB）。
- `rquickjs` 0.14.0（QuickJS-NG；`full-wasi`）；`wasmtime` 43.0.2 / 50.0.1；`wasmtime-wasi` 43.0.2 / 50.0.1；`wasmi` 2.0.0；`wasmi_wasi` 2.0.0；`wasmer` 7.5.0；`wasmedge-sdk` 0.17.1；`wasm3` 0.3.1（仅调研）。
- `monty` 1.4.0；`monty-types` 1.4.0。
- `eryx` 0.14.0（`embedded` / `macros`；wasmtime 50.0.1）。
- 无 wasmtime CLI、emcc、wasm-tools；未使用 WASI SDK。
- `quickjs-wasm-rs` 3.1.0 已 deprecated；未采用。
- `eryx-precompile` 0.14.0 安装后执行 `setup`：runtime.wasm 11.7MB，precompiled 32.3MB，缓存 `~/.cache/eryx`。

## 测试结果

### Boa 0.22

```rust
let executor = Rc::new(SimpleJobExecutor::new());
let mut context = Context::builder().job_executor(executor.clone()).build()?;
context.register_global_builtin_callable(
    js_string!("tool"),
    2,
    NativeFunction::from_async_fn(tool),
)?;
executor.clone().run_jobs_async(&RefCell::new(&mut context)).await?;
```

current-thread tokio + `LocalSet`；`Context` 是 `!Send`。

- `async_state=Fulfilled(JsValue(String(JsString("ok"))))`；`async_elapsed_ms=14`。
- `sync_result=3`。
- `loop_limit_err=RuntimeLimitError: reached the maximum number of iteration loops on this execution`。
- `recursion_limit_err=RuntimeLimitError: reached the maximum number of recursive calls on this execution`。
- `cancel_before_jobs=Ok("ok")`。

判断：异步 host call 直接可用；`RuntimeLimits` 只有 loop / stack / recursion / backtrace，无 heap limit；cancellation token 只在 job 边界生效；进程内，不是安全边界。

### QuickJS WASM（jsdet-core fork）

发布版 `jsdet-core` 0.1.1 编译失败（`vulnir` / `petgraph` / `SourceLocation`）；临时 fork 去掉 `vulnir_producer`，保留执行路径，补 `secfinding 0.4.3`。

- `compile_ms=1854`；`simple` 2.7ms；`tool` 2.2ms。
- `max_fuel=10M`：wasm trap（`js_call_c_function`）；`timed_out=false`。
- `timeout_ms=10` + `max_fuel=5M`：wasm trap（`JS_CallInternal`）；`timed_out=false`。
- `max_memory_bytes=8MB`：wasm trap（`JS_SetPropertyValue`）。
- Bridge 是同步 trait；JS bootstrap 用 `__jsdet_bridge_call(api, JSON.stringify(args))`。
- `timeout_ms` 只在脚本前后与 timer drain 检查；纯 JS 循环不会被 timeout 中断；fuel 是硬限制。

判断：WASM 隔离与 trap 后宿主存活已验证；同步 Bridge 限制 async tool dispatch；发布 crate 不可直接用。

### rquickjs WASI + wasmtime 43

Guest：

```rust
// rquickjs-guest/src/lib.rs
#[unsafe(no_mangle)]
pub extern "C" fn eval(ptr: *const u8, len: usize) -> i32 { ... }
```

```toml
[dependencies]
rquickjs = { version = "0.14", features = ["full-wasi"] }
```

构建：

```bash
cargo +nightly-2026-10-09 build --target wasm32-wasip1 --release
```

Host：wasmtime 43 + wasmtime-wasi 43；`WasiP1Ctx` + `add_to_linker_sync` + `inherit_stdio` + `Store::set_fuel`。

结果：

- guest wasm 1,192,274 bytes；release 构建 49.97s。
- `eval_ok=0`（`1 + 2`）；`eval_bad=1`（语法错误）。
- `fuel_spin=Err("wasm trap: all fuel consumed by WebAssembly")`；`fuel_before=95,548,585`，`fuel_after=0`。

判断：rquickjs 0.14 可用 WASI 目标；QuickJS-NG 活跃；wasmtime 提供 fuel / epoch / memory；host import 与 async dispatch 尚未原型，但架构可行；guest eval 目前只返回状态码，codemode 结果通道需扩展。`quickjs-rs`（langchain-ai）采用同一路线：rquickjs `wasm32-wasip1` + wasmtime。

### Wasmtime 43 substrate

WAT 模块含 memory / spin / grow / host_call；`consume_fuel`、`epoch_interruption`、`StoreLimitsBuilder`、`func_wrap_async`。

- `fuel_err=Some(trap backtrace)`。
- `epoch_err=Some(trap backtrace)`。
- `memory_grow=Ok(-1)`（64KiB StoreLimits）。
- `async_result=8`（`func_wrap_async` + `call_async`）。
- `Config::async_support` 在 43 已 deprecated（no longer has any effect）。

判断：硬性 fuel / epoch / memory；async host call 可用；不是 JS 引擎。

### Monty 1.4

纯 Rust Python 子集解释器；`MontyRun` 编译一次；`ResourceTracker` 限制 feed time / recursion；host 函数通过 `RunProgress::FunctionCall` pause，`resume` 继续；支持 snapshot。

- `fib(10)=55`；`fib_elapsed_us=10,631`。
- host call：`get_data(3)` -> resume 21 -> `42`；`host_elapsed_us=756`。
- `while True` + 20ms feed limit -> `TimeoutError: feed time limit exceeded: 20.085458ms > 20ms`；`limit_elapsed_us=20,305`。

判断：启动快、host 函数模型适合 CodeAct；进程内执行，crash isolation 用 `monty-pool`（subprocess pool）；memory limits 需 `monty-alloc`；不是 JS。

### Eryx 0.14

Python WASM；`Sandbox::embedded()`；async callback；`ResourceLimits`；`execute_cancellable`。

- `build_ms=97`（warm；首次 350ms）。
- `stdout="hello\n"`；`structured={"answer": 42}`。
- async callback `tool(name='read')` -> `{'ok': True, 'tool': 'read'}`。
- `execution_timeout=20ms` -> `Timeout(20ms)`，wall 138ms。
- `execute_cancellable` + 50ms cancel -> `Cancelled`。
- `max_memory_bytes=16MB` + `[0]*10_000_000` -> Python `MemoryError`；stats peak 52MB（>16MB，limit 语义待确认）。

判断：WASM 隔离、async callback、resource limits、硬取消可用；runtime 11.7MB + cwasm 32.3MB，较重；不是 JS。

## 其他方案调研（未测）

- **rquickjs 0.14**：QuickJS-NG 高层绑定；支持 native 与 `wasm32-wasip1/wasip2`；无内置 sandbox，需 wasmtime。本轮已测 WASI guest。
- **quickjs-wasm-rs 3.1**：已 deprecated；README 推荐 rquickjs；需要 WASI SDK。
- **deno_core 0.413**：V8；`quickjs` feature 是 experimental QuickJS backend（`v8x`）；进程内，不是安全边界；二进制重。
- **rustyscript 0.12.3**：V8/deno_core；默认 sandboxed（无 FS/网络），支持 timeout、async、worker；仍是进程内 V8。
- **js-sandbox 0.2.0-rc.2**：Deno/V8；secure sandbox；timeout；早期开发。
- **javy 8.1.0**：基于 rquickjs；把 JS 编译为 WASM；支持 eval source / bytecode；工具链形态。
- **wasmi 2.0**：纯 Rust WASM 解释器；fuel；deterministic；已用同一 rquickjs guest 实测；比 Wasmtime 慢很多（eval 227ms vs 183us，10 次平均）。
- **MicroQuickJS**：ES5 子集、10KB RAM 级；不支持 arrow / let / const 等现代语法；不适合 codemode。
- **jsdet-core 0.1.1**：QuickJS WASM + wasmtime；发布 crate 编译失败；同步 Bridge；作参考。

## WASM runtime 选型

需求：Rust host；WASM 边界；JS 优先；给其他语言留口；CPU / 内存 / 取消 / async host call；Component Model / WASI。

| Runtime | 实现 | 许可 | 关键能力 | 取舍 |
|---|---|---|---|---|
| **Wasmtime 50.0.1** | Rust；Cranelift JIT | Apache-2.0 WITH LLVM-exception | Component Model + WASI p2 + async；fuel / epoch / memory；C API；Fastly / Shopify | 默认；JIT 编译约 2.05s/1.1MB guest；需跟踪 advisory |
| **Wasmi 2.0** | 纯 Rust interpreter | MIT / Apache-2.0 | fuel；WASI p1；deterministic；audited；no JIT | 冷启动快；执行慢；无 Component Model / async；embedded |
| **Wasmer 7.5** | Rust；Cranelift / LLVM / Singlepass | MIT | WASIX；WASI；JIT | WASIX / POSIX；标准 Component Model 不如 Wasmtime；API / advisory 历史 |
| **WasmEdge 0.17** | C++ runtime；Rust SDK | Apache-2.0 | AOT；WASI-NN；WASI；Component Model 进行中 | 非纯 Rust；AI / edge |
| **wasm3 0.3** | C interpreter；Rust bindings | MIT | tiny interpreter | WASI / limit 支持弱；不适合安全关键 |
| WAMR / Wazero | C / Go | — | 非 Rust | 不作为 Rust 首选 |

单次实测（同一 rquickjs WASI guest，1,192,274 bytes；平均值见下节）：

| Host | module 编译 | instantiate | eval `1+2` | fuel trap |
|---|---|---|---|---|
| Wasmtime 50.0.1 | 1876ms | 0ms | 644us | yes |
| Wasmi 2.0 | 95.3ms | 6.4ms | 281ms | yes |

解读：Wasmi 冷启动快（compile 69.7ms vs 2.05s），但 QuickJS WASM 解释执行慢很多（tiny 227ms vs 183us；loop 316ms vs 167us）；227ms 主要是每次 eval 新建 QuickJS Runtime/Context，tiny/json 几乎相同，loop 只多约 89ms。Wasmtime JIT 编译一次后执行快，模块缓存可摊销编译；Wasmi 适合 deterministic / embedded / 冷启动优先；Wasmtime 适合重复 codemode / 语言无关。

安全：

- Wasmtime 2026-04 有两个 Critical sandbox escape 公告；后续还有 filesystem / aarch64 advisory；必须 pin 最新 patch、跟踪 RustSec / GHSA。
- Wasmi 审计过两次（0.31 / 0.36-0.38），但也有 advisory 历史；审计不等于零漏洞。
- Wasmer 有 memory-safety / WASI advisory 历史。
- 没有“零漏洞” runtime；WASM 边界 + 及时补丁 + 必要时进程隔离。

语言无关：

- 定义 guest ABI：导出 `run` / `eval`，host import `tool_call`；用 WASI p1 或 Component Model / WASI p2。
- JS：rquickjs `wasm32-wasip1`。
- Python：Eryx（WASM）；Monty 不是 WASM，只能 trusted / prototype 或 monty-pool 进程隔离。
- 其他语言：Rust / C / C++ / Go / TS 编译为 WASM core module / component；Wasmtime Component Model 提供 typed interface。
- Wasmtime Component Model / WASI p2 是语言无关方向的最强支持；Wasmi 只有 p1，Wasmer 以 WASIX 为主。

## 代码库活跃度（GitHub，2026-11-05）

| Repo | Stars | Forks | Contributors | Commits 90d | Releases 12m | Open issues+PRs | Issue close | PR merge | Last push |
|---|---|---|---|---|---|---|---|---|---|
| bytecodealliance/wasmtime | 18,824 | 1,914 | 865 | 648 | 84 | 871 | 80.0% | 91.4% | 2026-11-05 |
| wasmi-labs/wasmi | 2,341 | 379 | 88 | 18 | 31 | 46 | 92.8% | 90.6% | 2026-11-04 |
| wasmerio/wasmer | 21,221 | 1,035 | 276 | 288 | 24 | 268 | 91.4% | 79.4% | 2026-11-05 |
| WasmEdge/WasmEdge | 10,856 | 1,206 | 278 | 190 | 39 | 178 | 93.9% | 79.4% | 2026-11-04 |
| wasm3/wasm3 | 8,078 | 537 | 79 | 84 | 2 | 21 | 96.4% | 72.4% | 2026-09-30 |
| DelSkayn/rquickjs | 1,059 | 157 | 64 | 53 | 0 | 68 | 81.6% | 72.2% | 2026-11-05 |

指标来自 GitHub API；`open_issues_count` 含 issue + PR；issue / PR 比例为生命周期累计；`releases_12m` 只计 GitHub Releases（rquickjs 的版本在 crates.io）。

解读：

- Wasmtime 绝对活跃度最高：648 commits/90d、84 releases/12m、865 contributors；PR merge 91.4%，但 open issues+PRs 871，issue close 80.0%。
- Wasmi 体量小但处理比例高：18 commits/90d、88 contributors、open 46；issue close 92.8%、PR merge 90.6%。
- Wasmer / WasmEdge 活跃；open PR 较多，PR merge 约 79.4%。
- wasm3 发布节奏低（2 releases/12m），issue close 高但 PR merge 72.4%。
- rquickjs 提交活跃（53 commits/90d），无 GitHub Release；issue / PR 比例较低（81.6% / 72.2%）。

## 本机多次实验平均值

同一 rquickjs WASI guest；每脚本 10 次、compile 3 次；fuel 1B；单位 us。

| Host | compile avg | instantiate avg | tiny avg | json avg | loop avg |
|---|---|---|---|---|---|
| Wasmtime 50.0.1 | 2,049,423 | 61 | 183 | 136 | 167 |
| Wasmi 2.0 | 69,650 | 6,370 | 227,171 | 227,908 | 315,811 |

脚本：tiny = `1 + 2`；json = `JSON.stringify({a:1,b:2}).length`；loop = `let s=0; for(let i=0;i<1000;i++){s+=i}; s`。

冷启动（compile + instantiate + tiny）：

- Wasmtime 50：2,049,423 + 61 + 183 = 2,049,667us ≈ 2.05s。
- Wasmi 2.0：69,650 + 6,370 + 227,171 = 303,191us ≈ 303ms。
- Wasmi 冷启动约快 6.8x。

热启动（模块复用，instantiate + tiny）：

- Wasmtime 50：61 + 183 = 244us。
- Wasmi 2.0：6,370 + 227,171 = 233,541us ≈ 233.5ms。
- Wasmtime 热启动约快 957x。

解读：Wasmi 的 227ms 不是纯 JS 算术执行；tiny 与 json 几乎相同，loop 只多约 89ms，说明每次 `eval` 新建 QuickJS Runtime/Context 的初始化占大头。Wasmtime JIT 把这段初始化压到 183us；Wasmi 解释器则放大到 227ms。若 codemode 每次只跑一个短脚本且 LLM 延迟占主导，227ms 不是瓶颈；若一轮内多次 tool call / eval，227ms 会累积。

## 对比

| 维度 | Boa 0.22 | jsdet QuickJS WASM | rquickjs WASI | Wasmtime 50 | Monty 1.4 | Eryx 0.14 |
|---|---|---|---|---|---|---|
| 语言 | JS | JS | JS | WASM | Python 子集 | Python |
| 隔离 | 进程内 capability | wasmtime | wasmtime | wasmtime | 进程内；pool 子进程 | wasmtime |
| 内存上限 | 无 heap limit | `max_memory_bytes` trap | wasmtime StoreLimits | StoreLimits | `monty-alloc` | `max_memory_bytes`（语义待确认） |
| CPU 上限 | loop / recursion | fuel 硬；timeout 非硬 | fuel / epoch | fuel / epoch | feed time | timeout / fuel |
| 取消 | job 边界 token | fuel / timeout | epoch | epoch | abort / limits | `execute_cancellable` |
| async host call | 原生 async | 同步 Bridge | host import / worker | `func_wrap_async` | pause/resume（同步） | async callback |
| 启动 / 构建 | 未测 | 编译 1.85s | guest 构建 50s；1.1MB | module 2.05s；eval 183us（avg） | 10.6ms fib | 97ms warm；11.7MB runtime |
| 适用 | trusted / prototype | fallback | untrusted JS 首选 | 默认 substrate | Python CodeAct 快速路径 | Python WASM 隔离 |

## 建议

- **生产 untrusted codemode**：Wasmtime 50.0.1 + rquickjs WASI guest；fuel / epoch / memory；host import / async dispatch；缓存模块；跟踪 advisory。
- **Python**：Eryx（WASM）优先；Monty 不是 WASM，只用于 trusted / prototype，或 monty-pool 进程隔离。
- **其他语言**：编译为 WASM core module / component；Wasmtime Component Model / WASI p2；定义 guest ABI。
- **fallback**：Wasmi 2.0（纯 Rust interpreter、deterministic、audited；WASI p1；compile 69.7ms、eval 227ms）；jsdet-core fork 仅参考。
- **冷启动优先**：若每次只跑一个短脚本且执行不是瓶颈，Wasmi 冷启动 303ms vs Wasmtime 2.05s；但需评估多次 eval 累积与缺少 Component Model / async。
- **Wasmer**：仅当需要 WASIX / POSIX；**WasmEdge**：仅当需要 AOT / WASI-NN；**wasm3**：不适合安全关键。
- **Boa**：只作 trusted / prototype；不是 WASM，不用于 untrusted。
- **V8 方案**（deno_core / rustyscript / js-sandbox）：只在必须完整 V8 兼容时考虑；用进程隔离。
- 安全：pin Wasmtime 50.0.1；跟踪 RustSec / GHSA；必要时进程隔离。

## 待确认

- rquickjs guest 的 host import / async tool dispatch / 死锁风险。
- rquickjs guest 的 memory limit / epoch cancel 原型。
- Eryx `max_memory_bytes` 语义：16MB 限制下 peak 52MB，需复核。
- Monty memory limits / `monty-pool` crash isolation / snapshot。
- deno_core / rustyscript / js-sandbox 未测；进程隔离成本。
- MicroQuickJS 的现代语法限制。
- rquickjs / QuickJS-NG 许可与维护。
- Wasmtime 50 Component Model / WASI p2 guest ABI；Wasmi 无 Component Model / async；Wasmer / WasmEdge 未测。
- Wasmtime / Wasmi / Wasmer 安全公告与 patch 节奏；Wasmtime 模块缓存。
- Wasmi 对较大 codemode 脚本的性能。

## 参考

- [rquickjs](https://github.com/DelSkayn/rquickjs)
- [QuickJS-NG](https://quickjs-ng.github.io/quickjs/)
- [quickjs-rs (rquickjs + wasmtime)](https://github.com/langchain-ai/quickjs-rs)
- [quickjs-wasm-rs deprecation](https://github.com/bytecodealliance/javy/pull/618)
- [Javy](https://github.com/bytecodealliance/javy)
- [deno_core](https://github.com/denoland/deno)
- [rustyscript](https://github.com/rscarson/rustyscript)
- [js-sandbox](https://github.com/Bromeon/js-sandbox)
- [wasmi](https://github.com/wasmi-labs/wasmi)
- [MicroQuickJS](https://github.com/bellard/mquickjs)
- [Monty](https://github.com/pydantic/monty)
- [monty-pool](https://github.com/pydantic/monty/blob/main/crates/monty-pool/README.md)
- [Eryx](https://github.com/eryx-org/eryx)
- [Wasmtime](https://github.com/bytecodealliance/wasmtime)
- [Wasmtime security advisories](https://bytecodealliance.org/articles/wasmtime-security-advisories)
- [Wasmi](https://github.com/wasmi-labs/wasmi)
- [Wasmer](https://github.com/wasmerio/wasmer)
- [WasmEdge](https://github.com/WasmEdge/WasmEdge)
- [wasm3](https://github.com/wasm3/wasm3)
- [WASM runtime comparison](https://github.com/wasmruntime-io/wasm-runtime-comparison)
- [WASI Preview 2 vs WASIX](https://wasmruntime.com/en/blog/wasi-preview2-vs-wasix-2026)

## 命令

```bash
cd /tmp/xylitol-codemode-spike/boa && cargo run --quiet
cd /tmp/xylitol-codemode-spike/quickjs && cargo run --quiet
cd /tmp/xylitol-codemode-spike/rquickjs-guest && cargo +nightly-2026-10-09 build --target wasm32-wasip1 --release
cd /tmp/xylitol-codemode-spike/rquickjs-host && cargo run --quiet
cd /tmp/xylitol-codemode-spike/wasmi-host && cargo run --quiet
cd /tmp/xylitol-codemode-spike/rquickjs-host && cargo run --quiet --bin bench
cd /tmp/xylitol-codemode-spike/wasmi-host && cargo run --quiet --bin bench
cd /tmp/xylitol-codemode-spike/wasmtime && cargo run --quiet
cd /tmp/xylitol-codemode-spike/monty && cargo run --quiet
cd /tmp/xylitol-codemode-spike/eryx && cargo run --quiet
```
