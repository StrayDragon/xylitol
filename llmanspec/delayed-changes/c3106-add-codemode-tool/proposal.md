---
depends_on: []
title: "Add codemode tool and tool exposure model"
---

## Why

PTC（Programmatic Tool Calling，程序化工具调用）让模型写一段程序，程序内调用工具；harness 执行程序、分发嵌套工具调用，只把精简结果交回模型。它把多步工具调用从“多次模型往返”压成“一次程序执行”。

上一版提案把问题缩成“加一个 run_code 工具 + 固定 SDK”，这是路径依赖：真正难的是让模型理解并使用全部工具来源——内置、MCP、外进程插件、官方插件工具。pi codemode 的做法是工具暴露模型 + 声明生成 + 动态发现，而不是固定 SDK。

本 change 的目标：新增 codemode 工具，同时建立工具暴露/catalog 层；模型经 codemode 能调用所有被允许的工具；能力限制在 catalog 与 dispatch 两侧生效；不做强制模式，不做 provider 原生 PTC。

## What Changes

- 工具暴露元数据：XyTool 增加可选 exposure（direct/model-only/codemode/deferred/hidden）、namespace、output_schema、codemode identifier；默认 direct。
- Tool catalog 服务（runtime）：按会话工具集、能力开关、plan mode、permission、trust 过滤；生成 snapshot（name、identifier、namespace、description、schema、output_schema、exposure、guidelines）。
- 声明生成：codemode 插件按 snapshot 生成 JS（TypeScript 风格）声明；按 namespace 分组；inline budget；deferred 工具不内联，靠 ALL_TOOLS/searchTools/describeTool/describeNamespace 发现。
- dispatch 端口（XyToolDispatch）：runtime 实现；嵌套调用复用 prepare_call/run_one 的 hooks、loop 中间件、权限终裁与事件；禁止递归 run_code；并发上限；取消传播。
- run_code 工具：入参 code、description、可选 timeoutMs；语言 runtime 为 Boa 0.22（见 design.md）；只暴露工具绑定，不暴露 FS/网络；输出 text/return；结果上限；默认超时 120s、上限 600s。
- codemode.mode：on（默认，direct 工具仍声明）/ only（隐藏 direct 工具，全部经脚本）；首切片可只做 on。
- 权限：run_code 自身走 permission；嵌套调用逐次复用；子进程剥离保留凭据；文档明示 permission 非安全边界。
- 事件：首切片只发外层 ToolExecutionStart/Update/End；嵌套进度走 ToolExecutionUpdate；嵌套事件树后续。
- 会话：只把外层 run_code 结果写入模型上下文；嵌套结果不进 session history；store/load 不在 MVP。
- 测试：catalog/declaration/dispatch 单测；fake runtime；BDD（fake model 发 run_code）；取消、递归拒绝、结果上限、未配置零装配。

## Shape

| 形态 | 模型可见面 | 产品面 | 首切片 |
|---|---|---|---|
| tool | run_code + direct 工具 | 无模式状态/命令/ModeStrip | 本 change |
| mode | 只有 run_code（codemode.mode only） | 会话状态、/codemode、ModeStrip | 后续 |
| executor | 只有 run_code | 替换 native executor | 后续/不做 |

## Naming

- 工具名：run_code（推荐；动作清晰，和 DSH 对齐）。
- 能力/产品名：codemode（Code Mode，代码模式）；spec capability 可用 agent-codemode（运行时语义）或 infra-codemode（插件装配）。
- 不得双名/别名。

## Capabilities

- 新增 agent-codemode：catalog、声明、dispatch、runtime、限额、事件、权限。
- 更新 agent-tools：XyTool exposure/namespace/output_schema；run_code 经插件注册表注册，不进 7 工具闭集；并发类 Barrier/Sequential。
- 更新 agent-runtime：catalog 过滤；嵌套调用复用 before_tool/权限/after_tool；取消；事件；不得把嵌套结果写入模型上下文。
- 更新 domain-security：run_code 权限映射；凭据剥离；隔离声明。
- 更新 runtime-config：codemode 配置节（enabled、runtime=boa、mode=on、timeout、max_output_tokens、max_result_bytes、max_parallel_subcalls、max_nested_calls、inline_budget、limits、exposure）。
- 更新 workspace-assembly：官方插件装配列表。
- 首切片不改 protocol-app/wire/TUI；嵌套事件树后续。

## Impact

xylitol-plugin-sdk（exposure 元数据 + dispatch 端口 + ctx 字段）、xylitol-runtime（catalog + dispatch + 嵌套执行）、xylitol-kernel（插件装配 + 注入）、新官方插件、配置 schema、测试 harness。语言 runtime 的二进制/构建影响见 research。

## Further Notes

- 调研：llmanspec/delayed-changes/c3106-add-codemode-tool/research/codemode-architecture-2026-11.md
- 设计暂按 Boa 0.22；沙箱方案未定（Boa in-process vs QuickJS WASM vs Eryx），见 research/codemode-sandbox-security-2026-11.md；park 后再定。
- pi codemode：QuickJS WASM worker；tools.<name>；ALL_TOOLS/searchTools/describeTool/describeNamespace；TypeScript declarations；deferred；codemode.mode on/only；exposure modes（https://pi.dev/docs/latest/codemode）。
- 语言候选：JS（QuickJS WASM / Boa / rquickjs）；Python（Eryx / Monty）；Rust DSL/DAG 不作主语言。
- 现有基础：HostRegistries.tools / ToolSet / selection / replaces / freeze；MCP mcp__ 工具；外进程插件 {plugin}__{tool}；prepare_call/run_one；permission_router。
- 缺口：XyTool 无 exposure/namespace/output_schema；无 catalog/declaration；无 dispatch；无 runtime；无输出规范化；事件无 parent/root id。
- 与 c3060-add-mcp-tool-catalog 协同：catalog 的 MCP 部分可直接复用。

## Open Questions

- 已决：exposure 默认；mode on；声明/输出策略，见 design.md。
- 未定：sandbox/runtime（Boa in-process vs QuickJS WASM vs Eryx）；恢复时先定这一项。
- 后续（不在本 change）：mode only；image()；store/load；output_schema 结构化消费；worker 子进程 hard kill；嵌套事件树；多语言/WASM。

## Parking

本 change 暂存 delayed-changes：范围大（工具暴露/catalog/声明/dispatch/沙箱），runtime/隔离方案未定（Boa in-process vs QuickJS WASM vs Eryx）。恢复条件：确定 sandbox/runtime 与首切片范围后，经 propose/ff 迁回 changes/。
