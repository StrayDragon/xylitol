# Tasks

- [x] 1. Implement spec requirements:
  - `src/infra/settings/types.rs` — Settings struct with all pi-equivalent fields (camelCase serde)
  - `src/infra/settings/storage.rs` — FileSettingsStorage + InMemorySettingsStorage with retry locking
  - `src/infra/settings/manager.rs` — SettingsManager: from_files/from_storage/in_memory, deep_merge, accessors, mutators, reload, project_trust
- [x] 2. Write 12 unit tests (types: camelCase serialization; manager: deep_merge, roundtrip, compaction defaults, tools allow/deny, reload, project trust toggle; storage: in-memory + file roundtrip)
- [x] 3. `cargo test` 367 total PASS (290 unit + 77 BDD)
- [x] 4. Run `llman sdd validate c35-align-settings-manager --strict --no-interactive`
- [x] 5. Run `just qa` ✅
