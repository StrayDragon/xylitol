//! BDD bindings for `package-tui-markdown` (c2865 / c2890).

use crate::bdd::steps_package_tui_markdown::{
    MarkdownFenceBdd, MarkdownMermaidBdd, markdown_fence_bdd, markdown_mermaid_bdd,
};
use rstest_bdd_macros::scenario;

#[scenario(
    path = "packages/xylitol-tui/llmanspec/specs/package-tui-markdown/package-tui-markdown.feature",
    name = "streaming-partial-fence-height-stable"
)]
fn test_streaming_partial_fence_height_stable(markdown_fence_bdd: MarkdownFenceBdd) {
    let _ = markdown_fence_bdd;
}

#[scenario(
    path = "packages/xylitol-tui/llmanspec/specs/package-tui-markdown/package-tui-markdown.feature",
    name = "mermaid-closed-flowchart-draws-boxes"
)]
fn test_mermaid_closed_flowchart_draws_boxes(markdown_mermaid_bdd: MarkdownMermaidBdd) {
    let _ = markdown_mermaid_bdd;
}

#[scenario(
    path = "packages/xylitol-tui/llmanspec/specs/package-tui-markdown/package-tui-markdown.feature",
    name = "mermaid-unknown-type-stays-source"
)]
fn test_mermaid_unknown_type_stays_source(markdown_mermaid_bdd: MarkdownMermaidBdd) {
    let _ = markdown_mermaid_bdd;
}

#[scenario(
    path = "packages/xylitol-tui/llmanspec/specs/package-tui-markdown/package-tui-markdown.feature",
    name = "mermaid-final-open-fence-stays-source"
)]
fn test_mermaid_final_open_fence_stays_source(markdown_mermaid_bdd: MarkdownMermaidBdd) {
    let _ = markdown_mermaid_bdd;
}

#[scenario(
    path = "packages/xylitol-tui/llmanspec/specs/package-tui-markdown/package-tui-markdown.feature",
    name = "mermaid-off-never-draws"
)]
fn test_mermaid_off_never_draws(markdown_mermaid_bdd: MarkdownMermaidBdd) {
    let _ = markdown_mermaid_bdd;
}

#[scenario(
    path = "packages/xylitol-tui/llmanspec/specs/package-tui-markdown/package-tui-markdown.feature",
    name = "mermaid-too-wide-falls-back"
)]
fn test_mermaid_too_wide_falls_back(markdown_mermaid_bdd: MarkdownMermaidBdd) {
    let _ = markdown_mermaid_bdd;
}

#[scenario(
    path = "packages/xylitol-tui/llmanspec/specs/package-tui-markdown/package-tui-markdown.feature",
    name = "mermaid-streaming-open-garbage-falls-back"
)]
fn test_mermaid_streaming_open_garbage_falls_back(markdown_mermaid_bdd: MarkdownMermaidBdd) {
    let _ = markdown_mermaid_bdd;
}
