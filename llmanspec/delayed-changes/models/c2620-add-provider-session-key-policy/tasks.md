# Tasks: c2620-add-provider-session-key-policy

测试 seam:ai-bridge attribution / WirePolicy 单测 + 现有 provider 单测。不打网;缓存行为 lab(`lab_*`)另证,不入本刀门禁。

升格回 `llmanspec/changes/` 后再 `change start` / Specs landing。c2600 字段已归档，本刀只填呈报策略。

- [ ] t1 specs:`package-ai-bridge` 新增键策略 MUST(轮廓分流;书签不进供应商键/稿首;子本不继承切点后 `previous_response_id`)
- [ ] t2 策略实现:`x-opencode-session` / `prompt_cache_key` / `previous_response_id` 取值走命名策略,不再默认绑书签
- [ ] t3 单测:fork 子本键不冷化同树干;书签 UUID 不出现在任何供应商键/模型稿;默认关的键仍默认关
- [ ] t4 相关单测 + `llman-sdd validate c2620-add-provider-session-key-policy --strict --no-check`
