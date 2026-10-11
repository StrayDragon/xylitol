//! BDD bindings for `package-tui-engine` r1946 (c2880).

use crate::bdd::steps_package_tui_engine::{
    EngineOfferBdd, EngineViewportBdd, engine_offer_bdd, engine_viewport_bdd,
};
use rstest_bdd_macros::scenario;

#[scenario(
    path = "packages/xylitol-tui/llmanspec/specs/package-tui-engine/package-tui-engine.feature",
    name = "dispatch-offer-pointer-blocks-focus"
)]
fn test_dispatch_offer_pointer_blocks_focus(engine_offer_bdd: EngineOfferBdd) {
    let _ = engine_offer_bdd;
}

#[scenario(
    path = "packages/xylitol-tui/llmanspec/specs/package-tui-engine/package-tui-engine.feature",
    name = "overlay-viewport-pins-after-reproject"
)]
fn test_overlay_viewport_pins_after_reproject(engine_viewport_bdd: EngineViewportBdd) {
    let _ = engine_viewport_bdd;
}
