//! Realtime `calls` -- see docs/superpowers/specs/2026-09-02-realtime-phase-d-sip-calls-design.md.
//! Plain REST, unlike the WebSocket session in `super::connection` -- fits this crate's
//! ordinary `interfaces`/`apis` + `wiremock` pattern directly.

use crate::client::OpenAI;
use crate::interfaces::realtime;
use crate::shared::response_wrapper::{OpenAIError, OpenAIResponse};
use reqwest::multipart::{Form, Part};

/// Builds the multipart body for `POST /realtime/calls`. Confirmed via source
/// (`encode_multipart`'s call in `calls.py`): `sdp` is a plain-text part typed
/// `application/sdp`, `session` (when present) is a JSON-encoded text part typed
/// `application/json`.
fn create_call_form(req: &realtime::CreateCallRequest) -> OpenAIResponse<Form> {
    let sdp_part = Part::text(req.sdp.clone())
        .mime_str("application/sdp")
        .expect("\"application/sdp\" is always a valid mime type");
    let mut form = Form::new().part("sdp", sdp_part);

    if let Some(session) = &req.session {
        let session_json = serde_json::to_string(session).map_err(OpenAIError::JSONDeserialize)?;
        let session_part = Part::text(session_json)
            .mime_str("application/json")
            .expect("\"application/json\" is always a valid mime type");
        form = form.part("session", session_part);
    }

    Ok(form)
}

pub struct RealtimeCalls<'a> {
    openai: &'a OpenAI,
}

impl<'a> RealtimeCalls<'a> {
    pub(crate) fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }

    /// Creates a new Realtime call over WebRTC and returns the SDP answer needed to complete
    /// the peer connection.
    pub async fn create(&self, req: &realtime::CreateCallRequest) -> OpenAIResponse<String> {
        let form = create_call_form(req)?;
        self.openai
            .post_form_with_text_response("/realtime/calls", form)
            .await
    }
}

pub struct RealtimeCall<'a> {
    openai: &'a OpenAI,
    call_id: String,
}

impl<'a> RealtimeCall<'a> {
    pub(crate) fn new(openai: &'a OpenAI, call_id: String) -> Self {
        Self { openai, call_id }
    }

    /// Accepts an incoming SIP call and configures the Realtime session that will handle it.
    /// Takes the same session-config shape as `session.update`'s `session` field -- confirmed
    /// via source these are field-for-field identical.
    pub async fn accept(&self, session: &realtime::RealtimeSessionConfig) -> OpenAIResponse<()> {
        self.openai
            .post_no_content(&format!("/realtime/calls/{}/accept", self.call_id), session)
            .await
    }

    /// Ends an active Realtime call, whether it was initiated over SIP or WebRTC.
    pub async fn hangup(&self) -> OpenAIResponse<()> {
        self.openai
            .post_no_content(&format!("/realtime/calls/{}/hangup", self.call_id), &())
            .await
    }

    /// Transfers an active SIP call to a new destination using the SIP REFER verb.
    pub async fn refer(&self, req: &realtime::ReferCallRequest) -> OpenAIResponse<()> {
        self.openai
            .post_no_content(&format!("/realtime/calls/{}/refer", self.call_id), req)
            .await
    }

    /// Declines an incoming SIP call by returning a SIP status code to the caller.
    pub async fn reject(&self, req: &realtime::RejectCallRequest) -> OpenAIResponse<()> {
        self.openai
            .post_no_content(&format!("/realtime/calls/{}/reject", self.call_id), req)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interfaces::realtime::{
        CreateCallRequest, RealtimeSessionConfig, ReferCallRequest, RejectCallRequest,
    };
    use wiremock::matchers::{body_json, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn create_sends_multipart_and_returns_sdp_text() {
        let mock_server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/realtime/calls"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string("v=0\r\no=- 0 0 IN IP4 127.0.0.1\r\n")
                    .insert_header("content-type", "application/sdp"),
            )
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = CreateCallRequest::builder()
            .sdp("v=0\r\no=- 0 0 IN IP4 0.0.0.0\r\n")
            .session(RealtimeSessionConfig::builder().r#type("realtime").build())
            .build();

        let sdp = client.realtime().calls().create(&req).await.unwrap();
        assert!(sdp.starts_with("v=0"));
    }

    #[tokio::test]
    async fn accept_sends_session_config_and_handles_no_content() {
        let mock_server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/realtime/calls/call_1/accept"))
            .and(body_json(
                serde_json::json!({"type": "realtime", "model": "gpt-realtime-2.1"}),
            ))
            .respond_with(ResponseTemplate::new(204))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let session = RealtimeSessionConfig::builder()
            .r#type("realtime")
            .model("gpt-realtime-2.1")
            .build();

        client
            .realtime()
            .call("call_1")
            .accept(&session)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn hangup_sends_no_body_and_handles_no_content() {
        let mock_server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/realtime/calls/call_1/hangup"))
            .and(body_json(serde_json::json!(null)))
            .respond_with(ResponseTemplate::new(204))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        client.realtime().call("call_1").hangup().await.unwrap();
    }

    #[tokio::test]
    async fn refer_sends_target_uri() {
        let mock_server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/realtime/calls/call_1/refer"))
            .and(body_json(
                serde_json::json!({"target_uri": "tel:+14155550123"}),
            ))
            .respond_with(ResponseTemplate::new(204))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = ReferCallRequest::builder()
            .target_uri("tel:+14155550123")
            .build();
        client.realtime().call("call_1").refer(&req).await.unwrap();
    }

    #[tokio::test]
    async fn reject_sends_status_code() {
        let mock_server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/realtime/calls/call_1/reject"))
            .and(body_json(serde_json::json!({"status_code": 486})))
            .respond_with(ResponseTemplate::new(204))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = RejectCallRequest::builder().status_code(486).build();
        client.realtime().call("call_1").reject(&req).await.unwrap();
    }

    #[tokio::test]
    async fn no_content_action_maps_api_error_on_failure() {
        let mock_server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/realtime/calls/call_1/hangup"))
            .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
                "error": {
                    "message": "No such call",
                    "type": "invalid_request_error",
                    "code": "call_not_found"
                }
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let err = client.realtime().call("call_1").hangup().await.unwrap_err();
        assert!(matches!(
            err,
            crate::shared::response_wrapper::OpenAIError::ApiError(_)
        ));
    }
}
