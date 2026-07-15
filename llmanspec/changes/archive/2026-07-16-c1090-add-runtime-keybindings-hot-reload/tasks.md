# Tasks — c1090-add-runtime-keybindings-hot-reload

- [x] 1. Delta + design + tasks；`LLMANSPEC_BASE_REF=main llman sdd validate c1090-add-runtime-keybindings-hot-reload --no-interactive`
- [x] 2. 包：`KeybindingsConfig` 改为拥有型 String；单测 override / unknown id
- [x] 3. 产品：`keybindings` 模块 — APP 目录 + 启动 merge + `load`/`reload_keybindings`
- [x] 4. 产品：slot_input / input_policy / session_resume / host 改 id 匹配
- [x] 5. harness：默认 id 行为 + reload 成功/坏文件
- [x] 6. PI_DELTAS（若有 `app.tools.blocks` 等产品特有 id）
- [x] 7. `just fmt` + 相关测试；validate --strict
