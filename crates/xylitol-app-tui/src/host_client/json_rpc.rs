use serde::de::DeserializeOwned;
use serde_json::Value;
use xylitol_protocol::codec;
use xylitol_protocol::method;
use xylitol_protocol::{
    ApprovalRequestedPayload, BashOutputPayload, ErrorObject, HostDescribeValue, Message,
    Notification, Outcome, PROTOCOL_VERSION, QuestionRequestedPayload, RequestId, RpcResult,
    SessionEventPayload, SessionResourcesPayload, SessionResyncRequiredPayload,
    SessionSubscribedPayload, WIRE_FORMAT_JSONRPC,
};

use super::HostClientError;

#[derive(Debug)]
pub(crate) enum Inbound {
    Response {
        id: Option<String>,
        result: RpcResult,
    },
    Notification(Notification),
    Ignored,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Downlink {
    SessionEvent(SessionEventPayload),
    SessionSubscribed(SessionSubscribedPayload),
    SessionResyncRequired(SessionResyncRequiredPayload),
    SessionResources(SessionResourcesPayload),
    BashOutput(BashOutputPayload),
    Interaction {
        method: &'static str,
        call_id: String,
        params: Value,
    },
}

#[derive(Debug, thiserror::Error)]
#[error("downlink {method} payload: {source}")]
pub(crate) struct DownlinkError {
    method: String,
    source: serde_json::Error,
}

pub(crate) fn request_text(id: &str, name: &str, params: &Value) -> String {
    codec::request_value(&RequestId::Str(id.to_string()), name, params).to_string()
}

pub(crate) fn decode_text_frame(text: &str) -> Result<Inbound, String> {
    let message = codec::decode_message(text).map_err(|rejected| rejected.error.message)?;
    Ok(match message {
        Message::Response(response) => Inbound::Response {
            id: response.id.as_ref().map(RequestId::key),
            result: result_of(response.outcome),
        },
        Message::Notification(note) => Inbound::Notification(note),
        Message::Request(_) => Inbound::Ignored,
    })
}

fn result_of(outcome: Outcome) -> RpcResult {
    match outcome {
        Outcome::Result(value) => RpcResult::ok_value(value),
        Outcome::Error(ErrorObject {
            code,
            message,
            data_code,
        }) => RpcResult::error(data_code.unwrap_or_else(|| code.to_string()), message),
    }
}

pub(crate) fn check_describe(result: &RpcResult) -> Result<(), HostClientError> {
    if !result.ok {
        let detail = result
            .error
            .as_ref()
            .map_or_else(String::new, |e| format!("{}: {}", e.code, e.details));
        return Err(HostClientError::transport(format!(
            "host.describe failed: {detail}"
        )));
    }
    let described = result
        .value
        .clone()
        .and_then(|value| serde_json::from_value::<HostDescribeValue>(value).ok());
    let got = described.as_ref().map_or(0, |d| d.protocol);
    if got != PROTOCOL_VERSION {
        return Err(HostClientError::ProtocolMismatch {
            got,
            expected: PROTOCOL_VERSION,
        });
    }
    let formats = described.map(|d| d.formats).unwrap_or_default();
    if formats.iter().any(|format| format == WIRE_FORMAT_JSONRPC) {
        Ok(())
    } else {
        Err(HostClientError::WireFormatMismatch {
            formats,
            expected: WIRE_FORMAT_JSONRPC,
        })
    }
}

pub(crate) fn decode_downlink(note: Notification) -> Result<Option<Downlink>, DownlinkError> {
    let Notification {
        method: name,
        params,
    } = note;
    let downlink = match name.as_str() {
        method::SESSION_EVENT => Downlink::SessionEvent(parse(&name, params)?),
        method::SESSION_SUBSCRIBED => Downlink::SessionSubscribed(parse(&name, params)?),
        method::SESSION_RESYNC_REQUIRED => Downlink::SessionResyncRequired(parse(&name, params)?),
        method::SESSION_RESOURCES => Downlink::SessionResources(parse(&name, params)?),
        method::BASH_OUTPUT => Downlink::BashOutput(parse(&name, params)?),
        method::APPROVAL_REQUESTED => {
            let call_id = parse::<ApprovalRequestedPayload>(&name, params.clone())?.call_id;
            Downlink::Interaction {
                method: method::APPROVAL_REQUESTED,
                call_id,
                params,
            }
        }
        method::QUESTION_REQUESTED => {
            let call_id = parse::<QuestionRequestedPayload>(&name, params.clone())?.call_id;
            Downlink::Interaction {
                method: method::QUESTION_REQUESTED,
                call_id,
                params,
            }
        }
        _ => return Ok(None),
    };
    Ok(Some(downlink))
}

fn parse<T: DeserializeOwned>(name: &str, params: Value) -> Result<T, DownlinkError> {
    serde_json::from_value(params).map_err(|source| DownlinkError {
        method: name.to_string(),
        source,
    })
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use xylitol_protocol::Event;

    use super::*;

    fn decode(name: &str, params: Value) -> Downlink {
        decode_downlink(Notification {
            method: name.to_string(),
            params,
        })
        .expect("payload decodes")
        .expect("registered downlink method")
    }

    #[test]
    fn request_text_is_jsonrpc_with_string_id() {
        let text = request_text("req-1", "prompt", &json!({"message": "hi"}));
        let value: Value = serde_json::from_str(&text).expect("request is json");
        assert_eq!(value["jsonrpc"], "2.0");
        assert_eq!(value["id"], "req-1");
        assert_eq!(value["method"], "prompt");
        assert_eq!(value["params"], json!({"message": "hi"}));
        assert_eq!(value.as_object().map(serde_json::Map::len), Some(4));
    }

    #[test]
    fn response_frame_maps_result_by_string_id() {
        match decode_text_frame(r#"{"jsonrpc":"2.0","id":"r1","result":{"seq":1}}"#)
            .expect("response frame")
        {
            Inbound::Response { id, result } => {
                assert_eq!(id.as_deref(), Some("r1"));
                assert_eq!(result, RpcResult::ok_value(json!({"seq": 1})));
            }
            other => panic!("expected response, got {other:?}"),
        }
    }

    #[test]
    fn business_error_maps_data_code() {
        match decode_text_frame(
            r#"{"jsonrpc":"2.0","id":"r2","error":{"code":-32000,"message":"gone","data":{"code":"not_found"}}}"#,
        )
        .expect("error frame")
        {
            Inbound::Response { id, result } => {
                assert_eq!(id.as_deref(), Some("r2"));
                assert_eq!(result, RpcResult::error("not_found", "gone"));
            }
            other => panic!("expected response, got {other:?}"),
        }
    }

    #[test]
    fn error_without_data_code_falls_back_to_numeric_code() {
        match decode_text_frame(
            r#"{"jsonrpc":"2.0","id":"r3","error":{"code":-32603,"message":"boom"}}"#,
        )
        .expect("error frame")
        {
            Inbound::Response { result, .. } => {
                assert_eq!(result, RpcResult::error("-32603", "boom"));
            }
            other => panic!("expected response, got {other:?}"),
        }
    }

    #[test]
    fn null_id_response_has_no_pending_key() {
        match decode_text_frame(
            r#"{"jsonrpc":"2.0","id":null,"error":{"code":-32700,"message":"parse"}}"#,
        )
        .expect("parse-error frame")
        {
            Inbound::Response { id, .. } => assert_eq!(id, None),
            other => panic!("expected response, got {other:?}"),
        }
    }

    #[test]
    fn server_request_and_notification_frames_classify() {
        assert!(matches!(
            decode_text_frame(r#"{"jsonrpc":"2.0","id":"x","method":"prompt","params":{}}"#),
            Ok(Inbound::Ignored)
        ));
        assert!(matches!(
            decode_text_frame(r#"{"jsonrpc":"2.0","method":"session/resync_required","params":{"session_id":"s0"}}"#),
            Ok(Inbound::Notification(note)) if note.method == "session/resync_required"
        ));
    }

    #[test]
    fn malformed_and_batch_frames_are_rejected() {
        assert!(decode_text_frame("not json").is_err());
        assert!(decode_text_frame("[]").is_err());
        assert!(decode_text_frame(r#"[{"jsonrpc":"2.0","id":"1","result":{}}]"#).is_err());
    }

    #[test]
    fn session_event_decodes_seq_and_event_payload() {
        let Downlink::SessionEvent(payload) = decode(
            method::SESSION_EVENT,
            json!({"session_id": "s0", "seq": 7, "event": {"type": "text_delta", "text": "hi"}}),
        ) else {
            panic!("session/event must decode to SessionEvent");
        };
        assert_eq!(payload.session_id, "s0");
        assert_eq!(payload.seq, 7);
        assert_eq!(payload.event, Event::TextDelta { text: "hi".into() });
    }

    #[test]
    fn session_event_with_unknown_variant_decodes_to_unknown() {
        let Downlink::SessionEvent(payload) = decode(
            method::SESSION_EVENT,
            json!({"session_id": "s0", "seq": 8, "event": {"type": "future_kind"}}),
        ) else {
            panic!("session/event must decode to SessionEvent");
        };
        assert_eq!(payload.seq, 8);
        assert_eq!(payload.event, Event::Unknown);
    }

    #[test]
    fn subscribed_decodes_inflight_flag() {
        let Downlink::SessionSubscribed(payload) = decode(
            method::SESSION_SUBSCRIBED,
            json!({"session_id": "s0", "seq": 3, "inflight": true}),
        ) else {
            panic!("session/subscribed must decode to SessionSubscribed");
        };
        assert_eq!(payload.seq, 3);
        assert!(payload.inflight);
    }

    #[test]
    fn resync_required_decodes_session_id() {
        let Downlink::SessionResyncRequired(payload) =
            decode(method::SESSION_RESYNC_REQUIRED, json!({"session_id": "s0"}))
        else {
            panic!("session/resync_required must decode to SessionResyncRequired");
        };
        assert_eq!(payload.session_id, "s0");
    }

    #[test]
    fn resources_decodes_snapshot_value() {
        let Downlink::SessionResources(payload) = decode(
            method::SESSION_RESOURCES,
            json!({"session_id": "s0", "snapshot": {"skill_names": ["alpha"]}}),
        ) else {
            panic!("session/resources must decode to SessionResources");
        };
        assert_eq!(payload.snapshot, json!({"skill_names": ["alpha"]}));
    }

    #[test]
    fn bash_output_decodes_chunk_fields() {
        let Downlink::BashOutput(payload) = decode(
            method::BASH_OUTPUT,
            json!({"session_id": "s0", "bash_id": "b1", "seq": 2, "data": "hi\n"}),
        ) else {
            panic!("bash/output must decode to BashOutput");
        };
        assert_eq!(payload.bash_id, "b1");
        assert_eq!(payload.seq, 2);
        assert_eq!(payload.data, "hi\n");
    }

    #[test]
    fn approval_and_question_keep_call_id_and_raw_params() {
        let Downlink::Interaction {
            method: approval,
            call_id,
            params,
        } = decode(
            method::APPROVAL_REQUESTED,
            json!({"call_id": "c1", "tool": "bash"}),
        )
        else {
            panic!("approval/requested must decode to Interaction");
        };
        assert_eq!(approval, method::APPROVAL_REQUESTED);
        assert_eq!(call_id, "c1");
        assert_eq!(params["tool"], "bash");

        let Downlink::Interaction {
            method: question,
            call_id,
            params,
        } = decode(
            method::QUESTION_REQUESTED,
            json!({"call_id": "q1", "questions": []}),
        )
        else {
            panic!("question/requested must decode to Interaction");
        };
        assert_eq!(question, method::QUESTION_REQUESTED);
        assert_eq!(call_id, "q1");
        assert_eq!(params["questions"], json!([]));
    }

    #[test]
    fn unregistered_downlink_method_is_ignored() {
        let outcome = decode_downlink(Notification {
            method: "session/unknown_kind".into(),
            params: json!({}),
        });
        assert!(matches!(outcome, Ok(None)));
    }

    #[test]
    fn malformed_payload_error_names_the_method() {
        let err = decode_downlink(Notification {
            method: method::BASH_OUTPUT.into(),
            params: json!({"session_id": "s0"}),
        })
        .expect_err("bash/output without bash_id is malformed");
        assert!(err.to_string().contains("bash/output"), "{err}");
    }

    #[test]
    fn describe_success_accepts_protocol_and_jsonrpc_format() {
        let result = RpcResult::ok_value(
            json!({"protocol": PROTOCOL_VERSION, "formats": [WIRE_FORMAT_JSONRPC]}),
        );
        assert!(check_describe(&result).is_ok());
    }

    #[test]
    fn describe_protocol_mismatch_is_fatal() {
        let result = RpcResult::ok_value(json!({"protocol": 3, "formats": ["jsonrpc"]}));
        let err = check_describe(&result).expect_err("protocol mismatch");
        assert!(
            matches!(err, HostClientError::ProtocolMismatch { got: 3, expected } if expected == PROTOCOL_VERSION),
            "{err:?}"
        );
        assert!(err.is_fatal_mismatch());
    }

    #[test]
    fn describe_without_jsonrpc_format_is_fatal() {
        let result = RpcResult::ok_value(
            json!({"protocol": PROTOCOL_VERSION, "formats": ["legacy-binary"]}),
        );
        let err = check_describe(&result).expect_err("format mismatch");
        assert!(
            matches!(&err, HostClientError::WireFormatMismatch { formats, expected }
                if formats == &["legacy-binary".to_string()] && *expected == WIRE_FORMAT_JSONRPC),
            "{err:?}"
        );
        assert!(err.is_fatal_mismatch());
    }

    #[test]
    fn describe_without_formats_is_fatal() {
        let result = RpcResult::ok_value(json!({"protocol": PROTOCOL_VERSION}));
        let err = check_describe(&result).expect_err("missing formats");
        assert!(
            matches!(&err, HostClientError::WireFormatMismatch { formats, .. } if formats.is_empty()),
            "{err:?}"
        );
    }

    #[test]
    fn describe_error_result_is_retryable_transport() {
        let result = RpcResult::error("internal_error", "boom");
        let err = check_describe(&result).expect_err("describe error");
        assert!(matches!(err, HostClientError::Transport(_)));
        assert!(!err.is_fatal_mismatch());
    }
}
