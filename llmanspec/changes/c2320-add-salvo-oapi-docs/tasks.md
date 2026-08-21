# Tasks

Host 监听器新增只读 `GET /openapi.json` 调试文档 + `GET /docs` Scalar UI；零生产行为变更。

## 1. Branch binding + Specs landing

- [x] 1.1 `change start` 绑定 `sdd/c2320-add-salvo-oapi-docs`
- [x] 1.2 live specs：`server-core` 新增 sr-oapi1（Scalar 唯一调试 UI 条款随 apply 同句）；`server-runtime.feature` 增可执行场景 `@req:sr-oapi1`
- [x] 1.3 commit Specs landing；结构过闸 + `readyToImplement=true`

## 2. 实现 + BDD 接线

- [x] 2.1 新增 `app/server/oapi.rs`（serde_json 手构 OpenAPI 3.1，方法条目从 UNARY_METHODS 生成，信封级 components，info 说明指向 specta bindings）+ 3 个单测
- [x] 2.2 `http.rs::router` 挂载 `GET /openapi.json`
- [x] 2.3 BDD 接线：bindings_server.rs 注册场景 + steps_server.rs 步骤定义（含 `.feature` 改动需 touch 触发宏重编译的坑）
- [x] 2.4 salvo 0.94 → 0.95.2（最新）+ salvo-oapi scalar-only 依赖；`GET /docs` Scalar 路由（唯一调试 UI）

## 3. 验收门禁

- [ ] 3.1 定向测试绿：oapi 单测 + server http 测试 + bdd server 场景
- [ ] 3.2 全量 validate（含 BDD 编译 `--check`）绿
- [ ] 3.3 tmux 真机冒烟：serve 起 → curl `/openapi.json` 与 `/docs` 断言 → 端口释放
- [ ] 3.4 架构文档落点补一句：`库与多客户端.md` 或 `远程体验与线协议.md` 提及调试文档端点
