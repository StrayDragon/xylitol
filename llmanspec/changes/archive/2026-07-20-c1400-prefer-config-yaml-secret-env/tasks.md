# Tasks: c1400-prefer-config-yaml-secret-env

- [x] 0. 实现档钉为 **C**：移除 local，无迁移
- [x] 1. loader + migrate：停止读取/复制 `config.local.*`；单测
- [x] 2. 文档 / example / secret.env.example / 缺模型文案 / 仓内注释对齐
- [x] 3. 测试夹具（含 tui e2e）改用 `config.yaml`
- [x] 4. live `runtime-config`：`rc20`/`rc21` + feature（随意图重钉）
- [x] 5. validate + finalize → **本地** merge 回 main（不 push/PR）
