# Future — c315 衍生

## spec-cleanup: security-policy 的死 SecurityEngine 需求

`security-policy` 仍保留若干描述**从未实现**的 `SecurityEngine` 的需求：

- r1 `security-wrapper` — `SecurityToolWrapper` 从未实现（c265 design §5 已确认零匹配）。
- r2 `tighten-only` — 三层配置"只收紧不放宽"，依赖未实现的 SecurityEngine。
- r3 `unified-path-field-check` — SecurityEngine 检查 `file_path`/`path`，未实现。
- r4 `mcp-tool-security-policy` — SecurityEngine 的 MCP 分支，未实现。
- r6 `hook-timeout-kill` — 与 `hook-system` spec 重复，且执行器语义在 `infra/hooks`。
- r7 `network-domain-enforcement` — 措辞指向 SecurityEngine，实际由 `XyPermission`
  的 `check_network`（s12）承担。

这些与 s13/s15（permission 是咨询性、非安全边界）的立场有概念张力：要么实现一个
真正的 SecurityEngine（与"不做 sandbox"立场相悖），要么承认这些需求描述了不会建造
的东西并从 spec 移除。建议在 xylitol 决定是否引入 tool-routing 沙箱模式时一并裁决。

触发信号：决定 tool 执行是否需要进程外/容器隔离时。
