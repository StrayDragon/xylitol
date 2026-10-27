# Tasks: c2946

## 1. sdk 注册表派生

- [ ] 1.1 `LoopMiddlewareRegistry::derive()`：冻结父链有序快照，重建 entry 列表（`order` + 续 `seq`），子可继续注册/注销。
- [ ] 1.2 单测：父快照冻结（父后续注册不流入子）、子本地注册不影响父、子注销不影响父。

## 2. runtime 作用域标记

- [ ] 2.1 `AgentRuntime` 增作用域标记（`Root` / `Session`），`set_loop_middleware` 保持默认 `Root`，新增带作用域的重载。
- [ ] 2.2 middleware span 属性增加 `loop.middleware.scope`（值 `root` / `session`）。

## 3. kernel / serve 槽接线

- [ ] 3.1 `SessionSlot` 持有 `Option<Arc<LoopMiddlewareRegistry>>`，`SessionSlot::new` 从 HostState 的 root registry 派生。
- [ ] 3.2 `materialize_writer_at` 把槽内 registry 交给 driver（带 session 作用域标记）。
- [ ] 3.3 kernel 契约测试：两槽 registry 互不影响；root 为空时槽内链为空。

## 4. embed 入口

- [ ] 4.1 `xylitol::embed::with_loop_registry(driver, registry)`，不泄漏 rutis 类型。

## 5. BDD

- [ ] 5.1 `agent-runtime` 新增会话作用域规则 + 场景（会话隔离、root 默认、释放不泄漏）。
- [ ] 5.2 实现 BDD 步骤并绑定。

## 6. 文档与验证

- [ ] 6.1 `src/AGENTS.md`「Loop 中间件」段补作用域说明。
- [ ] 6.2 `cargo test --all-features --test bdd` 绿。
- [ ] 6.3 `llman-sdd validate --all --strict` 绿。
- [ ] 6.4 `just qa` 绿。
