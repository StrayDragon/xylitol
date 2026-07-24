//! Tool-batch classify + window planning (c1545).
//!
//! Pure data helpers — no I/O. ReAct BarrierParallel wiring consumes these next.
#![allow(dead_code)] // used by unit tests now; ReAct flush path lands in follow-up

use crate::protocol::ports::{XyTool, XyToolExecutionMode};

/// A planned execution window for [`crate::protocol::ports::XyBatchMode::BarrierParallel`].
///
/// Indices refer to the source-order tool-call list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PlannedWindow {
    /// Consecutive ParallelSafe tools — flush/fan-out together.
    Parallel(Vec<usize>),
    /// Single Barrier tool — run alone after any open parallel window is flushed.
    Barrier(usize),
}

/// Classify a tool call for barrier-parallel scheduling.
///
/// Hard rule: names starting with `mcp:` are **always** Barrier
/// ([`XyToolExecutionMode::Sequential`]), even if the trait claims ParallelSafe.
/// Missing / unknown tools are Barrier.
pub(crate) fn classify(name: &str, tool: Option<&dyn XyTool>) -> XyToolExecutionMode {
    if name.starts_with("mcp:") {
        return XyToolExecutionMode::Sequential;
    }
    match tool {
        Some(t) => t.execution_mode(),
        None => XyToolExecutionMode::Sequential,
    }
}

/// Cut source-ordered concurrency classes into parallel windows and barrier singles.
///
/// Does not consult batch mode — callers only invoke this for BarrierParallel.
pub(crate) fn plan_windows(classes: &[XyToolExecutionMode]) -> Vec<PlannedWindow> {
    let mut windows = Vec::new();
    let mut parallel_buf: Vec<usize> = Vec::new();

    for (i, class) in classes.iter().enumerate() {
        match class {
            XyToolExecutionMode::Parallel => parallel_buf.push(i),
            XyToolExecutionMode::Sequential => {
                if !parallel_buf.is_empty() {
                    windows.push(PlannedWindow::Parallel(std::mem::take(&mut parallel_buf)));
                }
                windows.push(PlannedWindow::Barrier(i));
            }
        }
    }

    if !parallel_buf.is_empty() {
        windows.push(PlannedWindow::Parallel(parallel_buf));
    }

    windows
}

/// Classify each `(name, tool)` then [`plan_windows`].
pub(crate) fn plan_windows_for_calls<'a, I>(calls: I) -> Vec<PlannedWindow>
where
    I: IntoIterator<Item = (&'a str, Option<&'a dyn XyTool>)>,
{
    let classes: Vec<XyToolExecutionMode> = calls
        .into_iter()
        .map(|(name, tool)| classify(name, tool))
        .collect();
    plan_windows(&classes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use serde_json::{Value, json};

    use crate::protocol::error::XyToolError;
    use crate::protocol::ports::XyToolCtx;

    struct ModeTool {
        name: &'static str,
        mode: XyToolExecutionMode,
    }

    #[async_trait]
    impl XyTool for ModeTool {
        fn name(&self) -> &str {
            self.name
        }

        fn description(&self) -> &str {
            "test"
        }

        fn parameters_schema(&self) -> Value {
            json!({})
        }

        async fn execute(&self, _ctx: &XyToolCtx, _args: Value) -> Result<String, XyToolError> {
            Ok(String::new())
        }

        fn execution_mode(&self) -> XyToolExecutionMode {
            self.mode
        }
    }

    fn safe(name: &'static str) -> ModeTool {
        ModeTool {
            name,
            mode: XyToolExecutionMode::Parallel,
        }
    }

    fn barrier(name: &'static str) -> ModeTool {
        ModeTool {
            name,
            mode: XyToolExecutionMode::Sequential,
        }
    }

    #[test]
    fn classify_mcp_prefix_hard_barrier_even_if_trait_lies() {
        let lying = ModeTool {
            name: "mcp:fake:x",
            mode: XyToolExecutionMode::Parallel,
        };
        assert_eq!(
            classify("mcp:fake:x", Some(&lying)),
            XyToolExecutionMode::Sequential
        );
    }

    #[test]
    fn classify_unknown_is_barrier() {
        assert_eq!(classify("nope", None), XyToolExecutionMode::Sequential);
    }

    #[test]
    fn classify_respects_trait_for_non_mcp() {
        let read = safe("read");
        let write = barrier("write");
        assert_eq!(classify("read", Some(&read)), XyToolExecutionMode::Parallel);
        assert_eq!(
            classify("write", Some(&write)),
            XyToolExecutionMode::Sequential
        );
    }

    #[test]
    fn plan_read_write_read_windows() {
        let read = safe("read");
        let write = barrier("write");
        let windows = plan_windows_for_calls([
            ("read", Some(&read as &dyn XyTool)),
            ("write", Some(&write)),
            ("read", Some(&read)),
        ]);
        assert_eq!(
            windows,
            vec![
                PlannedWindow::Parallel(vec![0]),
                PlannedWindow::Barrier(1),
                PlannedWindow::Parallel(vec![2]),
            ]
        );
    }

    #[test]
    fn plan_all_safe() {
        let windows = plan_windows(&[
            XyToolExecutionMode::Parallel,
            XyToolExecutionMode::Parallel,
            XyToolExecutionMode::Parallel,
        ]);
        assert_eq!(windows, vec![PlannedWindow::Parallel(vec![0, 1, 2])]);
    }

    #[test]
    fn plan_all_barrier() {
        let windows = plan_windows(&[
            XyToolExecutionMode::Sequential,
            XyToolExecutionMode::Sequential,
        ]);
        assert_eq!(
            windows,
            vec![PlannedWindow::Barrier(0), PlannedWindow::Barrier(1)]
        );
    }

    #[test]
    fn plan_mcp_in_middle_forced_barrier() {
        let read = safe("read");
        let lying_mcp = ModeTool {
            name: "mcp:srv:t",
            mode: XyToolExecutionMode::Parallel,
        };
        let windows = plan_windows_for_calls([
            ("read", Some(&read as &dyn XyTool)),
            ("mcp:srv:t", Some(&lying_mcp)),
            ("read", Some(&read)),
        ]);
        assert_eq!(
            windows,
            vec![
                PlannedWindow::Parallel(vec![0]),
                PlannedWindow::Barrier(1),
                PlannedWindow::Parallel(vec![2]),
            ]
        );
    }
}
