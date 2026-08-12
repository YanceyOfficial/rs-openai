# Conversations API: Core Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement the core Conversations API — `create`, `retrieve`, `update`, `delete` — turning Phase 2a's zero-method `Conversations`/`Conversation` scaffolds into working accessors.

**Architecture:** New file `rs_openai/src/interfaces/conversations.rs` holds every request/response type — small enough (4 types) to build in one task alongside `create()`. `rs_openai/src/apis/conversations.rs`'s already-scaffolded structs get real methods added. Two feature tasks (create; retrieve/update/delete) plus a lightweight final cross-check.

**Tech Stack:** Same as the rest of the crate (Rust edition 2024, `bon` builders, `serde`, `wiremock` tests) — no new dependencies.

## Global Constraints

- Spec: `docs/superpowers/specs/2026-08-12-conversations-design.md`.
- **Deliberately out of scope this round — do not add:** the `items` sub-resource (list/create/retrieve/delete individual conversation items) — not needed for this slice's purpose (supplying conversation IDs to Responses), and not researched in enough detail this session to model safely.
- `CreateConversationRequest.items` reuses `crate::interfaces::responses::ResponseInputItem` directly (already exists, from the Responses core slice) — do not define a new, duplicate item type.
- `UpdateConversationRequest` accepts exactly `metadata` — confirmed the real endpoint accepts nothing else. Do not add other fields.
- Naming collision, resolved and non-negotiable (same resolution already established for Responses and Vector Stores): `interfaces::conversations::Conversation` collides with `apis::conversations::Conversation<'a>` (the ID-scoped accessor, fixed by Phase 2a's scaffold). Inside `apis/conversations.rs`, refer to the interface type via the qualified path `conversations::Conversation` wherever both names are in scope — ordinary Rust namespace disambiguation, not a new pattern.
- `apis/mod.rs` and `client.rs` already have `conversations` fully wired (Phase 2a scaffold: `pub fn conversations(&self) -> conversations::Conversations<'_>` and `pub fn conversation(&self, id: impl Into<String>) -> conversations::Conversation<'_>` both already exist in `client.rs`) — do not touch either file in this plan.
- Every task's "done" check includes `cargo build --workspace --all-targets`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace`, and `cargo fmt --all -- --check`.
- **This machine needs `NO_PROXY=127.0.0.1,localhost` prefixed on every `cargo test` invocation**, or every `wiremock`-based HTTP test fails with a misleading `JSONDeserialize(Error("EOF while parsing a value", ...))` (a local proxy-client environment quirk, not a code defect — pure `#[test]`s with no HTTP are unaffected). E.g. `NO_PROXY=127.0.0.1,localhost cargo test --workspace`.
- Follow TDD: write the failing test first, confirm it fails for the expected reason, then implement.
- `interfaces/conversations.rs` does not exist yet — Task 1 creates it and wires `pub mod conversations;` into `interfaces/mod.rs`, positioned alphabetically between `pub mod completions;` and `pub mod embeddings;`.

---

### Task 1: Type foundation + `create`

**Files:**
- Create: `rs_openai/src/interfaces/conversations.rs`
- Modify: `rs_openai/src/interfaces/mod.rs`
- Modify: `rs_openai/src/apis/conversations.rs`

**Interfaces:**
- Consumes: `crate::interfaces::responses::ResponseInputItem` (already exists, from the Responses core slice).
- Produces: `CreateConversationRequest`, `Conversation`; `Conversations::create()`. Task 2 consumes `conversations::Conversation` as the return type for `retrieve`/`update`.

- [ ] **Step 1: Write the failing tests**

Replace the entire contents of `rs_openai/src/apis/conversations.rs`:

```rust
//! Stateful conversation context for the Responses API — replaces the Assistants API's
//! "thread" concept. Core CRUD only this round, no items sub-resource — see
//! docs/superpowers/specs/2026-08-12-conversations-design.md.

use crate::client::OpenAI;
use crate::interfaces::conversations;
pub use crate::interfaces::conversations::*;
use crate::shared::response_wrapper::OpenAIResponse;

pub struct Conversations<'a> {
    openai: &'a OpenAI,
}

impl<'a> Conversations<'a> {
    pub(crate) fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }

    /// Creates a conversation.
    pub async fn create(
        &self,
        req: &conversations::CreateConversationRequest,
    ) -> OpenAIResponse<conversations::Conversation> {
        self.openai.post("/conversations", req).await
    }
}

pub struct Conversation<'a> {
    openai: &'a OpenAI,
    id: String,
}

impl<'a> Conversation<'a> {
    pub(crate) fn new(openai: &'a OpenAI, id: String) -> Self {
        Self { openai, id }
    }
}

#[cfg(test)]
mod tests {
    use crate::client::OpenAI;
    use crate::interfaces::conversations::CreateConversationRequest;
    use crate::interfaces::responses::{ResponseInputItem, ResponseInputRole};
    use wiremock::matchers::{body_json, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn create_serializes_items_and_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/conversations"))
            .and(body_json(serde_json::json!({
                "items": [{"type": "message", "role": "user", "content": "hi"}]
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "conv_123",
                "object": "conversation",
                "created_at": 1700000000_i64,
                "metadata": null
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = CreateConversationRequest::builder()
            .items(vec![ResponseInputItem::Message {
                role: ResponseInputRole::User,
                content: "hi".to_string(),
            }])
            .build();

        let res = client.conversations().create(&req).await.unwrap();
        assert_eq!(res.id, "conv_123");
    }

    #[tokio::test]
    async fn create_without_items_omits_the_field() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/conversations"))
            .and(body_json(serde_json::json!({
                "metadata": {"project": "support"}
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "conv_456",
                "object": "conversation",
                "created_at": 1700000000_i64,
                "metadata": {"project": "support"}
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = CreateConversationRequest::builder()
            .metadata(std::collections::HashMap::from([(
                "project".to_string(),
                "support".to_string(),
            )]))
            .build();

        let res = client.conversations().create(&req).await.unwrap();
        assert_eq!(
            res.metadata.as_ref().and_then(|m| m.get("project")),
            Some(&"support".to_string())
        );
    }
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `NO_PROXY=127.0.0.1,localhost cargo test --workspace --lib apis::conversations`
Expected: FAIL with compile errors — `crate::interfaces::conversations` doesn't exist yet.

- [ ] **Step 3: Implement — create `interfaces/conversations.rs`**

Create `rs_openai/src/interfaces/conversations.rs`:

```rust
use bon::Builder;
use serde::{Deserialize, Serialize};

#[derive(Builder, Debug, Serialize, Clone)]
pub struct CreateConversationRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub items: Option<Vec<crate::interfaces::responses::ResponseInputItem>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<std::collections::HashMap<String, String>>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct Conversation {
    pub id: String,
    pub object: String,
    pub created_at: i64,
    pub metadata: Option<std::collections::HashMap<String, String>>,
}
```

- [ ] **Step 4: Wire `conversations` into `interfaces/mod.rs`**

Replace the full contents of `rs_openai/src/interfaces/mod.rs` with:

```rust
pub mod assistants;
pub mod audio;
pub mod batch;
pub mod chat;
pub mod completions;
pub mod conversations;
pub mod embeddings;
pub mod files;
pub mod fine_tuning;
pub mod images;
pub mod messages;
pub mod models;
pub mod moderations;
pub mod responses;
pub mod runs;
pub mod threads;
pub mod uploads;
pub mod vector_stores;
```

(`apis/mod.rs` and `client.rs` already have `conversations` fully wired from Phase 2a — do not touch either file.)

- [ ] **Step 5: Run the tests to verify they pass**

Run: `NO_PROXY=127.0.0.1,localhost cargo test --workspace --lib apis::conversations`
Expected: PASS — both new tests green.

Run: `NO_PROXY=127.0.0.1,localhost cargo test --workspace`
Expected: PASS — full suite green (baseline count plus 2 new tests).

- [ ] **Step 6: Verify**

Run: `cargo build --workspace --all-targets`
Expected: clean build.

Run: `cargo clippy --workspace --all-targets --all-features -- -D warnings`
Expected: exit 0.

Run: `cargo fmt --all -- --check`
Expected: clean (run `cargo fmt --all` first if needed).

- [ ] **Step 7: Commit**

```bash
git add -A
git commit -m "$(cat <<'EOF'
feat: add Conversations type foundation and create()

Creates interfaces/conversations.rs (didn't exist -- Phase 2a only
scaffolded apis/conversations.rs). CreateConversationRequest.items
reuses Responses' ResponseInputItem directly -- genuine shape/
purpose identity, same scoped-down item family (Message/
FunctionCallOutput) already established there. See
docs/superpowers/specs/2026-08-12-conversations-design.md.
EOF
)"
```

---

### Task 2: `retrieve`/`update`/`delete`

**Files:**
- Modify: `rs_openai/src/interfaces/conversations.rs`
- Modify: `rs_openai/src/apis/conversations.rs`

**Interfaces:**
- Consumes: `conversations::Conversation` (as the return type for `retrieve`/`update`) from Task 1.
- Produces: `UpdateConversationRequest`, `DeleteConversationResponse`; `Conversation::retrieve()`, `Conversation::update()`, `Conversation::delete()`. Nothing later in this plan depends on this task's specifics — Task 3 is a read-only cross-check.

- [ ] **Step 1: Write the failing tests**

Add to `rs_openai/src/apis/conversations.rs`'s test module. Replace:

```rust
    use crate::client::OpenAI;
    use crate::interfaces::conversations::CreateConversationRequest;
    use crate::interfaces::responses::{ResponseInputItem, ResponseInputRole};
    use wiremock::matchers::{body_json, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};
