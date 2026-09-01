//! Realtime API (Phase A: core text-only voice session; Phase B: audio input/output) -- see
//! docs/superpowers/specs/2026-09-01-realtime-phase-a-design.md and
//! docs/superpowers/specs/2026-09-02-realtime-phase-b-audio-design.md.

pub mod connection;

use crate::client::OpenAI;
use crate::interfaces::realtime;
pub use crate::interfaces::realtime::*;
use crate::shared::response_wrapper::OpenAIResponse;
pub use connection::RealtimeConnection;

pub struct Realtime<'a> {
    openai: &'a OpenAI,
}

impl<'a> Realtime<'a> {
    pub(crate) fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }

    /// Opens a WebSocket connection to the Realtime API.
    pub async fn connect(
        &self,
        req: &realtime::RealtimeConnectRequest,
    ) -> OpenAIResponse<RealtimeConnection> {
        RealtimeConnection::connect(
            self.openai.base_url(),
            self.openai.api_key(),
            req.model.as_deref(),
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use crate::interfaces::realtime::{
        ConversationContentPart, ConversationItem, ConversationMessageItem, RealtimeClientEvent,
        RealtimeResponseCreateParams, RealtimeServerEvent, RealtimeSessionConfig,
    };

    #[test]
    fn session_update_serializes_with_type_tag() {
        let event = RealtimeClientEvent::SessionUpdate {
            event_id: None,
            session: RealtimeSessionConfig::builder()
                .r#type("realtime")
                .model("gpt-realtime-2.1")
                .output_modalities(vec!["text".to_string()])
                .build(),
        };
        let json = serde_json::to_value(&event).unwrap();
        assert_eq!(
            json,
            serde_json::json!({
                "type": "session.update",
                "session": {
                    "type": "realtime",
                    "model": "gpt-realtime-2.1",
                    "output_modalities": ["text"]
                }
            })
        );
    }

    #[test]
    fn conversation_item_create_serializes_user_message() {
        let event = RealtimeClientEvent::ConversationItemCreate {
            event_id: None,
            previous_item_id: None,
            item: ConversationItem::Message(ConversationMessageItem::User {
                content: vec![
                    ConversationContentPart::builder()
                        .r#type("input_text")
                        .text("Hello")
                        .build(),
                ],
                id: None,
                object: None,
                status: None,
            }),
        };
        let json = serde_json::to_value(&event).unwrap();
        assert_eq!(
            json,
            serde_json::json!({
                "type": "conversation.item.create",
                "item": {
                    "type": "message",
                    "role": "user",
                    "content": [{"type": "input_text", "text": "Hello"}]
                }
            })
        );
    }

    #[test]
    fn conversation_item_delete_and_retrieve_serialize() {
        let delete = RealtimeClientEvent::ConversationItemDelete {
            event_id: None,
            item_id: "item_1".to_string(),
        };
        assert_eq!(
            serde_json::to_value(&delete).unwrap(),
            serde_json::json!({"type": "conversation.item.delete", "item_id": "item_1"})
        );

        let retrieve = RealtimeClientEvent::ConversationItemRetrieve {
            event_id: None,
            item_id: "item_1".to_string(),
        };
        assert_eq!(
            serde_json::to_value(&retrieve).unwrap(),
            serde_json::json!({"type": "conversation.item.retrieve", "item_id": "item_1"})
        );
    }

    #[test]
    fn response_create_and_cancel_serialize() {
        let create = RealtimeClientEvent::ResponseCreate {
            event_id: None,
            response: Some(
                RealtimeResponseCreateParams::builder()
                    .instructions("Be concise.")
                    .build(),
            ),
        };
        assert_eq!(
            serde_json::to_value(&create).unwrap(),
            serde_json::json!({
                "type": "response.create",
                "response": {"instructions": "Be concise."}
            })
        );

        let cancel = RealtimeClientEvent::ResponseCancel {
            event_id: None,
            response_id: Some("resp_1".to_string()),
        };
        assert_eq!(
            serde_json::to_value(&cancel).unwrap(),
            serde_json::json!({"type": "response.cancel", "response_id": "resp_1"})
        );
    }

    #[test]
    fn conversation_item_two_level_tag_round_trips_for_all_three_roles() {
        for (role, json_role) in [
            ("system", "system"),
            ("user", "user"),
            ("assistant", "assistant"),
        ] {
            let value = serde_json::json!({
                "type": "message",
                "role": role,
                "content": [{"type": "input_text", "text": "hi"}]
            });
            let item: ConversationItem = serde_json::from_value(value.clone()).unwrap();
            match &item {
                ConversationItem::Message(ConversationMessageItem::System { .. }) => {
                    assert_eq!(json_role, "system")
                }
                ConversationItem::Message(ConversationMessageItem::User { .. }) => {
                    assert_eq!(json_role, "user")
                }
                ConversationItem::Message(ConversationMessageItem::Assistant { .. }) => {
                    assert_eq!(json_role, "assistant")
                }
            }
            // Round-trip: re-serializing must reproduce an equivalent object (same tags).
            let round_tripped = serde_json::to_value(&item).unwrap();
            assert_eq!(round_tripped["type"], "message");
            assert_eq!(round_tripped["role"], json_role);
        }
    }

    #[test]
    fn server_events_deserialize() {
        let cases = [
            (
                serde_json::json!({
                    "type": "session.created",
                    "event_id": "evt_1",
                    "session": {"type": "realtime", "model": "gpt-realtime-2.1"}
                }),
                "SessionCreated",
            ),
            (
                serde_json::json!({
                    "type": "session.updated",
                    "event_id": "evt_2",
                    "session": {"type": "realtime"}
                }),
                "SessionUpdated",
            ),
            (
                serde_json::json!({
                    "type": "conversation.created",
                    "event_id": "evt_3",
                    "conversation": {"id": "conv_1", "object": "realtime.conversation"}
                }),
                "ConversationCreated",
            ),
            (
                serde_json::json!({
                    "type": "conversation.item.created",
                    "event_id": "evt_4",
                    "item": {"type": "message", "role": "user", "content": []}
                }),
                "ConversationItemCreated",
            ),
            (
                serde_json::json!({
                    "type": "conversation.item.added",
                    "event_id": "evt_5",
                    "item": {"type": "message", "role": "assistant", "content": []}
                }),
                "ConversationItemAdded",
            ),
            (
                serde_json::json!({
                    "type": "conversation.item.done",
                    "event_id": "evt_6",
                    "item": {"type": "message", "role": "assistant", "content": []}
                }),
                "ConversationItemDone",
            ),
            (
                serde_json::json!({
                    "type": "conversation.item.deleted",
                    "event_id": "evt_7",
                    "item_id": "item_1"
                }),
                "ConversationItemDeleted",
            ),
            (
                serde_json::json!({
                    "type": "conversation.item.retrieved",
                    "event_id": "evt_8",
                    "item": {"type": "message", "role": "user", "content": []}
                }),
                "ConversationItemRetrieved",
            ),
            (
                serde_json::json!({
                    "type": "response.created",
                    "event_id": "evt_9",
                    "response": {"id": "resp_1", "object": "realtime.response"}
                }),
                "ResponseCreated",
            ),
            (
                serde_json::json!({
                    "type": "response.done",
                    "event_id": "evt_10",
                    "response": {"id": "resp_1", "status": "completed"}
                }),
                "ResponseDone",
            ),
            (
                serde_json::json!({
                    "type": "response.output_item.added",
                    "event_id": "evt_11",
                    "item": {"type": "message", "role": "assistant", "content": []},
                    "output_index": 0,
                    "response_id": "resp_1"
                }),
                "ResponseOutputItemAdded",
            ),
            (
                serde_json::json!({
                    "type": "response.output_item.done",
                    "event_id": "evt_12",
                    "item": {"type": "message", "role": "assistant", "content": []},
                    "output_index": 0,
                    "response_id": "resp_1"
                }),
                "ResponseOutputItemDone",
            ),
            (
                serde_json::json!({
                    "type": "response.content_part.added",
                    "content_index": 0,
                    "event_id": "evt_13",
                    "item_id": "item_1",
                    "output_index": 0,
                    "part": {"type": "text", "text": ""},
                    "response_id": "resp_1"
                }),
                "ResponseContentPartAdded",
            ),
            (
                serde_json::json!({
                    "type": "response.content_part.done",
                    "content_index": 0,
                    "event_id": "evt_14",
                    "item_id": "item_1",
                    "output_index": 0,
                    "part": {"type": "text", "text": "Hello."},
                    "response_id": "resp_1"
                }),
                "ResponseContentPartDone",
            ),
            (
                serde_json::json!({
                    "type": "response.output_text.delta",
                    "content_index": 0,
                    "delta": "Hel",
                    "event_id": "evt_15",
                    "item_id": "item_1",
                    "output_index": 0,
                    "response_id": "resp_1"
                }),
                "ResponseTextDelta",
            ),
            (
                serde_json::json!({
                    "type": "response.output_text.done",
                    "content_index": 0,
                    "event_id": "evt_16",
                    "item_id": "item_1",
                    "output_index": 0,
                    "response_id": "resp_1",
                    "text": "Hello."
                }),
                "ResponseTextDone",
            ),
            (
                serde_json::json!({
                    "type": "error",
                    "event_id": "evt_17",
                    "error": {"message": "boom", "type": "server_error"}
                }),
                "Error",
            ),
            (
                serde_json::json!({
                    "type": "rate_limits.updated",
                    "event_id": "evt_18",
                    "rate_limits": [{"name": "requests", "limit": 100, "remaining": 99}]
                }),
                "RateLimitsUpdated",
            ),
            (
                serde_json::json!({
                    "type": "input_audio_buffer.committed",
                    "event_id": "evt_19",
                    "item_id": "item_1"
                }),
                "InputAudioBufferCommitted",
            ),
            (
                serde_json::json!({
                    "type": "input_audio_buffer.cleared",
                    "event_id": "evt_20"
                }),
                "InputAudioBufferCleared",
            ),
            (
                serde_json::json!({
                    "type": "input_audio_buffer.speech_started",
                    "audio_start_ms": 120,
                    "event_id": "evt_21",
                    "item_id": "item_1"
                }),
                "InputAudioBufferSpeechStarted",
            ),
            (
                serde_json::json!({
                    "type": "input_audio_buffer.speech_stopped",
                    "audio_end_ms": 980,
                    "event_id": "evt_22",
                    "item_id": "item_1"
                }),
                "InputAudioBufferSpeechStopped",
            ),
            (
                serde_json::json!({
                    "type": "input_audio_buffer.timeout_triggered",
                    "audio_end_ms": 5000,
                    "audio_start_ms": 3000,
                    "event_id": "evt_23",
                    "item_id": "item_1"
                }),
                "InputAudioBufferTimeoutTriggered",
            ),
            (
                serde_json::json!({
                    "type": "response.output_audio.delta",
                    "content_index": 0,
                    "delta": "YmFzZTY0",
                    "event_id": "evt_24",
                    "item_id": "item_1",
                    "output_index": 0,
                    "response_id": "resp_1"
                }),
                "ResponseAudioDelta",
            ),
            (
                serde_json::json!({
                    "type": "response.output_audio.done",
                    "content_index": 0,
                    "event_id": "evt_25",
                    "item_id": "item_1",
                    "output_index": 0,
                    "response_id": "resp_1"
                }),
                "ResponseAudioDone",
            ),
            (
                serde_json::json!({
                    "type": "response.output_audio_transcript.delta",
                    "content_index": 0,
                    "delta": "Hel",
                    "event_id": "evt_26",
                    "item_id": "item_1",
                    "output_index": 0,
                    "response_id": "resp_1"
                }),
                "ResponseAudioTranscriptDelta",
            ),
            (
                serde_json::json!({
                    "type": "response.output_audio_transcript.done",
                    "content_index": 0,
                    "event_id": "evt_27",
                    "item_id": "item_1",
                    "output_index": 0,
                    "response_id": "resp_1",
                    "transcript": "Hello."
                }),
                "ResponseAudioTranscriptDone",
            ),
        ];

        for (value, expected_variant) in cases {
            let event: RealtimeServerEvent = serde_json::from_value(value).unwrap();
            let debug = format!("{event:?}");
            assert!(
                debug.starts_with(expected_variant),
                "expected {expected_variant}, got {debug}"
            );
        }
    }

    #[test]
    fn input_audio_buffer_events_serialize() {
        let append = RealtimeClientEvent::InputAudioBufferAppend {
            event_id: None,
            audio: "YmFzZTY0".to_string(),
        };
        assert_eq!(
            serde_json::to_value(&append).unwrap(),
            serde_json::json!({"type": "input_audio_buffer.append", "audio": "YmFzZTY0"})
        );

        let commit = RealtimeClientEvent::InputAudioBufferCommit { event_id: None };
        assert_eq!(
            serde_json::to_value(&commit).unwrap(),
            serde_json::json!({"type": "input_audio_buffer.commit"})
        );

        let clear = RealtimeClientEvent::InputAudioBufferClear { event_id: None };
        assert_eq!(
            serde_json::to_value(&clear).unwrap(),
            serde_json::json!({"type": "input_audio_buffer.clear"})
        );
    }

    #[test]
    fn realtime_audio_format_tagged_variants_serialize() {
        use crate::interfaces::realtime::RealtimeAudioFormat;

        assert_eq!(
            serde_json::to_value(RealtimeAudioFormat::AudioPcm { rate: Some(24000) }).unwrap(),
            serde_json::json!({"type": "audio/pcm", "rate": 24000})
        );
        assert_eq!(
            serde_json::to_value(RealtimeAudioFormat::AudioPcmu).unwrap(),
            serde_json::json!({"type": "audio/pcmu"})
        );
        assert_eq!(
            serde_json::to_value(RealtimeAudioFormat::AudioPcma).unwrap(),
            serde_json::json!({"type": "audio/pcma"})
        );
    }

    #[test]
    fn realtime_turn_detection_tagged_variants_round_trip() {
        use crate::interfaces::realtime::RealtimeTurnDetection;

        let server_vad = RealtimeTurnDetection::ServerVad {
            create_response: Some(true),
            idle_timeout_ms: None,
            interrupt_response: Some(true),
            prefix_padding_ms: Some(300),
            silence_duration_ms: Some(500),
            threshold: Some(0.5),
        };
        let json = serde_json::to_value(&server_vad).unwrap();
        assert_eq!(json["type"], "server_vad");
        assert_eq!(json["prefix_padding_ms"], 300);
        let parsed: RealtimeTurnDetection = serde_json::from_value(json).unwrap();
        assert!(matches!(parsed, RealtimeTurnDetection::ServerVad { .. }));

        let semantic_vad = RealtimeTurnDetection::SemanticVad {
            create_response: Some(true),
            eagerness: Some("high".to_string()),
            interrupt_response: None,
        };
        let json = serde_json::to_value(&semantic_vad).unwrap();
        assert_eq!(json["type"], "semantic_vad");
        assert_eq!(json["eagerness"], "high");
    }

    #[test]
    fn session_config_serializes_audio_block() {
        use crate::interfaces::realtime::{
            RealtimeAudioConfig, RealtimeAudioConfigInput, RealtimeAudioFormat,
            RealtimeAudioOutputConfig, RealtimeNoiseReduction, RealtimeTurnDetection,
        };

        let session = RealtimeSessionConfig::builder()
            .r#type("realtime")
            .audio(
                RealtimeAudioConfig::builder()
                    .input(
                        RealtimeAudioConfigInput::builder()
                            .format(RealtimeAudioFormat::AudioPcm { rate: Some(24000) })
                            .noise_reduction(
                                RealtimeNoiseReduction::builder()
                                    .r#type("near_field")
                                    .build(),
                            )
                            .turn_detection(RealtimeTurnDetection::ServerVad {
                                create_response: Some(true),
                                idle_timeout_ms: None,
                                interrupt_response: Some(true),
                                prefix_padding_ms: None,
                                silence_duration_ms: None,
                                threshold: None,
                            })
                            .build(),
                    )
                    .output(
                        RealtimeAudioOutputConfig::builder()
                            .format(RealtimeAudioFormat::AudioPcm { rate: Some(24000) })
                            .voice(serde_json::json!("marin"))
                            .build(),
                    )
                    .build(),
            )
            .build();

        let json = serde_json::to_value(&session).unwrap();
        assert_eq!(json["audio"]["input"]["format"]["type"], "audio/pcm");
        assert_eq!(
            json["audio"]["input"]["noise_reduction"]["type"],
            "near_field"
        );
        assert_eq!(
            json["audio"]["input"]["turn_detection"]["type"],
            "server_vad"
        );
        assert_eq!(json["audio"]["output"]["voice"], "marin");
    }
}
