//! ResponsesAssembler — sole business-layout entry for `/v1/responses` bodies (c1890).

use serde_json::Value;

use crate::dto::{AiBridgeMessage, AiBridgeToolSchema};
use crate::thinking::AiBridgeGenerateOptions;
use crate::wire_policy::WirePolicy;

use super::{apply_responses_wire_policy, assemble_responses_body_with_diagnostics};

/// Constructs OpenAI Responses JSON bodies under a fixed [`WirePolicy`].
///
/// Sole public business-layout entry for `/v1/responses` bodies (c1890).
/// Internal `assemble_responses_body_with_diagnostics` / `apply_responses_wire_policy` stay crate-private.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResponsesAssembler {
    wire_policy: WirePolicy,
}

impl Default for ResponsesAssembler {
    fn default() -> Self {
        Self::new(WirePolicy::default())
    }
}

impl ResponsesAssembler {
    pub fn new(wire_policy: WirePolicy) -> Self {
        Self { wire_policy }
    }

    pub fn wire_policy(self) -> WirePolicy {
        self.wire_policy
    }

    /// Assemble a `/v1/responses` body (pi-aligned store/strict/summary/include).
    pub fn assemble(
        self,
        model: &str,
        messages: Vec<AiBridgeMessage>,
        tools: &[AiBridgeToolSchema],
        stream: bool,
        options: &AiBridgeGenerateOptions,
    ) -> Value {
        self.assemble_with_diagnostics(model, messages, tools, stream, options)
            .0
    }

    /// Like [`Self::assemble`], also returning full-replay omit diagnostics
    /// (illegal `thinkingSignature` JSON omitted from `input`).
    pub fn assemble_with_diagnostics(
        self,
        model: &str,
        messages: Vec<AiBridgeMessage>,
        tools: &[AiBridgeToolSchema],
        stream: bool,
        options: &AiBridgeGenerateOptions,
    ) -> (Value, Vec<crate::dto::Diagnostic>) {
        assemble_responses_body_with_diagnostics(
            model,
            messages,
            tools,
            stream,
            options,
            &self.wire_policy,
        )
    }

    /// Apply wire-policy stripping to an already-built body (test / inject harness).
    pub fn apply_wire_policy(self, body: &mut Value) {
        apply_responses_wire_policy(body, &self.wire_policy);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dto::AiBridgeMessage;
    use crate::wire_policy::{Compat, ExtraPolicy};

    #[test]
    fn default_assembler_matches_assemble_fn() {
        let msgs = vec![AiBridgeMessage::user("hi")];
        let opts = AiBridgeGenerateOptions::default();
        let (via_fn, _) = assemble_responses_body_with_diagnostics(
            "m",
            msgs.clone(),
            &[],
            false,
            &opts,
            &WirePolicy::default(),
        );
        let via_asm = ResponsesAssembler::default().assemble("m", msgs, &[], false, &opts);
        assert_eq!(via_asm, via_fn);
    }

    #[test]
    fn wire_policy_diff_changes_allowed_knobs() {
        let deny = ResponsesAssembler::default();
        let allow = ResponsesAssembler::new(WirePolicy {
            compat: Compat::Generic,
            extra_policy: ExtraPolicy {
                prompt_cache_usage: false,
                prompt_cache_key: true,
                previous_response_id: true,
            },
        });
        let mut denied = serde_json::json!({
            "previous_response_id": "resp_1",
            "prompt_cache_key": "ck",
        });
        let mut allowed = denied.clone();
        deny.apply_wire_policy(&mut denied);
        allow.apply_wire_policy(&mut allowed);
        assert!(denied.get("previous_response_id").is_none());
        assert!(denied.get("prompt_cache_key").is_none());
        assert_eq!(allowed["previous_response_id"], "resp_1");
        assert_eq!(allowed["prompt_cache_key"], "ck");
    }

    #[test]
    fn assemble_with_diagnostics_reports_illegal_signature_omit() {
        use crate::dto::{AiBridgePart, AiBridgeStopReason};

        let msgs = vec![AiBridgeMessage::AssistantMessage {
            content: vec![
                AiBridgePart::Thinking {
                    thinking: "x".into(),
                    redacted: false,
                    thinking_signature: Some("not-json".into()),
                },
                AiBridgePart::text("ok"),
            ],
            stop_reason: Some(AiBridgeStopReason::Stop),
            usage: None,
            api: String::new(),
            provider: String::new(),
            model: String::new(),
            response_id: None,
            error_message: None,
            timestamp: 0,
            diagnostics: Vec::new(),
        }];
        let opts = AiBridgeGenerateOptions::default();
        let (body, diags) =
            ResponsesAssembler::default().assemble_with_diagnostics("m", msgs, &[], false, &opts);
        let input = body["input"].as_array().expect("input");
        assert!(
            input
                .iter()
                .all(|i| i.get("type") != Some(&serde_json::json!("reasoning"))),
            "{input:?}"
        );
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("omit illegal thinkingSignature"));
        assert_eq!(diags[0].source.as_deref(), Some("openai-responses"));
    }

    #[test]
    fn assemble_prefix_idempotent_and_serde_roundtrip() {
        use crate::dto::{AiBridgePart, AiBridgeStopReason, AiBridgeToolSchema};

        let sig = r#"{"type":"reasoning","id":"rs_pab27","summary":[]}"#;
        let msgs = vec![
            AiBridgeMessage::user("hi"),
            AiBridgeMessage::AssistantMessage {
                content: vec![
                    AiBridgePart::Thinking {
                        thinking: "t".into(),
                        redacted: false,
                        thinking_signature: Some(sig.into()),
                    },
                    AiBridgePart::text("reply"),
                ],
                stop_reason: Some(AiBridgeStopReason::Stop),
                usage: None,
                api: "openai-responses".into(),
                provider: "test".into(),
                model: "m".into(),
                response_id: None,
                error_message: None,
                timestamp: 0,
                diagnostics: Vec::new(),
            },
            AiBridgeMessage::user("$ ls\na.txt"),
        ];
        let opts = AiBridgeGenerateOptions {
            system_prompt: Some("Current date: 2026-08-06".into()),
            thinking_level: "medium".into(),
            ..Default::default()
        };
        let tools = [AiBridgeToolSchema {
            name: "read".into(),
            description: "read".into(),
            parameters: serde_json::json!({"type": "object", "properties": {}}),
        }];
        let asm = ResponsesAssembler::default();
        let a = asm.assemble("m", msgs.clone(), &tools, false, &opts);
        let b = asm.assemble("m", msgs.clone(), &tools, false, &opts);
        assert_eq!(a["input"], b["input"]);
        assert_eq!(a["tools"], b["tools"]);

        let wire = serde_json::to_value(&msgs).expect("ser");
        let back: Vec<AiBridgeMessage> = serde_json::from_value(wire).expect("de");
        let c = asm.assemble("m", back, &tools, false, &opts);
        assert_eq!(
            c["input"], a["input"],
            "serde round-trip must not change assemble input (pab27)"
        );
        assert_eq!(c["tools"], a["tools"]);
    }
}
