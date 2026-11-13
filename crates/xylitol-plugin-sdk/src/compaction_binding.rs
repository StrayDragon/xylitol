use std::sync::Arc;

use crate::XyModel;
use crate::generate::{ObsSessionContext, XyGenerateOptions};
use crate::model::{THINKING_OFF, XyModelConfig, XyModelKind, XyModelMeta};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompactionModelAttribution {
    pub requested_model: Option<String>,
    pub actual_model: String,
    pub fallback: bool,
    pub thinking_rejected: bool,
}

impl CompactionModelAttribution {
    pub fn enrich_obs(&self, obs: &ObsSessionContext) -> ObsSessionContext {
        let mut out = obs.clone();
        out.compaction_requested_model = self.requested_model.clone();
        out.compaction_actual_model = Some(self.actual_model.clone());
        out.compaction_model_fallback = Some(self.fallback);
        out.compaction_thinking_rejected = Some(self.thinking_rejected);
        out
    }

    pub fn notice_message(&self) -> Option<String> {
        if self.fallback {
            let requested = self
                .requested_model
                .as_deref()
                .unwrap_or("configured summary model");
            Some(format!(
                "Compaction summary fell back to {} (could not use {})",
                self.actual_model, requested
            ))
        } else if self.thinking_rejected {
            Some(format!(
                "Compaction summary used {} without thinking (thinking rejected)",
                self.actual_model
            ))
        } else {
            None
        }
    }
}

#[derive(Clone)]
pub struct CompactionSummaryBinding {
    pub model: Arc<dyn XyModel>,
    #[cfg_attr(not(test), allow(dead_code))]
    pub meta: XyModelMeta,
    pub generate_options: XyGenerateOptions,
    pub attribution: CompactionModelAttribution,
}

impl CompactionSummaryBinding {
    pub fn for_test(model: Arc<dyn XyModel>, wire_id: &str) -> Self {
        let meta = XyModelMeta {
            id: wire_id.into(),
            config: XyModelConfig {
                kind: XyModelKind::Fake,
                api_key: String::new(),
                model: wire_id.into(),
                base_url: None,
                api: None,
                compat: None,
                prompt_cache: None,
            },
            display_name: wire_id.into(),
            thinking: false,
            context_window: 128_000,
            api: String::new(),
            provider: "fake".into(),
            cost_input: 0.0,
            cost_output: 0.0,
            cost_cache_read: 0.0,
            cost_cache_write: 0.0,
            max_tokens: 0,
            thinking_levels: vec![THINKING_OFF.into()],
            thinking_level_map: Default::default(),
        };
        Self {
            model,
            meta,
            generate_options: XyGenerateOptions::default(),
            attribution: CompactionModelAttribution {
                requested_model: None,
                actual_model: wire_id.into(),
                fallback: false,
                thinking_rejected: false,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thinking_rejected_emits_notice_through_same_pipeline() {
        let mut attribution = CompactionModelAttribution {
            requested_model: None,
            actual_model: "small".into(),
            fallback: false,
            thinking_rejected: true,
        };
        let notice = attribution.notice_message().expect("thinking notice");
        assert!(notice.contains("without thinking"), "{notice}");
        attribution.thinking_rejected = false;
        assert!(attribution.notice_message().is_none());
        attribution.fallback = true;
        let notice = attribution.notice_message().expect("fallback notice");
        assert!(notice.contains("fell back"), "{notice}");
    }
}
