//! ResponsesAssembler — sole business-layout entry for `/v1/responses` bodies (c1890).

use serde_json::Value;

use crate::dto::{AiBridgeMessage, AiBridgeToolSchema};
use crate::thinking::AiBridgeGenerateOptions;
use crate::wire_policy::WirePolicy;

use super::{apply_responses_wire_policy, assemble_responses_body};

/// Constructs OpenAI Responses JSON bodies under a fixed [`WirePolicy`].
///
/// Sole public business-layout entry for `/v1/responses` bodies (c1890).
/// Internal `assemble_responses_body` / `apply_responses_wire_policy` stay crate-private.
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
        assemble_responses_body(model, messages, tools, stream, options, &self.wire_policy)
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
        let via_fn =
            assemble_responses_body("m", msgs.clone(), &[], false, &opts, &WirePolicy::default());
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
}
