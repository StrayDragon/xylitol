# Tasks — c03-update-cli-mode-dispatch

- [x] Delete `RunMode` enum and `--mode` field from `CliArgs`
- [x] Add `--acp` (feature-gated `infra-acp`) and `--list-models` fields to `CliArgs`
- [x] Rewrite `run()`: config load → `--list-models` early-exit → `--acp` → prompt auto-detect
- [x] Implement `list_models_and_exit()` with model table output
- [x] Add `__fake__` sentinel short-circuit in `build_model_config()`
- [x] Add `ModelKind::Fake` variant (feature-gated) to `src/agent/model.rs`
- [x] Add `ModelConfig::build()` arm for `Fake` constructing `FakeProvider`
- [x] Add `provider_name()` arm for `Fake`
- [x] Rewrite tests: remove `--mode`, add auto-detect + `--acp` + `--list-models` + `__fake__`
- [x] Update `llmanspec/specs/cli-entry/spec.md`
- [x] Run `just fmt && just lint && just test`
