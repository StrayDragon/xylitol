# Tasks — Agent profile resume diff

- [x] 1. Header metadata：agent_profile / fingerprint 可选字段；store get/set；单测。
- [x] 2. Kernel fingerprint：SHA256(system_prompt + model + tools)，传给 driver / factory。
- [x] 3. Driver：new_session 写入 profile；switch_session 比较并记录 mismatch notice。
- [x] 4. 测试：header 序列化、store round-trip。
- [x] 5. 门禁：just fmt / just lint / just qa / llman-sdd validate c3100-add-agent-preset-resume-diff --strict。
