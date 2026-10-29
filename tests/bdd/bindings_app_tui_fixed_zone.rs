use crate::bdd::steps_app_tui_fixed_zone::{FixedZoneBdd, fixed_zone_bdd};
use crate::bdd::steps_app_tui_transcript::{TranscriptBdd, transcript_bdd};
use rstest_bdd_macros::scenario;

#[scenario(
    path = "llmanspec/specs/app-tui-fixed-zone/app-tui-fixed-zone.feature",
    name = "待办栏位于介于队列与提示条"
)]
fn test_todo_bar_sits_between_queue_and_toast(fixed_zone_bdd: FixedZoneBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-host/app-tui-host.feature",
    name = "资源头已刷新来自快照"
)]
fn test_ath38_header_from_downlink(fixed_zone_bdd: FixedZoneBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-fixed-zone/app-tui-fixed-zone.feature",
    name = "固定区槽顺序"
)]
fn test_atz_slot_order(fixed_zone_bdd: FixedZoneBdd) {}
#[scenario(
    path = "llmanspec/specs/app-tui-fixed-zone/app-tui-fixed-zone.feature",
    name = "页脚携带无队列徽标"
)]
fn test_atz_footer_no_badge(fixed_zone_bdd: FixedZoneBdd) {}
#[scenario(
    path = "llmanspec/specs/app-tui-fixed-zone/app-tui-fixed-zone.feature",
    name = "字形设置按配置不字体探针"
)]
fn test_atz_glyph_config(transcript_bdd: TranscriptBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-fixed-zone/app-tui-fixed-zone.feature",
    name = "首屏只加载最新一页"
)]
fn test_fz_first_page(fixed_zone_bdd: FixedZoneBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-fixed-zone/app-tui-fixed-zone.feature",
    name = "滚顶加载更早"
)]
fn test_fz_load_earlier(fixed_zone_bdd: FixedZoneBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-fixed-zone/app-tui-fixed-zone.feature",
    name = "窗口溢出后提示为收起而非加载更早"
)]
fn test_fz_collapsed_notice(fixed_zone_bdd: FixedZoneBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-fixed-zone/app-tui-fixed-zone.feature",
    name = "缓冲超限后回退为加载更早"
)]
fn test_fz_buffer_capped() {}

#[scenario(
    path = "llmanspec/specs/app-tui-fixed-zone/app-tui-fixed-zone.feature",
    name = "短会话无分页提示"
)]
fn test_fz_short_no_notice(fixed_zone_bdd: FixedZoneBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-fixed-zone/app-tui-fixed-zone.feature",
    name = "默认暗色次要文本对比度达标"
)]
fn test_c2999_contrast(fixed_zone_bdd: FixedZoneBdd) {}
