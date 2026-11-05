# Codemode 深挖清单（2026-11）

> 性质：change 附属调研，不是 live spec。
> 状态：c3106 parked；以下为恢复前需要定的问题。

## 阻塞项

1. sandbox/runtime：Boa in-process vs QuickJS WASM vs Eryx；需 spike（jsdet-core / quickjs-wasm-rs）。
2. catalog/exposure：Direct/ModelOnly/Codemode/Deferred/Hidden 精确语义；默认分配；config 覆盖；namespace；identifier 冲突。
3. dispatch：嵌套并发/Barrier；取消；递归；事件抑制；权限映射；max calls；FileMutationQueue。
4. declaration：TypeScript 声明生成；inline budget；复杂 MCP schema；deferred 发现；token 估算。
5. output：AgentPart image；MCP structuredContent；bash 结构化；结果上限/full output 文件。
6. plan-mode preview writes：独立 draft c3196；与 codemode catalog 交互。
7. 协调：c3060 MCP tool catalog；c1960 tool search；避免重复。
8. 安全：run_code permission（check_process）；凭据剥离；trust；prompt injection。
9. 测试/eval：fake runtime；BDD；真实模型成功率；性能/启动成本。
10. mode only/UI：后续。

## 建议顺序

1. sandbox spike（决定 runtime）。
2. catalog/exposure + declaration 原型（不依赖 runtime）。
3. dispatch 语义（复用 prepare_call/run_one）。
4. 输出/权限/测试。
5. plan preview draft 独立推进。

## 来源

- research/codemode-architecture-2026-11.md
- research/codemode-sandbox-security-2026-11.md
