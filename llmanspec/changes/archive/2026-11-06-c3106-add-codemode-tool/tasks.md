# Tasks — Codemode（Wasmtime）工具与工具暴露层

> 执行顺序按 T1 → T10。每完成一项立即勾选；收口（verify / finalize / archive）不在本文件。

- [x] T1 SDK 契约：新增 `XyToolExposure`、`XyToolDescriptor`、`XyToolCallResult`、`XyToolCatalog`、`XyToolDispatch`；`XyTool` 增加默认 `exposure` / `namespace` / `output_schema`；`XyToolCtx` 增加可选 dispatch。验证：`cargo test -p xylitol-plugin-sdk`。
- [x] T2 Runtime catalog：新增 catalog/identifier 模块；按 ToolSet freeze 快照生成 descriptor；`ToolTableFingerprint` 覆盖 exposure/namespace/output_schema；与 `loop_policies.tool_exposure` 取交集。验证：`cargo test -p xylitol-runtime`。
- [x] T3 Runtime dispatch：`DispatchHandle` 复用 `prepare_call` / `run_one`；拒绝 run_code/Hidden/ModelOnly；递归与嵌套上限；嵌套 Start/End 抑制；dispatch 注入 `XyToolCtx`；`SandboxCapability::Exec` 映射 `check_process`。验证：`cargo test -p xylitol-runtime`。
- [x] T4 Kernel/插件装配：新增 `plugins/xylitol-official-plugin-codemode`；配置节 `codemode`；未配置零装配；`compose_tools` 注册 `run_code`；注入 catalog/dispatch；`workspace-assembly` 列表。验证：`cargo test -p xylitol-kernel -p xylitol-official-plugin-config`。
- [x] T5 Codemode host runner：Wasmtime 50 Engine/Linker/Store；WASI p1；`func_wrap_async` host call；fuel/epoch/memory；模块编译/缓存；错误映射。先验证 sync export + async import；不支持则按 design fallback 记录 blocker。验证：`cargo test -p xylitol-official-plugin-codemode`。
- [x] T6 Guest crate/asset：`plugins/xylitol-official-plugin-codemode/guest`；rquickjs 0.14 WASI；导出 alloc/dealloc/eval/result；globals（tools/text/console/ALL_TOOLS/searchTools/describeTool）；预编译 `guest.wasm` 资产；`just codemode-guest` 与 hash 校验。验证：guest 单测 / host ABI smoke。
- [x] T7 run_code 工具：schema（code/description/timeoutMs）；声明生成（BeforeLlmRequest，order > tool-exposure）；inline budget；deferred；输出截断；外层结果落盘。验证：插件单测 + fake model 集成。
- [x] T8 Specs：新增 `agent-codemode.feature`；更新 agent-tools（exposure/run_code 不进 7 工具闭集）、agent-runtime（嵌套复用/事件/取消）、domain-security（Exec/凭据）、runtime-config（codemode 节）、workspace-assembly（插件列表）。验证：`llman-sdd validate --specs`。
- [x] T9 测试/BDD：catalog/identifier/declaration；dispatch 递归拒绝；tool-exposure 交集；freeze fingerprint；fuel/epoch/memory/输出；零装配；模块缓存；BDD 场景与 step 绑定。验证：`cargo nextest run --all-features --test bdd`。
- [x] T10 门禁/文档：根 `Cargo.toml` / `Cargo.lock` / `configs/config.schema.json` / `AGENTS.md` / `src/AGENTS.md`；`just fmt` / `just lint` / `just test` / `just qa`；残留检查。验证：全绿。
