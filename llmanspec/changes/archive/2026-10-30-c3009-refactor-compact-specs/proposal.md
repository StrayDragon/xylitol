---
depends_on: []
title: 压缩 specs 重复与重叠规则
needs_specs_change: true
branch: sdd/c3009-refactor-compact-specs
base_branch: main
base_sha: 26f271f1948676fd01dc15d2b1a72127000c9fb7
---

# 压缩 specs 重复与重叠规则

## Why

- specs 已积累 840 条规则，同一行为在多处重复描述（CWD 校验、fork、分支摘要、内置工具闭集、token 计量、overflow、信封/方法名等），导致阅读、检索与维护成本上升。
- 重复规则会放大 review 的 `unbound`/warning 噪声，也让同一行为在两层各自漂移。

## What Changes

- 合并语义等价或同一行为面的重复规则，保留 canonical `@req` id，不重编号。
- 保留全部既有 `场景:`（并入 canonical 规则），不删除可观察行为。
- 仅做 specs 文本压缩，不改运行时代码与外部行为。

## Capabilities

- `agent-session-store`
- `agent-tools`
- `agent-runtime`
- `domain-compaction`
- `protocol-app`
- `server-core`

## Impact

- **行为不变**：所有被删规则的场景并入保留规则。
- **id 稳定**：canonical id 保留；被删 id 不再出现。
- **可回退**：恢复被删规则块即可回到压缩前形状。

## Further Notes

- `agent-runtime` 的 loop-middleware 十条经逐条核对为不同行为，不合并。
- `SessionStore 端口`（agent-session r1077 / agent-session-store r1115）与 plan 提示注入（agent-plan r2951 / agent-runtime r2955）语义互补，不合并。
