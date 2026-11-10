use crate::bdd::steps_package_tui_engine::{
    EngineOfferBdd, EngineOverlayBdd, EnginePanelBdd, EngineViewportBdd, engine_offer_bdd,
    engine_overlay_bdd, engine_panel_bdd, engine_viewport_bdd,
};
use rstest_bdd_macros::scenario;

#[scenario(
    path = "crates/xylitol-tui/llmanspec/specs/package-tui-engine/package-tui-engine.feature",
    name = "分发提供指针块焦点"
)]
fn test_dispatch_offer_pointer_blocks_focus(engine_offer_bdd: EngineOfferBdd) {
    let _ = engine_offer_bdd;
}

#[scenario(
    path = "crates/xylitol-tui/llmanspec/specs/package-tui-engine/package-tui-engine.feature",
    name = "覆盖层视口钉住之后再投影"
)]
fn test_overlay_viewport_pins_after_reproject(engine_viewport_bdd: EngineViewportBdd) {
    let _ = engine_viewport_bdd;
}

#[scenario(
    path = "crates/xylitol-tui/llmanspec/specs/package-tui-engine/package-tui-engine.feature",
    name = "宽度与高度 clamp"
)]
fn test_panel_region_clamps_bounds(engine_panel_bdd: EnginePanelBdd) {
    let _ = engine_panel_bdd;
}

#[scenario(
    path = "crates/xylitol-tui/llmanspec/specs/package-tui-engine/package-tui-engine.feature",
    name = "overlay clip 求交"
)]
fn test_overlay_clip_intersects(engine_overlay_bdd: EngineOverlayBdd) {
    let _ = engine_overlay_bdd;
}
