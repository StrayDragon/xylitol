# Tasks — c646-update-domain-agent-part-wire

- [x] 1. Delta 校验 + design 锁定 1+2（A1/B1/C1/D1/E1/F1/G2）
- [x] 2. `SESSION_VERSION=5`；Entry/Header/SessionEntry 判别与字段 camelCase（`parentId`/`parentSession`/…）；单测 JSON 键
- [x] 3. `AgentPart` tagged serde（`thinking`/`thinkingSignature`/Image`mimeType`）；去掉 untagged 与内嵌 ToolResult；旧形态反序列化失败
- [x] 4. `AgentMessage` toolResult → `toolCallId`；`message_text` 只聚合 text
- [x] 5. converter / persist→load；非法 content E1；扫 provider/fixture/snapshot
- [x] 6. `session_entry_to_ui*` Thinking+Assistant；travel/fork 共用；harness 幂等
- [x] 7. `just fmt` + 相关 test/clippy 绿；`llman sdd validate c646-update-domain-agent-part-wire --strict --no-interactive`