```

with:

```rust
    use crate::client::OpenAI;
    use crate::interfaces::conversations::{CreateConversationRequest, UpdateConversationRequest};
    use crate::interfaces::responses::{ResponseInputItem, ResponseInputRole};
    use wiremock::matchers::{body_json, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};
```

Add these tests to the end of the `mod tests` block:

```rust

    #[tokio::test]
    async fn retrieve_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/conversations/conv_789"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "conv_789",
                "object": "conversation",
                "created_at": 1700000000_i64,
                "metadata": null
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let res = client.conversation("conv_789").retrieve().await.unwrap();
        assert_eq!(res.id, "conv_789");
    }

    #[tokio::test]
    async fn update_serializes_only_metadata() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/conversations/conv_789"))
            .and(body_json(serde_json::json!({
                "metadata": {"status": "archived"}
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "conv_789",
                "object": "conversation",
                "created_at": 1700000000_i64,
                "metadata": {"status": "archived"}
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = UpdateConversationRequest::builder()
            .metadata(std::collections::HashMap::from([(
                "status".to_string(),
                "archived".to_string(),
            )]))
            .build();
        let res = client.conversation("conv_789").update(&req).await.unwrap();
        assert_eq!(
            res.metadata.as_ref().and_then(|m| m.get("status")),
            Some(&"archived".to_string())
        );
    }

    #[tokio::test]
    async fn delete_maps_api_error() {
        let mock_server = MockServer::start().await;

        Mock::given(method("DELETE"))
            .and(path("/conversations/does-not-exist"))
            .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
                "error": {
                    "message": "No conversation found",
                    "type": "invalid_request_error",
                    "param": null,
                    "code": "not_found"
                }
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let err = client
            .conversation("does-not-exist")
            .delete()
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            crate::shared::response_wrapper::OpenAIError::ApiError(_)
        ));
    }
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `NO_PROXY=127.0.0.1,localhost cargo test --workspace --lib apis::conversations`
Expected: FAIL with compile errors — `.retrieve()`, `.update()`, `.delete()` don't exist on `Conversation<'a>` yet, and `UpdateConversationRequest` doesn't exist.

- [ ] **Step 3: Implement**

At the end of `rs_openai/src/interfaces/conversations.rs`, add:

```rust

#[derive(Builder, Debug, Serialize, Clone)]
pub struct UpdateConversationRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<std::collections::HashMap<String, String>>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct DeleteConversationResponse {
    pub id: String,
    pub object: String,
    pub deleted: bool,
}
```

In `rs_openai/src/apis/conversations.rs`, first remove the now-stale `#[allow(dead_code)]` above `pub struct Conversation<'a>` (its fields become genuinely read once `retrieve`/`update`/`delete` exist — same cleanup already established for `Responses`/`Response` and `VectorStores`/`VectorStore` in prior slices). Replace:

```rust
#[allow(dead_code)]
pub struct Conversation<'a> {
    openai: &'a OpenAI,
    id: String,
}
```

with:

```rust
pub struct Conversation<'a> {
    openai: &'a OpenAI,
    id: String,
}
```

Then add the three methods to `impl<'a> Conversation<'a>`. Replace:

```rust
impl<'a> Conversation<'a> {
    pub(crate) fn new(openai: &'a OpenAI, id: String) -> Self {
        Self { openai, id }
    }
}
```

with:

```rust
impl<'a> Conversation<'a> {
    pub(crate) fn new(openai: &'a OpenAI, id: String) -> Self {
        Self { openai, id }
    }

    /// Retrieves a conversation by ID.
    pub async fn retrieve(&self) -> OpenAIResponse<conversations::Conversation> {
        self.openai
            .get(&format!("/conversations/{}", self.id), &())
            .await
    }

    /// Updates a conversation's metadata.
    pub async fn update(
        &self,
        req: &conversations::UpdateConversationRequest,
    ) -> OpenAIResponse<conversations::Conversation> {
        self.openai
            .post(&format!("/conversations/{}", self.id), req)
            .await
    }

    /// Deletes a conversation.
    pub async fn delete(&self) -> OpenAIResponse<conversations::DeleteConversationResponse> {
        self.openai
            .delete(&format!("/conversations/{}", self.id), &())
            .await
    }
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `NO_PROXY=127.0.0.1,localhost cargo test --workspace --lib apis::conversations`
Expected: PASS — all 5 conversations tests green (2 from Task 1, 3 new).

Run: `NO_PROXY=127.0.0.1,localhost cargo test --workspace`
Expected: PASS — full suite green.

- [ ] **Step 5: Verify**

Run: `cargo build --workspace --all-targets`
Expected: clean build.

Run: `cargo clippy --workspace --all-targets --all-features -- -D warnings`
Expected: exit 0.

Run: `cargo fmt --all -- --check`
Expected: clean.

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "$(cat <<'EOF'
feat: add Conversation::retrieve/update/delete

UpdateConversationRequest accepts exactly metadata -- confirmed the
real endpoint does not accept any other field. See
docs/superpowers/specs/2026-08-12-conversations-design.md.
EOF
)"
```

---

### Task 3: Final cross-check and whole-workspace verification

**Files:**
- None modified (verification only, plus a possible trivial fix if the cross-check finds a gap).

**Interfaces:**
- Consumes: everything from Tasks 1-2.
- Produces: nothing further depends on this; it's the last task.

- [ ] **Step 1: Cross-check against the design spec**

Run: `grep -n "pub struct\|pub enum" rs_openai/src/interfaces/conversations.rs`
Expected: exactly 4 types — `CreateConversationRequest`, `Conversation`, `UpdateConversationRequest`, `DeleteConversationResponse`.

Run: `grep -n "pub async fn" rs_openai/src/apis/conversations.rs`
Expected: exactly 4 methods — `create` (on `Conversations<'a>`), `retrieve`/`update`/`delete` (on `Conversation<'a>`).

Run: `grep -rn "items(\|/items\"" rs_openai/src/interfaces/conversations.rs rs_openai/src/apis/conversations.rs`
Expected: no output referencing an `items` HTTP endpoint or a dedicated items-sub-resource method — confirms the deferred sub-resource wasn't accidentally implemented. (The `items` FIELD on `CreateConversationRequest` is expected and correct — this check is specifically about the separate `items` sub-resource endpoints, not that field.)

- [ ] **Step 2: Run the full verification suite on the assembled branch**

Run: `cargo fmt --all -- --check`
Expected: clean.

Run: `cargo build --workspace --all-targets`
Expected: clean build.

Run: `cargo clippy --workspace --all-targets --all-features -- -D warnings`
Expected: exit 0.

Run: `NO_PROXY=127.0.0.1,localhost cargo test --workspace`
Expected: PASS — full suite green, with the conversations module contributing 5 tests total (2 from Task 1, 3 from Task 2, plus whatever the pre-existing baseline was).

- [ ] **Step 3: Done**

If Steps 1-2 all passed clean, this task made no changes — no commit needed. If any check found a real gap, stop and report exactly what it found rather than silently patching it — the fix belongs in the task that owns the affected file, following that task's own conventions.

---
