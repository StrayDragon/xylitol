//! Shared AppConfig parse fixture used by runtime-config BDD
//! (`TokenizerBdd` name kept so existing bindings stay stable).

use crate::bdd::prelude::*;
use rstest::fixture;
use rstest_bdd_macros::{then, when};

pub struct TokenizerBdd {
    pub(crate) config: RefCell<xylitol::infra::config::types::AppConfig>,
    pub(crate) cfg_ok: Cell<bool>,
    pub(crate) cfg_err: RefCell<String>,
}

impl TokenizerBdd {
    fn new() -> Self {
        Self {
            config: RefCell::new(xylitol::infra::config::types::AppConfig::default()),
            cfg_ok: Cell::new(false),
            cfg_err: RefCell::new(String::new()),
        }
    }
}

#[fixture]
pub fn tokenizer_bdd() -> TokenizerBdd {
    TokenizerBdd::new()
}

pub fn parse_app_config_yaml(
    yaml: &str,
) -> Result<xylitol::infra::config::types::AppConfig, String> {
    let cfg: xylitol::infra::config::types::AppConfig =
        yaml_serde::from_str(yaml).map_err(|e| e.to_string())?;
    cfg.validate_thinking_levels().map_err(|e| e.to_string())?;
    cfg.validate_session_max_turns()
        .map_err(|e| e.to_string())?;
    Ok(cfg)
}

/// Given already attempted parse into `tokenizer_bdd`; this When is a no-op seam.
#[when("加载配置")]
fn w_rc_load(_tokenizer_bdd: &TokenizerBdd) {}

#[then("失败")]
fn t_rc_fail(tokenizer_bdd: &TokenizerBdd) {
    assert!(
        !tokenizer_bdd.cfg_ok.get(),
        "expected load failure, got: {}",
        tokenizer_bdd.cfg_err.borrow()
    );
}
