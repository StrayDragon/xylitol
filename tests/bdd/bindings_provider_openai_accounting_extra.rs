use crate::bdd::steps_provider_openai_accounting_extra::{T2EstBdd, t2_est_bdd};
use rstest_bdd_macros::scenario;

#[scenario(
    path = "plugins/xylitol-official-plugin-provider-openai/llmanspec/specs/provider-openai-accounting/provider-openai-accounting.feature",
    name = "估算优先级API首次"
)]
fn test_acc_api_first(t2_est_bdd: T2EstBdd) {}

#[scenario(
    path = "plugins/xylitol-official-plugin-provider-openai/llmanspec/specs/provider-openai-accounting/provider-openai-accounting.feature",
    name = "已中止用量不锚点"
)]
fn test_acc_aborted_anchor(t2_est_bdd: T2EstBdd) {}
