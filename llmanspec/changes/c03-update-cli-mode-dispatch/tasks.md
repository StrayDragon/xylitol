# Tasks — c03-update-cli-mode-dispatch

- [ ] Delete `RunMode` enum and `--mode` field from `CliArgs`
- [ ] Add `--acp` (feature-gated `infra-acp`) and `--list-models` fields to `CliArgs`
- [ ] Rewrite `run()`: config load → `--list-models` early-exit → `--acp` → prompt auto-detect
- [ ] Implement `list_models_and_exit()` with model table output
- [ ] Add `__fake__` sentinel short-circuit in `build_model_config()`
- [ ] Add `ModelKind::Fake` variant (feature-gated) to `src/agent/model.rs`
- [ ] Add `ModelConfig::build()` arm for `Fake` constructing `FakeProvider`
- [ ] Add `provider_name()` arm for `Fake`
- [ ] Rewrite tests: remove `--mode`, add auto-detect + `--acp` + `--list-models` + `__fake__`
- [ ] Update `llmanspec/specs/cli-entry/spec.md`
- [ ] Run `just fmt && just lint && just test`
