use crate::bdd::steps_remote_resilience::{ResilienceBdd, resilience_bdd};
use crate::bdd::steps_server::{ServerTest, server_test};
use rstest_bdd_macros::scenario;

#[scenario(
    path = "llmanspec/specs/app-tui-host/app-tui-host.feature",
    name = "复用半开空闲检测"
)]
async fn test_ath_halfopen_idle(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/app-tui-host/app-tui-host.feature",
    name = "attachhello不匹配致命"
)]
async fn test_ath44_hello_mismatch(resilience_bdd: ResilienceBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-host/app-tui-host.feature",
    name = "重连退避升级"
)]
async fn test_ath41_backoff(resilience_bdd: ResilienceBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-host/app-tui-host.feature",
    name = "重连过期生成已丢弃"
)]
async fn test_ath41_stale_generation(resilience_bdd: ResilienceBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-host/app-tui-host.feature",
    name = "attach合并突发单一投影"
)]
async fn test_ath43_coalesce(resilience_bdd: ResilienceBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-host/app-tui-host.feature",
    name = "真实杀死重连日志续跑"
)]
async fn test_ath44_real_reconnect(server_test: ServerTest) {
    let _ = server_test;
}

#[scenario(
    path = "llmanspec/specs/app-tui-host/app-tui-host.feature",
    name = "attach排空不已阻塞按待处理一元"
)]
async fn test_ath38_attach_drain_nonblocking(resilience_bdd: ResilienceBdd) {}
