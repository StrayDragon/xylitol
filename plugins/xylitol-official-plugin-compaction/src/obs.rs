use fastrace::prelude::*;
use xylitol_plugin_sdk::langfuse_observation_properties_from;
use xylitol_plugin_sdk::provider_trace_active;

pub(crate) fn compaction_reason_kind(reason: &str) -> &'static str {
    if reason.starts_with("overflow") {
        "overflow"
    } else if reason.starts_with("threshold") {
        "threshold"
    } else if reason.starts_with("loop_middleware") {
        "loop_middleware"
    } else {
        "manual"
    }
}

pub(crate) fn export_skipped(
    reason_detail: &str,
    parent: Option<SpanContext>,
    obs: &xylitol_plugin_sdk::ObsSessionContext,
) {
    if !provider_trace_active() {
        return;
    }
    let obs = obs.clone();
    let detail = reason_detail.to_string();
    let span = Span::root(
        "agent.compaction.skipped",
        parent.unwrap_or_else(SpanContext::random),
    )
    .with_properties(move || {
        let mut props = vec![("skip_reason".to_string(), detail)];
        props.extend(xylitol_plugin_sdk::langfuse_session_properties_from(&obs));
        props
    });
    span.add_event(Event::new("lifecycle").with_properties(|| {
        [
            ("kind", "lifecycle".to_string()),
            ("phase", "start".to_string()),
            ("name", "agent.compaction.skipped".to_string()),
        ]
    }));
    drop(span);
}

pub(crate) struct AgentCompactionSpan {
    span: Span,
}

impl AgentCompactionSpan {
    pub(crate) fn start(
        reason: &str,
        parent: Option<SpanContext>,
        obs: &xylitol_plugin_sdk::ObsSessionContext,
    ) -> Option<Self> {
        if !provider_trace_active() {
            return None;
        }
        let kind = compaction_reason_kind(reason);
        let parent = parent.unwrap_or_else(SpanContext::random);
        let obs = obs.clone();
        let span = Span::root("agent.compaction", parent).with_properties(move || {
            let mut props = vec![("reason".to_string(), kind.to_string())];
            if reason != kind {
                props.push(("reason.detail".to_string(), reason.to_string()));
            }
            props.extend(langfuse_observation_properties_from("span", &obs));
            props
        });
        span.add_event(Event::new("lifecycle").with_properties(|| {
            [
                ("kind", "lifecycle".to_string()),
                ("phase", "start".to_string()),
                ("name", "agent.compaction".to_string()),
                ("reason", kind.to_string()),
            ]
        }));
        Some(Self { span })
    }

    pub(crate) fn span_context(&self) -> Option<SpanContext> {
        SpanContext::from_span(&self.span)
    }

    pub(crate) fn finish(self, will_retry: bool, aborted: bool, error_message: Option<&str>) {
        self.span
            .add_property(|| ("will_retry", will_retry.to_string()));
        self.span.add_property(|| ("aborted", aborted.to_string()));
        if let Some(err) = error_message.filter(|s| !s.is_empty()) {
            let msg = if err.len() > 512 {
                format!("{}…", &err[..511])
            } else {
                err.to_string()
            };
            self.span
                .add_property(|| ("langfuse.observation.level", "ERROR".to_string()));
            self.span
                .add_property(|| ("langfuse.observation.status_message", msg.clone()));
            self.span.add_property(|| ("error_message", msg));
        }
        self.span
            .add_event(Event::new("lifecycle").with_properties(|| {
                [
                    ("kind", "lifecycle".to_string()),
                    ("phase", "end".to_string()),
                    ("name", "agent.compaction".to_string()),
                    ("will_retry", will_retry.to_string()),
                    ("aborted", aborted.to_string()),
                ]
            }));
        drop(self);
    }
}

#[cfg(test)]
mod tests {
    use super::compaction_reason_kind;

    #[test]
    fn middleware_reason_is_not_folded_into_manual() {
        assert_eq!(
            compaction_reason_kind("loop_middleware:focus"),
            "loop_middleware"
        );
        assert_eq!(compaction_reason_kind("manual"), "manual");
        assert_eq!(compaction_reason_kind("threshold auto"), "threshold");
        assert_eq!(compaction_reason_kind("overflow Case1"), "overflow");
    }
}
