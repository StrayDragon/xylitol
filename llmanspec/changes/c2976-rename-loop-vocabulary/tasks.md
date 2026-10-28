# Tasks

- [ ] 1. **specs**（spec）：`agent-runtime` 阶段词表与 `loop-middleware-policies-plugin`；`runtime-config` `loop_policies` 与旧键拒绝。测：`llman-sdd validate --all --strict`。
- [ ] 2. **SDK**（crate）：`LoopStage` / trait / ctx / `LoopChain` 改名。测：`cargo test -p xylitol-plugin-sdk`。
- [ ] 3. **runtime**（crate）：runner / fingerprint / ReAct 调用点改名。测：`cargo test -p xylitol-runtime`。
- [ ] 4. **plugin**（plugins）：`loop-example` → `loop-policies`；config / plugin id / 类型 / 目录。测：`cargo test -p xylitol-official-plugin-loop-policies`。
- [ ] 5. **kernel / config**（crates）：builtins / boot / bootstrap / schema / example / loader。测：`cargo test -p xylitol-kernel -p xylitol-official-plugin-config`。
- [ ] 6. **docs**（repo）：`crates/xylitol-plugin-sdk/README.md` 与 AGENTS 指针。测：`cargo doc` 经 `just qa`。
- [ ] 7. **tests**（tests）：BDD step / binding / runtime-config 改名。测：`cargo test --all-features --test bdd`。
- [ ] 8. **门禁**（repo）：`just qa` 全绿。测：`just qa`。
