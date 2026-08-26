//! app-tui-transcript BDD 绑定（P1：纯字形合约）。

use crate::steps_app_tui_transcript::{TranscriptBdd, transcript_bdd};
use rstest_bdd_macros::scenario;

#[scenario(
    path = "llmanspec/specs/app-tui-transcript/app-tui-transcript.feature",
    name = "fold-glyph-unicode-and-ascii-fallback"
)]
fn test_att19_fold_glyphs(transcript_bdd: TranscriptBdd) {}
