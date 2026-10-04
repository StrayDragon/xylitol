use crate::tests::bdd::fixtures::*;
use crate::tests::bdd::steps_runtime_config::{RcSnap, rc_snap};
use crate::tests::bdd::steps_tokenizer::{TokenizerBdd, tokenizer_bdd};
use rstest_bdd_macros::scenario;

#[scenario(
    path = "llmanspec/specs/runtime-config/runtime-config.feature",
    name = "tokenizer-hf-ok"
)]
fn test_rc18_named(tokenizer_bdd: TokenizerBdd) {}

#[scenario(
    path = "llmanspec/specs/runtime-config/runtime-config.feature",
    name = "tokenizer-inline-repo"
)]
fn test_rc18_inline(tokenizer_bdd: TokenizerBdd) {}

#[scenario(
    path = "llmanspec/specs/runtime-config/runtime-config.feature",
    name = "tokenizer-unknown-name-fails"
)]
fn test_rc18_unknown(tokenizer_bdd: TokenizerBdd) {}

#[scenario(
    path = "llmanspec/specs/runtime-config/runtime-config.feature",
    name = "local-tokenizer-default-off"
)]
fn test_rc19_default(tokenizer_bdd: TokenizerBdd) {}

#[scenario(
    path = "llmanspec/specs/runtime-config/runtime-config.feature",
    name = "local-tokenizer-on"
)]
fn test_rc19_on(tokenizer_bdd: TokenizerBdd) {}

#[scenario(
    path = "llmanspec/specs/runtime-config/runtime-config.feature",
    name = "local-tokenizer-invalid-fails"
)]
fn test_rc19_invalid(tokenizer_bdd: TokenizerBdd) {}

#[scenario(
    path = "llmanspec/specs/runtime-config/runtime-config.feature",
    name = "mode-set"
)]
fn test_rc_mode_set(rc_snap: RcSnap) {}
#[scenario(
    path = "llmanspec/specs/runtime-config/runtime-config.feature",
    name = "mode-default-one-at-a-time"
)]
fn test_rc_mode_default(rc_snap: RcSnap) {}
#[scenario(
    path = "llmanspec/specs/runtime-config/runtime-config.feature",
    name = "settings-delivered-surface-only"
)]
fn test_rc_settings_delivered_surface(rc_snap: RcSnap) {}
#[scenario(
    path = "llmanspec/specs/runtime-config/runtime-config.feature",
    name = "no-prompts-settings-field"
)]
fn test_rc_no_prompts_settings_field(rc_snap: RcSnap) {}
#[scenario(
    path = "llmanspec/specs/runtime-config/runtime-config.feature",
    name = "mapping-documented"
)]
fn test_rc_mapping(rc_snap: RcSnap) {}
#[scenario(
    path = "llmanspec/specs/runtime-config/runtime-config.feature",
    name = "field"
)]
fn test_rc_field(rc_snap: RcSnap) {}
#[scenario(
    path = "llmanspec/specs/runtime-config/runtime-config.feature",
    name = "default-setting"
)]
fn test_rc_default_setting(rc_snap: RcSnap) {}
#[scenario(
    path = "llmanspec/specs/runtime-config/runtime-config.feature",
    name = "select-ignores-settings-default"
)]
fn test_rc_select_ignores(rc_snap: RcSnap) {}
#[scenario(
    path = "llmanspec/specs/runtime-config/runtime-config.feature",
    name = "config-yaml-secret-env-layout"
)]
fn test_rc_config_docs(rc_snap: RcSnap) {}

#[scenario(
    path = "llmanspec/specs/runtime-config/runtime-config.feature",
    name = "parse-list"
)]
fn test_rc_parse_list(tokenizer_bdd: TokenizerBdd, rc_snap: RcSnap) {}
#[scenario(
    path = "llmanspec/specs/runtime-config/runtime-config.feature",
    name = "empty-token-fails"
)]
fn test_rc_empty_token_fails(tokenizer_bdd: TokenizerBdd) {}
#[scenario(
    path = "llmanspec/specs/runtime-config/runtime-config.feature",
    name = "freeform-ok"
)]
fn test_rc_freeform_ok(tokenizer_bdd: TokenizerBdd, rc_snap: RcSnap) {}
#[scenario(
    path = "llmanspec/specs/runtime-config/runtime-config.feature",
    name = "parse-map"
)]
fn test_rc_parse_map(tokenizer_bdd: TokenizerBdd, rc_snap: RcSnap) {}
#[scenario(
    path = "llmanspec/specs/runtime-config/runtime-config.feature",
    name = "map-key-outside-list-fails"
)]
fn test_rc_map_key_outside_list_fails(tokenizer_bdd: TokenizerBdd) {}
#[scenario(
    path = "llmanspec/specs/runtime-config/runtime-config.feature",
    name = "absent-key-ok"
)]
fn test_rc_absent_key_ok(tokenizer_bdd: TokenizerBdd, rc_snap: RcSnap) {}
#[scenario(
    path = "llmanspec/specs/runtime-config/runtime-config.feature",
    name = "config-local-not-merged"
)]
fn test_rc_local_not_merged(tokenizer_bdd: TokenizerBdd, rc_snap: RcSnap) {}

#[scenario(
    path = "llmanspec/specs/runtime-config/runtime-config.feature",
    name = "compaction-model-anchor-alias"
)]
fn test_rc_compaction_anchor(rc_snap: RcSnap, tokenizer_bdd: TokenizerBdd) {}

#[scenario(
    path = "llmanspec/specs/runtime-config/runtime-config.feature",
    name = "compaction-model-settings-ignored"
)]
fn test_rc_compaction_settings_ignored(rc_snap: RcSnap) {}

// ── c2835 后继：runtime-config 裸规则回填 ──────────────────────────

#[scenario(
    path = "llmanspec/specs/runtime-config/runtime-config.feature",
    name = "config-value-resolver-at-infra-edge"
)]
fn test_rc_local_ignored(rc_snap: RcSnap, tokenizer_bdd: TokenizerBdd) {}

#[scenario(
    path = "llmanspec/specs/runtime-config/runtime-config.feature",
    name = "json-schema-at-config-edge"
)]
fn test_rc_jsonschema_edge() {}

#[scenario(
    path = "llmanspec/specs/runtime-config/runtime-config.feature",
    name = "no-max-iterations-field"
)]
fn test_rc_no_max_iterations(rc_snap: RcSnap) {}

#[scenario(
    path = "llmanspec/specs/runtime-config/runtime-config.feature",
    name = "template-vars-home-only"
)]
fn test_rc_template_vars() {}

#[scenario(
    path = "llmanspec/specs/runtime-config/runtime-config.feature",
    name = "otel-section-optional"
)]
fn test_rc_otel_section() {}

#[scenario(
    path = "llmanspec/specs/runtime-config/runtime-config.feature",
    name = "editor-history-seed-default-one"
)]
fn test_rc_editor_seed_default() {}

#[scenario(
    path = "llmanspec/specs/runtime-config/runtime-config.feature",
    name = "tool-batch-mode-default-and-invalid"
)]
fn test_rc_tool_batch_mode() {}

#[scenario(
    path = "llmanspec/specs/runtime-config/runtime-config.feature",
    name = "session-max-turns-loaded"
)]
fn test_rc_session_max_turns(agent: AgentState, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/runtime-config/runtime-config.feature",
    name = "activity-fold-lexical-contract"
)]
fn test_rc_activity_fold_lexical() {}
