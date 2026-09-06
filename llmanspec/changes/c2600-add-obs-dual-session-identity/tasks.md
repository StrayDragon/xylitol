# Tasks: c2600-add-obs-dual-session-identity

测试 seam：infra-otel CollectingReporter + ai-bridge 观测属性单测。不打网。不新扩 BDD（与 otel6 头less 并存，双 id 由单测钉）。

建议在 c2590 归档后再 `change start` / Specs landing（本文件可先留在 main 防遗忘）。

- [ ] t1 specs：infra-otel 新增双 id + 树边 MUST；otel6 仍为 langfuse.session.id=书签且 MUST 同时写 bookmark 显式键。`rules_edit_acked` 仅当确需改已锁 otel6 句面
- [ ] t2 观测快照/span 写出 bookmark_id、llm_id；fork 子本写 parent_bookmark_id 与 fork_entry_id
- [ ] t3 单测：新会话无 parent 键；fork 子本有树边；langfuse.session.id 与 bookmark_id 一致且不等于「混用的单一旧语义」
- [ ] t4 相关单测 + `llman sdd validate c2600-add-obs-dual-session-identity --strict --no-check`
