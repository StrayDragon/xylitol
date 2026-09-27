//! app-tui-host attach-resilience BDD bindings (ath41/ath43/ath44, c2480).

use crate::tests::bdd::steps_remote_resilience::{ResilienceBdd, resilience_bdd};
use crate::tests::bdd::steps_server::{ServerTest, server_test};
use rstest_bdd_macros::scenario;

#[scenario(
    path = "llmanspec/specs/app-tui-host/app-tui-host.feature",
    name = "mux-halfopen-idle-detect"
)]
async fn test_ath_halfopen_idle(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/app-tui-host/app-tui-host.feature",
    name = "attach-hello-mismatch-fatal"
)]
async fn test_ath44_hello_mismatch(resilience_bdd: ResilienceBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-host/app-tui-host.feature",
    name = "reconnect-backoff-escalation"
)]
async fn test_ath41_backoff(resilience_bdd: ResilienceBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-host/app-tui-host.feature",
    name = "reconnect-stale-generation-dropped"
)]
async fn test_ath41_stale_generation(resilience_bdd: ResilienceBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-host/app-tui-host.feature",
    name = "attach-coalesce-burst-single-projection"
)]
async fn test_ath43_coalesce(resilience_bdd: ResilienceBdd) {}

/// Real-wire seam (ath44): real serve, real HttpWsClient handshake.
#[scenario(
    path = "llmanspec/specs/app-tui-host/app-tui-host.feature",
    name = "real-kill-reconnect-journal-resume"
)]
async fn test_ath44_real_reconnect(server_test: ServerTest) {
    let _ = server_test;
}
