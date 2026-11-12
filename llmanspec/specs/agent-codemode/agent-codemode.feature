# language: zh-CN
# capability: agent-codemode
# purpose: codemode（PTC）运行时：工具暴露、catalog、声明、嵌套 dispatch、Wasmtime 隔离与限额。
# scope: 只覆盖 run_code 工具与工具暴露层；不做 provider 原生 PTC、mode only、多语言、嵌套事件树。

功能: agent-codemode

  @req:r3250
  规则: exposure-and-catalog
    XyTool 须可声明 exposure（direct/model-only/codemode/deferred/hidden），默认 direct；模型工具表 须只含 direct 与 model-only；codemode catalog 须含 direct/codemode/deferred，不得含 model-only/hidden。catalog 须按会话工具集、能力开关、trust 过滤，并复用 ToolSet freeze 快照。

    场景: exposure 决定模型与 codemode 可见面
      假如 工具集含 direct/model-only/codemode/deferred/hidden 工具
      当 生成模型工具表与 codemode catalog
      那么 模型只含 direct 与 model-only 且 catalog 含 direct/codemode/deferred

  @req:r3251
  规则: tool-exposure-intersection
    codemode catalog 与 dispatch 须与 loop_policies.tool_exposure 的有效模型可见集取交集；allow/deny/read_only 拒绝的工具 不得出现在 catalog，也不得被 dispatch 调用。

    场景: tool-exposure 拒绝的工具不进入 catalog
      假如 loop_policies.tool_exposure 拒绝 write
      当 生成 codemode catalog
      那么 catalog 不含 write

  @req:r3252
  规则: declaration-budget
    声明 须按 catalog 生成；deferred 工具 不得内联；inline 声明 须受 inline_budget 约束；超预算时 须退回 ALL_TOOLS/searchTools/describeTool 发现。

    场景: deferred 工具不内联
      假如 catalog 含 deferred 工具
      当 生成声明
      那么 声明不含该工具且 ALL_TOOLS 可发现

  @req:r3253
  规则: nested-dispatch
    嵌套调用 须复用 before_tool / after_tool / 权限 / hooks；不得递归 run_code；不得把 model-only/hidden 暴露给脚本；只有外层 run_code 结果 须进入模型上下文与 session。

    场景: 嵌套调用复用权限且拒绝递归
      假如 run_code 脚本调用 write 与 run_code
      当 执行脚本
      那么 write 经 before_tool 与权限且 run_code 被拒绝

  @req:r3254
  规则: wasmtime-limits
    run_code 须在 Wasmtime 隔离中执行；guest 不得访问 FS/网络；fuel、epoch 取消/超时、内存上限 须映射为可分类错误；输出 须受 max_result_bytes 限制。

    场景: fuel 耗尽与取消映射为错误
      假如 run_code 执行无限循环或收到取消
      当 执行脚本
      那么 返回 fuel_exhausted 或 cancelled 错误

  @req:r3255
  规则: zero-assembly-and-cache
    codemode 未配置或关闭时 不得注册 run_code、声明中间件或 catalog 服务；模块缓存字节 须可信并按版本/target/guest hash 校验；缓存失败 须回退编译。

    场景: 未配置 codemode 零装配
      假如 配置没有 codemode 节
      当 装配工具与中间件
      那么 不含 run_code 且无 codemode 中间件

  @req:r3264
  规则: declarations-from-runtime-catalog
    codemode 声明 须由每次 LLM 请求前的运行期工具 catalog 快照生成；该快照 须与 guest `ALL_TOOLS`、dispatch catalog 同源，并反映 session 工具、`tools` `+` / `-` 选择、tool-exposure 与 exposure overrides。不得使用插件 `apply` 时的 `HostRegistries` 快照；声明 不得列出被禁用或不可见工具；catalog 为空时声明为空。

    场景: session 工具进入声明与 catalog
      假如 catalog 含 session 工具
      当 生成声明
      那么 声明含该工具且 ALL_TOOLS 同源

    场景: 禁用工具不出现在声明
      假如 tools 选择禁用 bash
      当 生成声明
      那么 声明不含 bash 且 catalog 不含 bash

    场景: tool-exposure 过滤同时作用于声明
      假如 catalog 已按 tool-exposure 过滤掉 write
      当 生成声明
      那么 声明不含 write 且 catalog 不含 write

    场景: 空 catalog 声明为空
      假如 catalog 为空
      当 生成声明
      那么 声明为空

  @req:r3265
  规则: activation-selection-orthogonal
    `codemode.enabled` 是唯一激活开关；`tools` 的 `+` / `-` 只做选择，不得隐式激活插件。`enabled` + `-run_code` 合法且不得创建 engine；`+run_code` 但未启用 须输出启动 warning；`RunCodeTool` 首次调用才创建 runner。

    场景: enabled 禁用 run_code 合法
      假如 codemode 启用且 tools 禁用 run_code
      当 解析工具选择
      那么 run_code 不可见且无冲突 warning

    场景: 未启用但选择 run_code 告警
      假如 codemode 未启用且 tools 选择 run_code
      当 检查配置
      那么 输出 codemode disabled warning

    场景: 首次调用才创建 runner
      假如 run_code 工具已注册但未调用
      当 首次调用 run_code
      那么 才创建 runner 且第二次复用

  @req:r3266
  规则: guest-wasm-source-build
    codemode guest wasm 须由源码在构建时生成并嵌入 host；不得提交 wasm、不得下载 release artifact。`build.rs` 与 `just codemode-guest` 须共用 WASI SDK 24 与同一 target / env 参数；缺 SDK 须构建失败并提示；`OUT_DIR/guest.wasm` 由 `include_bytes!` 嵌入；`module_cache` 行为不变。

    场景: 源码构建生成 guest wasm
      假如 codemode guest 源码与 WASI SDK 24 可用
      当 构建 codemode 插件
      那么 OUT_DIR 生成 guest.wasm 且 host 嵌入

    场景: 缺 WASI SDK 失败
      假如 WASI_SDK_PATH 缺失
      当 构建 codemode 插件
      那么 构建失败并提示 WASI SDK 24

  @req:r3347
  规则: codemode-submission-isolation
    run_code 每次提交的取消与超时 须 相互隔离：一个提交超时或取消 不得 中断仍在执行的并发提交；中断结果 须 可归因到具体提交。由单测覆盖。
