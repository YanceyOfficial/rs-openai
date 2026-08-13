# ChatKit Sessions + Minimal Threads Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement ChatKit's Sessions (create/cancel, full fidelity) and Threads (list/retrieve/delete, minimal confirmed fields) on top of Phase 2a's zero-method scaffold, plus the `client.rs` infrastructure ChatKit needs (a required `OpenAI-Beta: chatkit_beta=v1` header no other module uses).

**Architecture:** New file `rs_openai/src/interfaces/chatkit.rs` holds all request/response types for both Sessions and Threads (one flat file, matching the `interfaces::vector_stores` precedent even though `apis::chatkit` is split into `sessions.rs`/`threads.rs`). Real methods get added to the existing `apis::chatkit::sessions`/`apis::chatkit::threads` scaffold structs. `client.rs` gains three new `pub(crate)` methods (`chatkit_get`/`chatkit_post`/`chatkit_delete`) that mirror the existing `get`/`post`/`delete` but inject the ChatKit-specific beta header via a small private helper — this is additive and touches no other module's call sites.

**Tech Stack:** Rust, `serde`, `bon::Builder`, `wiremock` (including, for the first time in this crate, a header-presence assertion via `wiremock::matchers::header`).

## Global Constraints

- Spec: `docs/superpowers/specs/2026-08-14-chatkit-design.md`. Scope confirmed via `AskUserQuestion`: Sessions at full fidelity; Threads' collection ops (list/retrieve/delete) with ONLY the confirmed `ChatKitThread` fields (`id`/`object`/`created_at`/`status`/`title`); Thread Items sub-resource entirely deferred (`apis::chatkit::threads::ChatKitThreadItems` stays untouched, still a zero-method scaffold).
- Every ChatKit HTTP call MUST go through the new `chatkit_get`/`chatkit_post`/`chatkit_delete` client methods, never the plain `get`/`post`/`delete` — those don't send the required `OpenAI-Beta: chatkit_beta=v1` header and a real API call would fail without it.
- `ChatKitSession.status` and `ChatKitThread.status` both stay plain `String`, not closed enums — deliberate, for uniformity with every other status field in this crate (even where, as here, a closed value set is confirmed).
- `expires_after` on `CreateChatKitSessionRequest` reuses `crate::shared::types::ExpiresAfter` directly — do NOT define a new ChatKit-specific expires-after type. This is a genuine structural match (unlike Vector Stores' `expires_after`, which was NOT reused for good reason — different anchor/unit).
- `ChatKitWorkflowParam.state_variables` is `Option<HashMap<String, serde_json::Value>>`, NOT `HashMap<String, String>` — the docs describe "primitive-typed" values, not confirmed as string-only, so don't narrow it to the crate's usual all-string metadata convention.
- `ListChatKitThreadsRequest` gets exactly `after`/`before`/`limit` — do NOT add `order` (not confirmed for this specific endpoint) or guess at the "2 additional params" one source mentioned but didn't name.
- Two naming collisions apply here, same established resolution as `Response`/`VectorStore`/`Conversation`: `interfaces::chatkit::ChatKitSession` (response type) vs `apis::chatkit::sessions::ChatKitSession<'a>` (ID-scoped accessor), and `interfaces::chatkit::ChatKitThread` (response type) vs `apis::chatkit::threads::ChatKitThread<'a>` (ID-scoped accessor). Resolve via ordinary qualified paths (`chatkit::ChatKitSession`, `chatkit::ChatKitThread`) inside `apis::chatkit`'s own method signatures — do not rename either side.
- `#[allow(dead_code)]` on `ChatKitSessions`/`ChatKitSession` (sessions.rs) and `ChatKitThreads` (threads.rs) becomes stale once real methods are added that read their fields — remove it in the same task that adds the method. `ChatKitThreadItems` (threads.rs) keeps its `#[allow(dead_code)]` — it stays an unimplemented scaffold this slice.
- Stage commits with **explicit file paths** (`git add path/to/file.rs`), never `git add -A`.
- Run `cargo fmt` and `cargo clippy -p rs_openai --all-targets -- -D warnings` before every commit; both must be clean.
- Environment: this machine's shell routes HTTP through a local proxy that breaks `wiremock` tests. Prefix every `cargo test`/`build`/`clippy` invocation with `NO_PROXY=127.0.0.1,localhost`. This is a local-shell-only concern — never write it into crate code, `Cargo.toml`, or CI config.

---

### Task 1: `client.rs` ChatKit request infrastructure + Sessions (create/cancel)

**Files:**
- Modify: `rs_openai/src/client.rs` (add `chatkit_request`/`chatkit_get`/`chatkit_post`/`chatkit_delete`, immediately after the existing `delete` method at line 291-298, before `stream_sse` at line 300)
- Modify: `rs_openai/src/interfaces/mod.rs` (add `pub mod chatkit;`)
- Create: `rs_openai/src/interfaces/chatkit.rs` (Sessions types only this task — Threads types come in Task 2)
- Modify: `rs_openai/src/apis/chatkit/mod.rs` (update doc comments, no structural change)
- Modify: `rs_openai/src/apis/chatkit/sessions.rs` (add `create`/`cancel` methods, remove stale `#[allow(dead_code)]`, update doc comments)
- Test: `rs_openai/src/apis/chatkit/sessions.rs` (add a `#[cfg(test)] mod tests` block — none exists yet)

**Interfaces:**
- Consumes: existing `OpenAI` struct and its private `openai_request`/`headers` methods in `client.rs` (unchanged); existing `crate::shared::types::ExpiresAfter`/`ExpiresAfterAnchor` (reused directly, not modified); existing `apis::chatkit::sessions::{ChatKitSessions, ChatKitSession}` struct shapes from Phase 2a (unchanged field layout, `openai: &'a OpenAI` on both, `id: String` on `ChatKitSession`).
- Produces: `OpenAI::chatkit_get`/`chatkit_post`/`chatkit_delete` (same generic signatures as `get`/`post`/`delete`) — Task 2 reuses `chatkit_get`/`chatkit_delete` directly. `interfaces::chatkit::{ChatKitWorkflowTracing, ChatKitWorkflowParam, ChatKitAutomaticThreadTitling, ChatKitFileUploadConfig, ChatKitHistoryConfig, ChatKitConfiguration, ChatKitRateLimits, CreateChatKitSessionRequest, ChatKitSession}` — Task 2 does not depend on these (Threads types are independent), but the final cross-check task does.

- [ ] **Step 1: Write the failing test — `create` sends the beta header and serializes the full request**

First, read the current contents of `rs_openai/src/apis/chatkit/sessions.rs` (shown in full in this task's brief) so you preserve the existing `ChatKitSessions`/`ChatKitSession` struct definitions and only add to them.

Add a test module to `rs_openai/src/apis/chatkit/sessions.rs`:

```rust
#[cfg(test)]
mod tests {
    use crate::client::OpenAI;
    use crate::interfaces::chatkit::{
        ChatKitConfiguration, ChatKitWorkflowParam, ChatKitWorkflowTracing,
        CreateChatKitSessionRequest,
    };
    use crate::shared::types::{ExpiresAfter, ExpiresAfterAnchor};
    use std::collections::HashMap;
    use wiremock::matchers::{body_json, header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn create_sends_beta_header_and_serializes_full_request() {
        let mock_server = MockServer::start().await;

        let mut state_variables = HashMap::new();
        state_variables.insert("tier".to_string(), serde_json::json!("gold"));

        Mock::given(method("POST"))
            .and(path("/chatkit/sessions"))
            .and(header("OpenAI-Beta", "chatkit_beta=v1"))
            .and(body_json(serde_json::json!({
                "workflow": {
                    "id": "wf_123",
                    "state_variables": {"tier": "gold"},
                    "tracing": {"enabled": false},
                    "version": "2"
                },
                "user": "user_abc",
                "chatkit_configuration": {
                    "automatic_thread_titling": {"enabled": true}
                },
                "expires_after": {"anchor": "created_at", "seconds": 3600},
                "rate_limits": {"max_requests_per_1_minute": 20}
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "cksess_1",
                "object": "chatkit.session",
                "client_secret": "cksec_abc",
                "expires_at": 1700003600_i64,
                "status": "active",
                "user": "user_abc",
                "workflow": {"id": "wf_123"},
                "chatkit_configuration": {},
                "rate_limits": {"max_requests_per_1_minute": 20},
                "max_requests_per_1_minute": 20
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = CreateChatKitSessionRequest::builder()
            .workflow(
                ChatKitWorkflowParam::builder()
                    .id("wf_123")
                    .state_variables(state_variables)
                    .tracing(ChatKitWorkflowTracing::builder().enabled(false).build())
                    .version("2")
                    .build(),
            )
            .user("user_abc")
            .chatkit_configuration(
                ChatKitConfiguration::builder()
                    .automatic_thread_titling(
                        crate::interfaces::chatkit::ChatKitAutomaticThreadTitling::builder()
                            .enabled(true)
                            .build(),
                    )
                    .build(),
            )
            .expires_after(
                ExpiresAfter::builder()
                    .anchor(ExpiresAfterAnchor::CreatedAt)
                    .seconds(3600)
                    .build(),
            )
            .rate_limits(
                crate::interfaces::chatkit::ChatKitRateLimits::builder()
                    .max_requests_per_1_minute(20)
                    .build(),
            )
            .build();

        let res = client.chatkit().sessions().create(&req).await.unwrap();
        assert_eq!(res.id, "cksess_1");
        assert_eq!(res.client_secret, "cksec_abc");
        assert_eq!(res.status, "active");
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai create_sends_beta_header_and_serializes_full_request 2>&1 | tail -40`
Expected: FAIL to compile — `crate::interfaces::chatkit` module doesn't exist yet, `client.chatkit().sessions().create(...)` doesn't exist.

- [ ] **Step 3: Create `rs_openai/src/interfaces/chatkit.rs` with the Sessions types**

```rust
//! ChatKit (Beta) sessions and threads — see
//! docs/superpowers/specs/2026-08-14-chatkit-design.md.

use bon::Builder;
use serde::{Deserialize, Serialize};

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct ChatKitWorkflowTracing {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct ChatKitWorkflowParam {
    #[builder(into)]
    pub id: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub state_variables: Option<std::collections::HashMap<String, serde_json::Value>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub tracing: Option<ChatKitWorkflowTracing>,

    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct ChatKitAutomaticThreadTitling {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct ChatKitFileUploadConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_file_size: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_files: Option<u32>,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct ChatKitHistoryConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub recent_threads: Option<u32>,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct ChatKitConfiguration {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub automatic_thread_titling: Option<ChatKitAutomaticThreadTitling>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_upload: Option<ChatKitFileUploadConfig>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub history: Option<ChatKitHistoryConfig>,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct ChatKitRateLimits {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_requests_per_1_minute: Option<u32>,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct CreateChatKitSessionRequest {
    pub workflow: ChatKitWorkflowParam,

    #[builder(into)]
    pub user: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub chatkit_configuration: Option<ChatKitConfiguration>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_after: Option<crate::shared::types::ExpiresAfter>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_limits: Option<ChatKitRateLimits>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ChatKitSession {
    pub id: String,
    pub object: String,
    pub client_secret: String,
    pub expires_at: i64,
    pub status: String,
    pub user: String,
    pub workflow: ChatKitWorkflowParam,
    pub chatkit_configuration: ChatKitConfiguration,
    pub rate_limits: ChatKitRateLimits,
    pub max_requests_per_1_minute: u32,
}
```

Note: the mock response body in Step 1's test omits several optional fields from nested objects (e.g. `workflow` only returns `{"id": "wf_123"}`, `chatkit_configuration` returns `{}`) — this only works because every field in `ChatKitWorkflowParam`/`ChatKitConfiguration`/etc. besides `id` is `Option` with `#[serde(skip_serializing_if = "Option::is_none")]` on the way out AND plain `Option<T>` (defaults to `None` when the JSON key is absent) on the way in. Do not make any of these fields non-`Option` or the mock response will fail to deserialize.

- [ ] **Step 4: Register the new module**

In `rs_openai/src/interfaces/mod.rs`, add (alphabetically, after `pub mod chat;` and before `pub mod completions;`):

```rust
pub mod chatkit;
```

- [ ] **Step 5: Add the `chatkit_*` methods to `client.rs`**

In `rs_openai/src/client.rs`, insert immediately after the `delete` method (ends at line 298, right before `async fn stream_sse` at line 300):

```rust
    /// Same as `openai_request`, but adds the `OpenAI-Beta: chatkit_beta=v1` header every
    /// ChatKit endpoint requires in addition to the standard auth/org/project headers.
    fn chatkit_request(
        &self,
        method: Method,
        route: &str,
    ) -> Result<reqwest_middleware::RequestBuilder, OpenAIError> {
        Ok(self
            .openai_request(method, route)?
            .header("OpenAI-Beta", "chatkit_beta=v1"))
    }

    pub(crate) async fn chatkit_get<T, F>(&self, route: &str, query: &F) -> OpenAIResponse<T>
    where
        T: DeserializeOwned + Debug,
        F: Serialize,
    {
        let request = self.chatkit_request(Method::GET, route)?.query(query);
        Self::resolve_response(request).await
    }

    pub(crate) async fn chatkit_post<T, F>(&self, route: &str, json: &F) -> OpenAIResponse<T>
    where
        T: DeserializeOwned + Debug,
        F: Serialize,
    {
        let request = self.chatkit_request(Method::POST, route)?.json(json);
        Self::resolve_response(request).await
    }

    pub(crate) async fn chatkit_delete<T, F>(&self, route: &str, json: &F) -> OpenAIResponse<T>
    where
        T: DeserializeOwned + Debug,
        F: Serialize,
    {
        let request = self.chatkit_request(Method::DELETE, route)?.json(json);
        Self::resolve_response(request).await
    }
```

- [ ] **Step 6: Implement `ChatKitSessions::create` and `ChatKitSession::cancel`, remove stale `#[allow(dead_code)]`**

Replace the full contents of `rs_openai/src/apis/chatkit/sessions.rs` with:

```rust
//! ChatKit sessions: `POST /chatkit/sessions` create,
//! `POST /chatkit/sessions/{session_id}/cancel` cancel — see
//! docs/superpowers/specs/2026-08-14-chatkit-design.md.

use crate::client::OpenAI;
use crate::interfaces::chatkit;
use crate::shared::response_wrapper::OpenAIResponse;

pub struct ChatKitSessions<'a> {
    openai: &'a OpenAI,
}

impl<'a> ChatKitSessions<'a> {
    pub(crate) fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }

    /// Creates a ChatKit session, returning a `client_secret` to hand off to the ChatKit
    /// client library.
    pub async fn create(
        &self,
        req: &chatkit::CreateChatKitSessionRequest,
    ) -> OpenAIResponse<chatkit::ChatKitSession> {
        self.openai.chatkit_post("/chatkit/sessions", req).await
    }
}

pub struct ChatKitSession<'a> {
    openai: &'a OpenAI,
    id: String,
}

impl<'a> ChatKitSession<'a> {
    pub(crate) fn new(openai: &'a OpenAI, id: String) -> Self {
        Self { openai, id }
    }

    /// Cancels this ChatKit session.
    pub async fn cancel(&self) -> OpenAIResponse<chatkit::ChatKitSession> {
        self.openai
            .chatkit_post(&format!("/chatkit/sessions/{}/cancel", self.id), &())
            .await
    }
}

#[cfg(test)]
mod tests {
    use crate::client::OpenAI;
    use crate::interfaces::chatkit::{
        ChatKitConfiguration, ChatKitWorkflowParam, ChatKitWorkflowTracing,
        CreateChatKitSessionRequest,
    };
    use crate::shared::types::{ExpiresAfter, ExpiresAfterAnchor};
    use std::collections::HashMap;
    use wiremock::matchers::{body_json, header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn create_sends_beta_header_and_serializes_full_request() {
        let mock_server = MockServer::start().await;

        let mut state_variables = HashMap::new();
        state_variables.insert("tier".to_string(), serde_json::json!("gold"));

        Mock::given(method("POST"))
            .and(path("/chatkit/sessions"))
            .and(header("OpenAI-Beta", "chatkit_beta=v1"))
            .and(body_json(serde_json::json!({
                "workflow": {
                    "id": "wf_123",
                    "state_variables": {"tier": "gold"},
                    "tracing": {"enabled": false},
                    "version": "2"
                },
                "user": "user_abc",
                "chatkit_configuration": {
                    "automatic_thread_titling": {"enabled": true}
                },
                "expires_after": {"anchor": "created_at", "seconds": 3600},
                "rate_limits": {"max_requests_per_1_minute": 20}
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "cksess_1",
                "object": "chatkit.session",
                "client_secret": "cksec_abc",
                "expires_at": 1700003600_i64,
                "status": "active",
                "user": "user_abc",
                "workflow": {"id": "wf_123"},
                "chatkit_configuration": {},
                "rate_limits": {"max_requests_per_1_minute": 20},
                "max_requests_per_1_minute": 20
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = CreateChatKitSessionRequest::builder()
            .workflow(
                ChatKitWorkflowParam::builder()
                    .id("wf_123")
                    .state_variables(state_variables)
                    .tracing(ChatKitWorkflowTracing::builder().enabled(false).build())
                    .version("2")
                    .build(),
            )
            .user("user_abc")
            .chatkit_configuration(
                ChatKitConfiguration::builder()
                    .automatic_thread_titling(
                        crate::interfaces::chatkit::ChatKitAutomaticThreadTitling::builder()
                            .enabled(true)
                            .build(),
                    )
                    .build(),
            )
            .expires_after(
                ExpiresAfter::builder()
                    .anchor(ExpiresAfterAnchor::CreatedAt)
                    .seconds(3600)
                    .build(),
            )
            .rate_limits(
                crate::interfaces::chatkit::ChatKitRateLimits::builder()
                    .max_requests_per_1_minute(20)
                    .build(),
            )
            .build();

        let res = client.chatkit().sessions().create(&req).await.unwrap();
        assert_eq!(res.id, "cksess_1");
        assert_eq!(res.client_secret, "cksec_abc");
        assert_eq!(res.status, "active");
    }
}
```

Also update `rs_openai/src/apis/chatkit/mod.rs`'s doc comments — replace the module doc comment and the `sessions()`/`session()` method doc comments (leave `threads()`/`thread()` untouched, Task 2 handles those):

```rust
//! ChatKit (Beta): embeddable chat UI sessions and their threads. Sessions implemented;
//! Threads implemented at minimal fidelity (list/retrieve/delete only, Items sub-resource
//! still not implemented) — see docs/superpowers/specs/2026-08-14-chatkit-design.md.

pub mod sessions;
pub mod threads;

use crate::client::OpenAI;
pub use crate::interfaces::chatkit::*;

pub struct ChatKit<'a> {
    openai: &'a OpenAI,
}

impl<'a> ChatKit<'a> {
    pub(crate) fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }

    /// Scoped accessor for ChatKit sessions.
    pub fn sessions(&self) -> sessions::ChatKitSessions<'a> {
        sessions::ChatKitSessions::new(self.openai)
    }

    /// Scoped accessor for a specific ChatKit session.
    pub fn session(&self, id: impl Into<String>) -> sessions::ChatKitSession<'a> {
        sessions::ChatKitSession::new(self.openai, id.into())
    }

    /// Scoped accessor for ChatKit threads. Not yet implemented.
    pub fn threads(&self) -> threads::ChatKitThreads<'a> {
        threads::ChatKitThreads::new(self.openai)
    }

    /// Scoped accessor for a specific ChatKit thread. Not yet implemented.
    pub fn thread(&self, id: impl Into<String>) -> threads::ChatKitThread<'a> {
        threads::ChatKitThread::new(self.openai, id.into())
    }
}
```

- [ ] **Step 7: Run test to verify it passes**

Run: `NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai create_sends_beta_header_and_serializes_full_request 2>&1 | tail -30`
Expected: PASS.

- [ ] **Step 8: Write the failing test for `cancel`**

Add to the same test module in `rs_openai/src/apis/chatkit/sessions.rs`:

```rust
    #[tokio::test]
    async fn cancel_deserializes_cancelled_session() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/chatkit/sessions/cksess_2/cancel"))
            .and(header("OpenAI-Beta", "chatkit_beta=v1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "cksess_2",
                "object": "chatkit.session",
                "client_secret": "cksec_xyz",
                "expires_at": 1700003600_i64,
                "status": "cancelled",
                "user": "user_abc",
                "workflow": {"id": "wf_123"},
                "chatkit_configuration": {},
                "rate_limits": {"max_requests_per_1_minute": 10},
                "max_requests_per_1_minute": 10
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let res = client
            .chatkit()
            .session("cksess_2")
            .cancel()
            .await
            .unwrap();
        assert_eq!(res.status, "cancelled");
    }
```

- [ ] **Step 9: Run test to verify it passes**

Run: `NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai cancel_deserializes_cancelled_session 2>&1 | tail -20`
Expected: PASS.

- [ ] **Step 10: Write a minimal-fields test proving optional fields are correctly omitted**

Add to the same test module:

```rust
    #[tokio::test]
    async fn create_with_only_required_fields_omits_optional_ones() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/chatkit/sessions"))
            .and(body_json(serde_json::json!({
                "workflow": {"id": "wf_456"},
                "user": "user_def"
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "cksess_3",
                "object": "chatkit.session",
                "client_secret": "cksec_def",
                "expires_at": 1700003600_i64,
                "status": "active",
                "user": "user_def",
                "workflow": {"id": "wf_456"},
                "chatkit_configuration": {},
                "rate_limits": {},
                "max_requests_per_1_minute": 10
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = CreateChatKitSessionRequest::builder()
            .workflow(ChatKitWorkflowParam::builder().id("wf_456").build())
            .user("user_def")
            .build();

        let res = client.chatkit().sessions().create(&req).await.unwrap();
        assert_eq!(res.id, "cksess_3");
    }
```

- [ ] **Step 11: Run test to verify it passes**

Run: `NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai create_with_only_required_fields_omits_optional_ones 2>&1 | tail -20`
Expected: PASS.

- [ ] **Step 12: Format, lint, full test run**

Run:
```bash
cargo fmt
NO_PROXY=127.0.0.1,localhost cargo clippy -p rs_openai --all-targets -- -D warnings
NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai 2>&1 | tail -10
```
Expected: clippy clean, all tests pass (152 + 3 new = 155).

- [ ] **Step 13: Commit**

```bash
git add rs_openai/src/client.rs rs_openai/src/interfaces/mod.rs rs_openai/src/interfaces/chatkit.rs rs_openai/src/apis/chatkit/mod.rs rs_openai/src/apis/chatkit/sessions.rs
git commit -m "feat(chatkit): add sessions create/cancel and chatkit_* client infrastructure"
```

---

### Task 2: Threads (list/retrieve/delete, minimal fidelity)

**Files:**
- Modify: `rs_openai/src/interfaces/chatkit.rs` (add Threads types to the same file, after the Sessions types)
- Modify: `rs_openai/src/apis/chatkit/mod.rs` (update `threads()`/`thread()` doc comments)
- Modify: `rs_openai/src/apis/chatkit/threads.rs` (add `list`/`retrieve`/`delete` methods, remove stale `#[allow(dead_code)]` from `ChatKitThreads` only — `ChatKitThreadItems` keeps its `#[allow(dead_code)]`, still unimplemented)
- Test: `rs_openai/src/apis/chatkit/threads.rs` (add a `#[cfg(test)] mod tests` block — none exists yet)

**Interfaces:**
- Consumes: `OpenAI::chatkit_get`/`chatkit_delete` from Task 1 (already implemented and tested — do not re-verify the beta header here, Task 1's tests already cover that); existing `apis::chatkit::threads::{ChatKitThreads, ChatKitThread, ChatKitThreadItems}` struct shapes from Phase 2a (unchanged).
- Produces: `interfaces::chatkit::{ListChatKitThreadsRequest, ChatKitThread, ListChatKitThreadsResponse, DeleteChatKitThreadResponse}` — no later task in this plan depends on these (this is effectively the last feature task), but the final cross-check does.

- [ ] **Step 1: Write the failing test — `list` serializes query params and deserializes a paginated response**

Add a test module to `rs_openai/src/apis/chatkit/threads.rs`:

```rust
#[cfg(test)]
mod tests {
    use crate::client::OpenAI;
    use crate::interfaces::chatkit::ListChatKitThreadsRequest;
    use wiremock::matchers::{header, method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn list_serializes_pagination_params_and_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/chatkit/threads"))
            .and(header("OpenAI-Beta", "chatkit_beta=v1"))
            .and(query_param("limit", "10"))
            .and(query_param("after", "thread_abc"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "object": "list",
                "data": [
                    {
                        "id": "thread_1",
                        "object": "chatkit.thread",
                        "created_at": 1700000000_i64,
                        "status": "active",
                        "title": "Support chat"
                    },
                    {
                        "id": "thread_2",
                        "object": "chatkit.thread",
                        "created_at": 1700000100_i64,
                        "status": "active",
                        "title": null
                    }
                ],
                "first_id": "thread_1",
                "last_id": "thread_2",
                "has_more": false
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = ListChatKitThreadsRequest::builder()
            .after("thread_abc")
            .limit(10)
            .build();

        let res = client.chatkit().threads().list(&req).await.unwrap();
        assert_eq!(res.data.len(), 2);
        assert_eq!(res.data[0].id, "thread_1");
        assert_eq!(res.data[0].title.as_deref(), Some("Support chat"));
        assert!(res.data[1].title.is_none());
        assert!(!res.has_more);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai list_serializes_pagination_params_and_deserializes_response 2>&1 | tail -30`
Expected: FAIL to compile — `ListChatKitThreadsRequest` doesn't exist, `client.chatkit().threads().list(...)` doesn't exist.

- [ ] **Step 3: Add the Threads types to `rs_openai/src/interfaces/chatkit.rs`**

Append to the end of `rs_openai/src/interfaces/chatkit.rs` (after the `ChatKitSession` struct from Task 1):

```rust
#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct ListChatKitThreadsRequest {
    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after: Option<String>,

    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub before: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ChatKitThread {
    pub id: String,
    pub object: String,
    pub created_at: i64,
    pub status: String,
    pub title: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ListChatKitThreadsResponse {
    pub object: String,
    pub data: Vec<ChatKitThread>,
    pub first_id: Option<String>,
    pub last_id: Option<String>,
    pub has_more: bool,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct DeleteChatKitThreadResponse {
    pub id: String,
    pub object: String,
    pub deleted: bool,
}
```

- [ ] **Step 4: Implement `ChatKitThreads::list`, remove stale `#[allow(dead_code)]`**

Replace the full contents of `rs_openai/src/apis/chatkit/threads.rs` with:

```rust
//! ChatKit threads: `GET /chatkit/threads` list, `GET .../{thread_id}` retrieve,
//! `DELETE .../{thread_id}` delete. `GET .../{thread_id}/items` list items still not
//! implemented — see docs/superpowers/specs/2026-08-14-chatkit-design.md.

use crate::client::OpenAI;
use crate::interfaces::chatkit;
use crate::shared::response_wrapper::OpenAIResponse;

pub struct ChatKitThreads<'a> {
    openai: &'a OpenAI,
}

impl<'a> ChatKitThreads<'a> {
    pub(crate) fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }

    /// Lists ChatKit threads.
    pub async fn list(
        &self,
        req: &chatkit::ListChatKitThreadsRequest,
    ) -> OpenAIResponse<chatkit::ListChatKitThreadsResponse> {
        self.openai.chatkit_get("/chatkit/threads", req).await
    }
}

pub struct ChatKitThread<'a> {
    openai: &'a OpenAI,
    id: String,
}

impl<'a> ChatKitThread<'a> {
    pub(crate) fn new(openai: &'a OpenAI, id: String) -> Self {
        Self { openai, id }
    }

    /// Retrieves a ChatKit thread by ID.
    pub async fn retrieve(&self) -> OpenAIResponse<chatkit::ChatKitThread> {
        self.openai
            .chatkit_get(&format!("/chatkit/threads/{}", self.id), &())
            .await
    }

    /// Deletes a ChatKit thread.
    pub async fn delete(&self) -> OpenAIResponse<chatkit::DeleteChatKitThreadResponse> {
        self.openai
            .chatkit_delete(&format!("/chatkit/threads/{}", self.id), &())
            .await
    }

    /// Scoped accessor for this thread's items. Not yet implemented.
    pub fn items(&self) -> ChatKitThreadItems<'a> {
        ChatKitThreadItems::new(self.openai, self.id.clone())
    }
}

#[allow(dead_code)]
pub struct ChatKitThreadItems<'a> {
    openai: &'a OpenAI,
    thread_id: String,
}

impl<'a> ChatKitThreadItems<'a> {
    pub(crate) fn new(openai: &'a OpenAI, thread_id: String) -> Self {
        Self { openai, thread_id }
    }
}

#[cfg(test)]
mod tests {
    use crate::client::OpenAI;
    use crate::interfaces::chatkit::ListChatKitThreadsRequest;
    use wiremock::matchers::{header, method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn list_serializes_pagination_params_and_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/chatkit/threads"))
            .and(header("OpenAI-Beta", "chatkit_beta=v1"))
            .and(query_param("limit", "10"))
            .and(query_param("after", "thread_abc"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "object": "list",
                "data": [
                    {
                        "id": "thread_1",
                        "object": "chatkit.thread",
                        "created_at": 1700000000_i64,
                        "status": "active",
                        "title": "Support chat"
                    },
                    {
                        "id": "thread_2",
                        "object": "chatkit.thread",
                        "created_at": 1700000100_i64,
                        "status": "active",
                        "title": null
                    }
                ],
                "first_id": "thread_1",
                "last_id": "thread_2",
                "has_more": false
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = ListChatKitThreadsRequest::builder()
            .after("thread_abc")
            .limit(10)
            .build();

        let res = client.chatkit().threads().list(&req).await.unwrap();
        assert_eq!(res.data.len(), 2);
        assert_eq!(res.data[0].id, "thread_1");
        assert_eq!(res.data[0].title.as_deref(), Some("Support chat"));
        assert!(res.data[1].title.is_none());
        assert!(!res.has_more);
    }
}
```

Also update `rs_openai/src/apis/chatkit/mod.rs`'s `threads()`/`thread()` doc comments (leave everything else from Task 1 unchanged):

```rust
    /// Scoped accessor for ChatKit threads.
    pub fn threads(&self) -> threads::ChatKitThreads<'a> {
        threads::ChatKitThreads::new(self.openai)
    }

    /// Scoped accessor for a specific ChatKit thread.
    pub fn thread(&self, id: impl Into<String>) -> threads::ChatKitThread<'a> {
        threads::ChatKitThread::new(self.openai, id.into())
    }
```

- [ ] **Step 5: Run test to verify it passes**

Run: `NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai list_serializes_pagination_params_and_deserializes_response 2>&1 | tail -20`
Expected: PASS.

- [ ] **Step 6: Write the failing test for `retrieve`**

Add to the same test module in `rs_openai/src/apis/chatkit/threads.rs`:

```rust
    #[tokio::test]
    async fn retrieve_deserializes_thread() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/chatkit/threads/thread_9"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "thread_9",
                "object": "chatkit.thread",
                "created_at": 1700000000_i64,
                "status": "active",
                "title": "Billing question"
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let res = client
            .chatkit()
            .thread("thread_9")
            .retrieve()
            .await
            .unwrap();
        assert_eq!(res.id, "thread_9");
        assert_eq!(res.title.as_deref(), Some("Billing question"));
    }
```

- [ ] **Step 7: Run test to verify it passes**

Run: `NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai retrieve_deserializes_thread 2>&1 | tail -20`
Expected: PASS.

- [ ] **Step 8: Write the failing test for `delete`**

Add to the same test module:

```rust
    #[tokio::test]
    async fn delete_deserializes_confirmation() {
        let mock_server = MockServer::start().await;

        Mock::given(method("DELETE"))
            .and(path("/chatkit/threads/thread_9"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "thread_9",
                "object": "chatkit.thread.deleted",
                "deleted": true
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let res = client
            .chatkit()
            .thread("thread_9")
            .delete()
            .await
            .unwrap();
        assert!(res.deleted);
    }
```

- [ ] **Step 9: Run test to verify it passes**

Run: `NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai delete_deserializes_confirmation 2>&1 | tail -20`
Expected: PASS.

- [ ] **Step 10: Format, lint, full test run**

Run:
```bash
cargo fmt
NO_PROXY=127.0.0.1,localhost cargo clippy -p rs_openai --all-targets -- -D warnings
NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai 2>&1 | tail -10
```
Expected: clippy clean, all tests pass (155 + 3 new = 158).

- [ ] **Step 11: Commit**

```bash
git add rs_openai/src/interfaces/chatkit.rs rs_openai/src/apis/chatkit/mod.rs rs_openai/src/apis/chatkit/threads.rs
git commit -m "feat(chatkit): add threads list/retrieve/delete at minimal fidelity"
```

---

### Task 3: Final cross-check

**Files:**
- No new code changes expected — this task verifies the whole slice, and only touches files if it finds a real gap.

**Interfaces:**
- Consumes: everything from Task 1 and Task 2.
- Produces: nothing new — this is a verification-only task, same role as the final lightweight task in every prior Phase 2b slice's plan.

- [ ] **Step 1: Run the full test suite, clippy, and build**

Run:
```bash
cargo fmt --check
NO_PROXY=127.0.0.1,localhost cargo clippy -p rs_openai --all-targets -- -D warnings
NO_PROXY=127.0.0.1,localhost cargo build -p rs_openai
NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai 2>&1 | tail -10
```
Expected: fmt clean (no diff), clippy clean, build clean, 158/158 tests passing.

- [ ] **Step 2: Manually verify scope and cleanup, reading the actual files (don't just trust the step count)**

- `rs_openai/src/apis/chatkit/sessions.rs`: `ChatKitSessions` and `ChatKitSession` structs have NO `#[allow(dead_code)]` (both have real methods now).
- `rs_openai/src/apis/chatkit/threads.rs`: `ChatKitThreads` and `ChatKitThread` structs have NO `#[allow(dead_code)]`; `ChatKitThreadItems` STILL HAS `#[allow(dead_code)]` (unimplemented, correct).
- `rs_openai/src/interfaces/chatkit.rs` defines exactly these 12 types: `ChatKitWorkflowTracing`, `ChatKitWorkflowParam`, `ChatKitAutomaticThreadTitling`, `ChatKitFileUploadConfig`, `ChatKitHistoryConfig`, `ChatKitConfiguration`, `ChatKitRateLimits`, `CreateChatKitSessionRequest`, `ChatKitSession`, `ListChatKitThreadsRequest`, `ChatKitThread`, `ListChatKitThreadsResponse`, `DeleteChatKitThreadResponse` — no Thread Items types (`ChatKitThreadUserMessageItem` etc.) were added; confirm none exist in the file (grep for "Item" in `interfaces/chatkit.rs` should return zero matches).
- `rs_openai/src/client.rs`: confirm `chatkit_get`/`chatkit_post`/`chatkit_delete`/`chatkit_request` exist and that no existing `get`/`post`/`delete`/`openai_request` call site anywhere else in the crate was modified (`git diff master -- rs_openai/src/client.rs` — or, if not on a branch with that history, re-read the whole file — should show only additions, no line inside the pre-existing `get`/`post`/`delete`/`openai_request` bodies changed).
- Confirm every ChatKit HTTP call (`create`, `cancel`, `list`, `retrieve`, `delete`) goes through a `chatkit_*` method, not the plain `get`/`post`/`delete`.
- `rs_openai/src/apis/chatkit/mod.rs` contains `pub use crate::interfaces::chatkit::*;` (matching the `apis::vector_stores::mod.rs` precedent for a split apis module) — confirm `rs_openai::chatkit::ChatKitSession` and `rs_openai::chatkit::ChatKitThread` are reachable from outside the crate (e.g. a doc-test or a quick `cargo doc` build check is sufficient, doesn't need a dedicated test).

If any of these checks fail, fix it directly and re-run Step 1.

- [ ] **Step 3: No commit needed for this task if Step 2 found nothing to fix** (if it did, commit the fix with an explicit-path `git add` and a message describing what was found).
