use crate::steps_cli_tokenizer::{TokenizerBdd, tokenizer_bdd};
use crate::steps_runtime_config::{RcSnap, rc_snap};
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
    name = "transport"
)]
fn test_rc_transport(rc_snap: RcSnap) {}
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
    name = "shell-path"
)]
fn test_rc_shell_path(rc_snap: RcSnap) {}
#[scenario(
    path = "llmanspec/specs/runtime-config/runtime-config.feature",
    name = "trust-default"
)]
fn test_rc_trust_default(rc_snap: RcSnap) {}
#[scenario(
    path = "llmanspec/specs/runtime-config/runtime-config.feature",
    name = "themes-list"
)]
fn test_rc_themes_list(rc_snap: RcSnap) {}
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
#[serial_test::serial(env_global)]

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
#[serial_test::serial(env_global)]

fn test_rc_local_not_merged(tokenizer_bdd: TokenizerBdd, rc_snap: RcSnap) {}
