//! BDD bindings for `package-tui-markdown` (c2865).

use crate::bdd::steps_package_tui_markdown::{MarkdownFenceBdd, markdown_fence_bdd};
use rstest_bdd_macros::scenario;

#[scenario(
    path = "packages/xylitol-tui/llmanspec/specs/package-tui-markdown/package-tui-markdown.feature",
    name = "streaming-partial-fence-height-stable"
)]
fn test_streaming_partial_fence_height_stable(markdown_fence_bdd: MarkdownFenceBdd) {
    let _ = markdown_fence_bdd;
}
