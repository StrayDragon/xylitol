use mermaid_text::{RenderOptions, render_with_options};

pub fn try_render_mermaid(src: &str, available_width: usize) -> Option<Vec<String>> {
    if src.trim().is_empty() {
        return None;
    }
    let opts = RenderOptions {
        max_width: Some(available_width.max(1)),
        max_width_strict: true,
        ascii: false,
        color: false,
        ..Default::default()
    };
    let art = render_with_options(src, &opts).ok()?;
    let lines: Vec<String> = art.lines().map(str::to_string).collect();
    if lines.is_empty() {
        return None;
    }
    Some(lines)
}

#[cfg(test)]
mod tests {
    use super::try_render_mermaid;

    #[test]
    fn flowchart_td_draws_boxes() {
        let src = "flowchart TD\nA --> B";
        let lines = try_render_mermaid(src, 80).expect("should draw");
        let joined = lines.join("\n");
        assert!(
            joined.contains('─') || joined.contains('│') || joined.contains('┌'),
            "expected box drawing:\n{joined}"
        );
        assert!(!joined.contains("```"));
    }

    #[test]
    fn garbage_returns_none() {
        assert!(try_render_mermaid("this is not a diagram", 80).is_none());
    }

    #[test]
    fn too_narrow_returns_none() {
        let src = "flowchart LR\nStart[Start Label] --> End[End Label]";
        assert!(try_render_mermaid(src, 8).is_none());
    }

    #[test]
    fn flowchart_lr_and_sequence_draw() {
        let lr = try_render_mermaid("flowchart LR\nA --> B", 80).expect("LR");
        assert!(lr.join("\n").contains('─') || lr.join("\n").contains('▶'));
        let seq = try_render_mermaid(
            "sequenceDiagram\nparticipant A\nparticipant B\nA->>B: hi",
            80,
        )
        .expect("sequence");
        assert!(seq.join("\n").contains('A') && seq.join("\n").contains('B'));
    }
}
