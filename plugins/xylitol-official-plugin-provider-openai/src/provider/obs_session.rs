//! Current xylitol session identity for Langfuse OTEL attributes (c1480).
//!
//! SSOT lives in `xylitol-plugin-sdk`. This module re-exports the shared slot
//! so provider plugins and tests keep a stable import path.

pub use xylitol_plugin_sdk::{
    ObsSessionContext, ObsSessionScope, XYLITOL_OBS_LANE_ATTR, XYLITOL_OBS_LANE_LLM,
    clear_obs_session, langfuse_generation_properties, langfuse_generation_properties_from,
    langfuse_observation_properties, langfuse_observation_properties_from,
    langfuse_session_properties, langfuse_session_properties_from, obs_session_context,
    set_obs_session, set_obs_session_name, xylitol_obs_lane_properties,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn id_without_name() {
        let _g = ObsSessionScope::enter(ObsSessionContext::default());
        set_obs_session("sid-1", None);
        let p = langfuse_session_properties();
        assert_eq!(
            p,
            vec![
                ("langfuse.session.id".into(), "sid-1".into()),
                ("xylitol.session.id".into(), "sid-1".into()),
            ]
        );
        assert!(!p.iter().any(|(k, _)| k.contains("session_name")));
    }

    #[test]
    fn id_with_name_and_rename() {
        let _g = ObsSessionScope::enter(ObsSessionContext::default());
        set_obs_session("sid-2", Some("alpha".into()));
        let p = langfuse_session_properties();
        assert!(p.contains(&("langfuse.session.id".into(), "sid-2".into())));
        assert!(p.contains(&(
            "langfuse.trace.metadata.session_name".into(),
            "alpha".into()
        )));
        set_obs_session_name(Some("beta"));
        let p2 = langfuse_session_properties();
        assert!(p2.contains(&("langfuse.session.id".into(), "sid-2".into())));
        assert!(p2.contains(&("langfuse.trace.metadata.session_name".into(), "beta".into())));
        set_obs_session_name(Some("  "));
        let p3 = langfuse_session_properties();
        assert_eq!(
            p3,
            vec![
                ("langfuse.session.id".into(), "sid-2".into()),
                ("xylitol.session.id".into(), "sid-2".into()),
            ]
        );
    }

    #[test]
    fn generation_properties_include_type_and_model() {
        let _g = ObsSessionScope::enter(ObsSessionContext::default());
        set_obs_session("sid-g", None);
        let p = langfuse_generation_properties("gpt-test");
        assert!(p.contains(&("langfuse.observation.type".into(), "generation".into())));
        assert!(p.contains(&("langfuse.observation.model.name".into(), "gpt-test".into())));
        assert!(
            !p.iter()
                .any(|(k, _)| k == "gen_ai.request.model" || k == "model")
        );
        assert!(p.contains(&("xylitol.session.id".into(), "sid-g".into())));
        assert!(p.contains(&(XYLITOL_OBS_LANE_ATTR.into(), XYLITOL_OBS_LANE_LLM.into())));
    }

    #[test]
    fn scope_does_not_pollute_process_slot() {
        clear_obs_session();
        {
            let _g = ObsSessionScope::enter(ObsSessionContext::default());
            set_obs_session("scoped-only", None);
            assert_eq!(
                obs_session_context().session_id.as_deref(),
                Some("scoped-only")
            );
        }
        assert!(obs_session_context().session_id.is_none());
    }

    #[test]
    fn properties_from_snapshot_ignore_process_slot() {
        let _g = ObsSessionScope::enter(ObsSessionContext {
            session_id: Some("process-wrong".into()),
            session_name: Some("wrong-name".into()),
            ..Default::default()
        });
        let snap = ObsSessionContext {
            session_id: Some("bookmark-a".into()),
            session_name: Some("alpha".into()),
            ..Default::default()
        };
        let p = langfuse_session_properties_from(&snap);
        assert_eq!(
            p,
            vec![
                ("langfuse.session.id".into(), "bookmark-a".into()),
                ("xylitol.session.id".into(), "bookmark-a".into()),
                (
                    "langfuse.trace.metadata.session_name".into(),
                    "alpha".into()
                ),
            ]
        );
        let g = langfuse_generation_properties_from("gpt-test", &snap);
        assert!(g.contains(&("langfuse.session.id".into(), "bookmark-a".into())));
        assert!(g.contains(&("xylitol.session.id".into(), "bookmark-a".into())));
        assert!(
            !g.iter()
                .any(|(k, _)| k == "xylitol.session.llm_gateway_session_id")
        );
        assert!(!g.iter().any(|(_, v)| v == "process-wrong"));
    }

    #[test]
    fn compaction_attribution_properties_emitted() {
        let snap = ObsSessionContext {
            compaction_requested_model: Some("summary-task".into()),
            compaction_actual_model: Some("main-wire".into()),
            compaction_model_fallback: Some(true),
            compaction_thinking_rejected: Some(true),
            ..Default::default()
        };
        let p = langfuse_session_properties_from(&snap);
        assert!(p.contains(&(
            "xylitol.compaction.requested_model".into(),
            "summary-task".into(),
        )));
        assert!(p.contains(&("xylitol.compaction.actual_model".into(), "main-wire".into(),)));
        assert!(p.contains(&("xylitol.compaction.model_fallback".into(), "true".into(),)));
        assert!(p.contains(&("xylitol.compaction.thinking_rejected".into(), "true".into(),)));
    }

    #[test]
    fn properties_omit_unpresented_gateway_and_missing_fork_edge() {
        let snap = ObsSessionContext {
            session_id: Some("s1".into()),
            parent_session_id: Some("parent".into()),
            fork_at_entry_id: Some("u6".into()),
            llm_gateway_session_id: Some("gw".into()),
            ..Default::default()
        };
        let p = langfuse_session_properties_from(&snap);
        assert!(p.contains(&("xylitol.session.parent_session_id".into(), "parent".into())));
        assert!(p.contains(&("xylitol.session.fork_at_entry_id".into(), "u6".into())));
        assert!(p.contains(&("xylitol.session.llm_gateway_session_id".into(), "gw".into())));
        let bare = ObsSessionContext {
            session_id: Some("s1".into()),
            ..Default::default()
        };
        let p2 = langfuse_session_properties_from(&bare);
        assert!(
            !p2.iter()
                .any(|(k, _)| k.starts_with("xylitol.session.parent"))
        );
        assert!(
            !p2.iter()
                .any(|(k, _)| k == "xylitol.session.llm_gateway_session_id")
        );
        assert!(
            !p2.iter()
                .any(|(k, _)| k == "xylitol.session.fork_at_entry_id")
        );
    }
}
