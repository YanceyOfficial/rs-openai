# Vector Stores API: Core Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement the core Vector Stores API — CRUD (`create`/`list`/`retrieve`/`update`/`delete`) + `search` on `VectorStores`/`VectorStore`, plus basic file attach lifecycle (`create`/`list`/`retrieve`/`delete`) on the Files sub-resource — turning Phase 2a's zero-method scaffolds into working accessors.

**Architecture:** New file `rs_openai/src/interfaces/vector_stores.rs` holds every request/response type. `rs_openai/src/apis/vector_stores/mod.rs`'s already-scaffolded `VectorStores<'a>`/`VectorStore<'a>` structs gain real methods. `rs_openai/src/apis/vector_stores/files.rs`'s already-scaffolded `VectorStoreFiles<'a>` gains real methods plus a brand-new `VectorStoreFile<'a>` ID-scoped accessor (not in the original scaffold — a natural extension of the plural/singular pattern one level deeper). Six tasks: (1) standalone type foundation, (2) create+list on VectorStores, (3) retrieve+update+delete on VectorStore, (4) search, (5) the Files sub-resource in full, (6) final cross-check.

**Tech Stack:** Same as the rest of the crate (Rust edition 2024, `bon` builders, `serde`, `strum::Display`, `wiremock` tests) — no new dependencies.

## Global Constraints

- Spec: `docs/superpowers/specs/2026-08-12-vector-stores-design.md`.
- **Deliberately out of scope this round — do not add:** the File Batches sub-resource (`apis/vector_stores/file_batches.rs` stays untouched), `VectorStoreFiles::update`/`::content`, a typed `filters` grammar on `search` (stays `Option<serde_json::Value>`), a closed enum for `ranking_options.ranker` (stays `Option<String>`).
- `ChunkingStrategy` is a 3-variant tagged union: `Auto`, `Static { config: StaticChunkingStrategy }` (wire key `static`), `Other` (response-only, no request use case, but must still deserialize — no fields).
- **Do not reuse `shared::types::ExpiresAfter`/`ExpiresAfterAnchor`** for Vector Stores' expiration — that type is `anchor: "created_at"` + `seconds` (Files/Uploads). Vector Stores' expiration is `anchor: "last_active_at"` + `days` — a new, distinct `VectorStoreExpiresAfter`/`VectorStoreExpiresAfterAnchor` pair.
- Any enum deriving both `strum::Display` and `serde::Serialize`/`Deserialize` needs matching rename coverage on both sides (`#[serde(rename_all = "snake_case")]` + `#[strum(serialize_all = "snake_case")]`) — a standing crate-wide rule from prior slices. Applies to `VectorStoreStatus` and `VectorStoreFileStatus`, `VectorStoreExpiresAfterAnchor` in this plan.
- `UpdateVectorStoreRequest` accepts exactly `name`/`expires_after`/`metadata` — confirmed this session that `description`/`file_ids`/`chunking_strategy` are NOT accepted by the real `update` endpoint. Do not add them.
- Naming collision, resolved and non-negotiable (same resolution the Responses core slice already established and this plan reuses twice): `interfaces::vector_stores::VectorStore` collides with `apis::vector_stores::VectorStore<'a>` (the accessor); `interfaces::vector_stores::VectorStoreFile` collides with the new `apis::vector_stores::files::VectorStoreFile<'a>` accessor. Inside each `apis` module, refer to the interface type via the qualified path `vector_stores::VectorStore` / `vector_stores::VectorStoreFile` wherever both names are in scope — ordinary Rust namespace disambiguation, not a new pattern.
- `apis/mod.rs` and `client.rs` already have `vector_stores` fully wired (Phase 2a scaffold: `pub fn vector_stores(&self) -> vector_stores::VectorStores<'_>` and `pub fn vector_store(&self, id: impl Into<String>) -> vector_stores::VectorStore<'_>` both already exist in `client.rs`) — do not touch either file in this plan.
- Every task's "done" check includes `cargo build --workspace --all-targets`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace`, and `cargo fmt --all -- --check`.
- **This machine needs `NO_PROXY=127.0.0.1,localhost` prefixed on every `cargo test` invocation**, or every `wiremock`-based HTTP test fails with a misleading `JSONDeserialize(Error("EOF while parsing a value", ...))` (a local proxy-client environment quirk, not a code defect — pure `#[test]`s with no HTTP are unaffected). E.g. `NO_PROXY=127.0.0.1,localhost cargo test --workspace`.
- Follow TDD: write the failing test first, confirm it fails for the expected reason, then implement.
- `interfaces/vector_stores.rs` does not exist yet — Task 1 creates it and wires `pub mod vector_stores;` into `interfaces/mod.rs` (as the new last line — `vector_stores` sorts after every module currently listed there).

---

### Task 1: Type foundation — chunking strategy, expiration, status, file counts

**Files:**
- Create: `rs_openai/src/interfaces/vector_stores.rs`
- Modify: `rs_openai/src/interfaces/mod.rs`
- Modify: `rs_openai/src/apis/vector_stores/mod.rs` (re-export line + test scaffolding)

**Interfaces:**
- Consumes: nothing from earlier tasks (first task).
- Produces: `ChunkingStrategy`, `StaticChunkingStrategy`, `VectorStoreExpiresAfterAnchor`, `VectorStoreExpiresAfter`, `VectorStoreStatus`, `VectorStoreFileCounts`. Task 2 consumes `ChunkingStrategy`/`VectorStoreExpiresAfter`/`VectorStoreStatus`/`VectorStoreFileCounts` directly in `CreateVectorStoreRequest`/`VectorStore`.

This task adds no HTTP methods — pure type definitions, tested via direct `serde_json` serialize/deserialize assertions.

- [ ] **Step 1: Write the failing tests**

Replace the entire contents of `rs_openai/src/apis/vector_stores/mod.rs`:

```rust
//! Semantic search over uploaded files, used by the Responses API's `file_search` tool. Core
//! surface only this round: CRUD + search, plus a basic file-attach lifecycle -- see
//! docs/superpowers/specs/2026-08-12-vector-stores-design.md.

pub mod file_batches;
pub mod files;

use crate::client::OpenAI;
use crate::interfaces::vector_stores;
pub use crate::interfaces::vector_stores::*;

pub struct VectorStores<'a> {
    openai: &'a OpenAI,
}

impl<'a> VectorStores<'a> {
    pub(crate) fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }
}

pub struct VectorStore<'a> {
    openai: &'a OpenAI,
    id: String,
}

impl<'a> VectorStore<'a> {
    pub(crate) fn new(openai: &'a OpenAI, id: String) -> Self {
        Self { openai, id }
    }

    /// Scoped accessor for files attached to this vector store.
    pub fn files(&self) -> files::VectorStoreFiles<'a> {
        files::VectorStoreFiles::new(self.openai, self.id.clone())
    }

    /// Scoped accessor for batch file-attach operations on this vector store. Not yet
    /// implemented.
    pub fn file_batches(&self) -> file_batches::VectorStoreFileBatches<'a> {
        file_batches::VectorStoreFileBatches::new(self.openai, self.id.clone())
    }
}

#[cfg(test)]
mod tests {
    use crate::interfaces::vector_stores::{ChunkingStrategy, StaticChunkingStrategy};

    #[test]
    fn chunking_strategy_auto_serializes_with_type_tag_only() {
        let strategy = ChunkingStrategy::Auto;
        let json = serde_json::to_value(&strategy).unwrap();
        assert_eq!(json, serde_json::json!({"type": "auto"}));
    }

    #[test]
    fn chunking_strategy_static_serializes_with_nested_static_key() {
        let strategy = ChunkingStrategy::Static {
            config: StaticChunkingStrategy::builder()
                .max_chunk_size_tokens(400_u32)
                .chunk_overlap_tokens(100_u32)
                .build(),
        };
        let json = serde_json::to_value(&strategy).unwrap();
        assert_eq!(
            json,
            serde_json::json!({
                "type": "static",
                "static": {"max_chunk_size_tokens": 400, "chunk_overlap_tokens": 100}
            })
        );
    }

    #[test]
    fn chunking_strategy_other_deserializes_with_no_fields() {
        let value = serde_json::json!({"type": "other"});
        let strategy: ChunkingStrategy = serde_json::from_value(value).unwrap();
        assert!(matches!(strategy, ChunkingStrategy::Other));
    }
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `NO_PROXY=127.0.0.1,localhost cargo test --workspace --lib apis::vector_stores`
Expected: FAIL with compile errors — `crate::interfaces::vector_stores` doesn't exist yet.

- [ ] **Step 3: Implement — create `interfaces/vector_stores.rs`**

Create `rs_openai/src/interfaces/vector_stores.rs`:

```rust
use bon::Builder;
use serde::{Deserialize, Serialize};

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct StaticChunkingStrategy {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_chunk_size_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chunk_overlap_tokens: Option<u32>,
}

/// How a file is split into searchable chunks. `Other` is response-only -- OpenAI echoes this
/// back for any strategy not created via `Auto`/`Static` through this API (e.g. dashboard-set).
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ChunkingStrategy {
    Auto,
    Static {
        #[serde(rename = "static")]
        config: StaticChunkingStrategy,
    },
    Other,
}

/// Distinct from `shared::types::ExpiresAfter` (Files/Uploads, `anchor: "created_at"` +
/// `seconds`) -- Vector Stores' expiration uses a different anchor value and a different
/// duration unit (`days`), so reusing that type would either be wrong or force it to accept
/// anchor/unit combinations no real endpoint supports.
#[derive(Debug, Serialize, Deserialize, Clone, strum::Display)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum VectorStoreExpiresAfterAnchor {
    LastActiveAt,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct VectorStoreExpiresAfter {
    pub anchor: VectorStoreExpiresAfterAnchor,
    pub days: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone, strum::Display)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum VectorStoreStatus {
    Expired,
    InProgress,
    Completed,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct VectorStoreFileCounts {
    pub total: u32,
    pub completed: u32,
    pub in_progress: u32,
    pub failed: u32,
    pub cancelled: u32,
}
```

- [ ] **Step 4: Wire `vector_stores` into `interfaces/mod.rs`**

Replace the full contents of `rs_openai/src/interfaces/mod.rs` with:

```rust
pub mod assistants;
pub mod audio;
pub mod batch;
pub mod chat;
pub mod completions;
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

(`apis/mod.rs` and `client.rs` already have `vector_stores` fully wired from Phase 2a — do not touch either file.)

- [ ] **Step 5: Run the tests to verify they pass**

Run: `NO_PROXY=127.0.0.1,localhost cargo test --workspace --lib apis::vector_stores`
Expected: PASS — all 3 new tests green.

Run: `NO_PROXY=127.0.0.1,localhost cargo test --workspace`
Expected: PASS — full suite green (baseline count plus 3 new tests).

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
feat: add Vector Stores type foundation (chunking, expiration, status)

Creates interfaces/vector_stores.rs (didn't exist -- Phase 2a only
scaffolded the apis/vector_stores/ directory). ChunkingStrategy is a
3-variant tagged union (Auto/Static/Other) -- Other is response-only,
covering strategies not created via this API. VectorStoreExpiresAfter
is deliberately NOT a reuse of the existing Files/Uploads
ExpiresAfter type -- different anchor value (last_active_at vs
created_at) and duration unit (days vs seconds). No HTTP methods yet.
See docs/superpowers/specs/2026-08-12-vector-stores-design.md.
EOF
)"
```

---

### Task 2: `create`/`list` on `VectorStores`

**Files:**
- Modify: `rs_openai/src/interfaces/vector_stores.rs`
- Modify: `rs_openai/src/apis/vector_stores/mod.rs`

**Interfaces:**
- Consumes: `ChunkingStrategy`, `VectorStoreExpiresAfter`, `VectorStoreStatus`, `VectorStoreFileCounts` from Task 1; `crate::interfaces::chat::SortOrder` (already exists).
- Produces: `CreateVectorStoreRequest`, `VectorStore`, `ListVectorStoresRequest`, `ListVectorStoresResponse`, and `VectorStores::create()`/`::list()`. Task 3 consumes `vector_stores::VectorStore` as the return type for `retrieve`/`update`. Task 4 consumes `VectorStore` for `search`'s context. Task 5 does not depend on this task's specifics.

- [ ] **Step 1: Write the failing tests**

In `rs_openai/src/apis/vector_stores/mod.rs`, add imports and tests to the `mod tests` block. Replace:

```rust
#[cfg(test)]
mod tests {
    use crate::interfaces::vector_stores::{ChunkingStrategy, StaticChunkingStrategy};
```

with:

```rust
#[cfg(test)]
mod tests {
    use crate::client::OpenAI;
    use crate::interfaces::chat::SortOrder;
    use crate::interfaces::vector_stores::{
        ChunkingStrategy, CreateVectorStoreRequest, ListVectorStoresRequest,
        StaticChunkingStrategy,
    };
    use wiremock::matchers::{body_json, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};
```

Add these tests to the end of the `mod tests` block:

```rust

    #[tokio::test]
    async fn create_serializes_request_and_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/vector_stores"))
            .and(body_json(serde_json::json!({
                "name": "Support docs",
                "file_ids": ["file-abc123"]
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "vs_123",
                "object": "vector_store",
                "name": "Support docs",
                "description": null,
                "status": "in_progress",
                "created_at": 1700000000_i64,
                "last_active_at": null,
                "expires_at": null,
                "expires_after": null,
                "usage_bytes": 0,
                "file_counts": {
                    "total": 1, "completed": 0, "in_progress": 1, "failed": 0, "cancelled": 0
                },
                "metadata": null
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = CreateVectorStoreRequest::builder()
            .name("Support docs")
            .file_ids(vec!["file-abc123".to_string()])
            .build();

        let res = client.vector_stores().create(&req).await.unwrap();
        assert_eq!(res.id, "vs_123");
        assert_eq!(res.file_counts.in_progress, 1);
    }

    #[tokio::test]
    async fn create_maps_api_error() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/vector_stores"))
            .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
                "error": {
                    "message": "name must be a string",
                    "type": "invalid_request_error",
                    "param": "name",
                    "code": null
                }
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = CreateVectorStoreRequest::builder().build();

        let err = client.vector_stores().create(&req).await.unwrap_err();
        match err {
            crate::shared::response_wrapper::OpenAIError::ApiError(e) => {
                assert_eq!(e.error.param, Some(serde_json::json!("name")));
            }
            other => panic!("expected ApiError, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn list_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/vector_stores"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "object": "list",
                "data": [{
                    "id": "vs_123",
                    "object": "vector_store",
                    "name": "Support docs",
                    "description": null,
                    "status": "completed",
                    "created_at": 1700000000_i64,
                    "last_active_at": 1700000100_i64,
                    "expires_at": null,
                    "expires_after": null,
                    "usage_bytes": 1024,
                    "file_counts": {
                        "total": 1, "completed": 1, "in_progress": 0, "failed": 0, "cancelled": 0
                    },
                    "metadata": null
                }],
                "first_id": "vs_123",
                "last_id": "vs_123",
                "has_more": false
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = ListVectorStoresRequest::builder()
            .limit(10_u32)
            .order(SortOrder::Desc)
            .build();
        let res = client.vector_stores().list(&req).await.unwrap();

        assert_eq!(res.data.len(), 1);
        assert_eq!(res.data[0].usage_bytes, 1024);
        assert!(!res.has_more);
    }
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `NO_PROXY=127.0.0.1,localhost cargo test --workspace --lib apis::vector_stores`
Expected: FAIL with compile errors — `CreateVectorStoreRequest`, `ListVectorStoresRequest`, and `VectorStores::create`/`::list` don't exist yet.

- [ ] **Step 3: Implement**

At the end of `rs_openai/src/interfaces/vector_stores.rs`, add:

```rust

#[derive(Builder, Debug, Serialize, Clone)]
pub struct CreateVectorStoreRequest {
    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_ids: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub chunking_strategy: Option<ChunkingStrategy>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_after: Option<VectorStoreExpiresAfter>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<std::collections::HashMap<String, String>>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct VectorStore {
    pub id: String,
    pub object: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub status: VectorStoreStatus,
    pub created_at: i64,
    pub last_active_at: Option<i64>,
    pub expires_at: Option<i64>,
    pub expires_after: Option<VectorStoreExpiresAfter>,
    pub usage_bytes: u64,
    pub file_counts: VectorStoreFileCounts,
    pub metadata: Option<std::collections::HashMap<String, String>>,
}

#[derive(Builder, Debug, Serialize, Clone)]
pub struct ListVectorStoresRequest {
    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub order: Option<crate::interfaces::chat::SortOrder>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ListVectorStoresResponse {
    pub object: String,
    pub data: Vec<VectorStore>,
    pub first_id: Option<String>,
    pub last_id: Option<String>,
    pub has_more: bool,
}
```

**Note — this "old_string" reflects the plan's original template, not Task 1's actual output.** Task 1's implementer omitted the bare `use crate::interfaces::vector_stores;` line (it would have been unused dead code at that point in the plan, correctly flagged by clippy) and left the pre-existing `#[allow(dead_code)]` above `VectorStores<'a>` untouched (also pre-existing, from the Phase 2a scaffold). If your actual file doesn't match the "old_string" below exactly, use this note to reconcile: you need to (a) add back `use crate::interfaces::vector_stores;` (now genuinely needed, since `create`/`list` use qualified `vector_stores::Type` paths), and (b) remove the stale `#[allow(dead_code)]` above `VectorStores<'a>` (its field becomes genuinely read once `create`/`list` call `self.openai...`).

In `rs_openai/src/apis/vector_stores/mod.rs`, add the necessary imports and the `create`/`list` methods. Replace:

```rust
use crate::client::OpenAI;
pub use crate::interfaces::vector_stores::*;

#[allow(dead_code)]
pub struct VectorStores<'a> {
    openai: &'a OpenAI,
}

impl<'a> VectorStores<'a> {
    pub(crate) fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }
}
```

with:

```rust
use crate::client::OpenAI;
use crate::interfaces::vector_stores;
pub use crate::interfaces::vector_stores::*;
use crate::shared::response_wrapper::OpenAIResponse;

pub struct VectorStores<'a> {
    openai: &'a OpenAI,
}

impl<'a> VectorStores<'a> {
    pub(crate) fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }

    /// Creates a vector store.
    pub async fn create(
        &self,
        req: &vector_stores::CreateVectorStoreRequest,
    ) -> OpenAIResponse<vector_stores::VectorStore> {
        self.openai.post("/vector_stores", req).await
    }

    /// Lists vector stores.
    pub async fn list(
        &self,
        req: &vector_stores::ListVectorStoresRequest,
    ) -> OpenAIResponse<vector_stores::ListVectorStoresResponse> {
        self.openai.get("/vector_stores", req).await
    }
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `NO_PROXY=127.0.0.1,localhost cargo test --workspace --lib apis::vector_stores`
Expected: PASS — all 6 vector_stores tests green (3 from Task 1, 3 new).

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
feat: add CreateVectorStoreRequest, the VectorStore object, create/list

The collection-level operations on VectorStores. See
docs/superpowers/specs/2026-08-12-vector-stores-design.md.
EOF
)"
```

---

### Task 3: `retrieve`/`update`/`delete` on `VectorStore`

**Files:**
- Modify: `rs_openai/src/interfaces/vector_stores.rs`
- Modify: `rs_openai/src/apis/vector_stores/mod.rs`

**Interfaces:**
- Consumes: `vector_stores::VectorStore` (as the return type for `retrieve`/`update`) from Task 2.
- Produces: `UpdateVectorStoreRequest`, `DeleteVectorStoreResponse`; `VectorStore::retrieve()`, `VectorStore::update()`, `VectorStore::delete()`. Task 4 doesn't depend on this task's specific additions, only on the same `VectorStore<'a>` accessor struct existing.

- [ ] **Step 1: Write the failing tests**

Add to `rs_openai/src/apis/vector_stores/mod.rs`'s `mod tests` block (place at the end). Extend the imports — replace:

```rust
    use crate::interfaces::vector_stores::{
        ChunkingStrategy, CreateVectorStoreRequest, ListVectorStoresRequest,
        StaticChunkingStrategy,
    };
```

with:

```rust
    use crate::interfaces::vector_stores::{
        ChunkingStrategy, CreateVectorStoreRequest, ListVectorStoresRequest,
        StaticChunkingStrategy, UpdateVectorStoreRequest,
    };
```

Add these tests:

```rust

    #[tokio::test]
    async fn retrieve_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/vector_stores/vs_456"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "vs_456",
                "object": "vector_store",
                "name": null,
                "description": null,
                "status": "completed",
                "created_at": 1700000000_i64,
                "last_active_at": null,
                "expires_at": null,
                "expires_after": null,
                "usage_bytes": 0,
                "file_counts": {
                    "total": 0, "completed": 0, "in_progress": 0, "failed": 0, "cancelled": 0
                },
                "metadata": null
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let res = client.vector_store("vs_456").retrieve().await.unwrap();
        assert_eq!(res.id, "vs_456");
    }

    #[tokio::test]
    async fn update_serializes_only_updatable_fields() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/vector_stores/vs_456"))
            .and(body_json(serde_json::json!({"name": "Renamed"})))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "vs_456",
                "object": "vector_store",
                "name": "Renamed",
                "description": null,
                "status": "completed",
                "created_at": 1700000000_i64,
                "last_active_at": null,
                "expires_at": null,
                "expires_after": null,
                "usage_bytes": 0,
                "file_counts": {
                    "total": 0, "completed": 0, "in_progress": 0, "failed": 0, "cancelled": 0
                },
                "metadata": null
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = UpdateVectorStoreRequest::builder().name("Renamed").build();
        let res = client.vector_store("vs_456").update(&req).await.unwrap();
        assert_eq!(res.name.as_deref(), Some("Renamed"));
    }

    #[tokio::test]
    async fn delete_maps_api_error() {
        let mock_server = MockServer::start().await;

        Mock::given(method("DELETE"))
            .and(path("/vector_stores/does-not-exist"))
            .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
                "error": {
                    "message": "No vector store found",
                    "type": "invalid_request_error",
                    "param": null,
                    "code": "not_found"
                }
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let err = client
            .vector_store("does-not-exist")
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

Run: `NO_PROXY=127.0.0.1,localhost cargo test --workspace --lib apis::vector_stores`
Expected: FAIL with compile errors — `.retrieve()`, `.update()`, `.delete()` don't exist on `VectorStore<'a>` yet, and `UpdateVectorStoreRequest` doesn't exist.

- [ ] **Step 3: Implement**

At the end of `rs_openai/src/interfaces/vector_stores.rs`, add:

```rust

#[derive(Builder, Debug, Serialize, Clone)]
pub struct UpdateVectorStoreRequest {
    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_after: Option<VectorStoreExpiresAfter>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<std::collections::HashMap<String, String>>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct DeleteVectorStoreResponse {
    pub id: String,
    pub object: String,
    pub deleted: bool,
}
```

In `rs_openai/src/apis/vector_stores/mod.rs`, add the three methods to `impl<'a> VectorStore<'a>`. Replace:

```rust
impl<'a> VectorStore<'a> {
    pub(crate) fn new(openai: &'a OpenAI, id: String) -> Self {
        Self { openai, id }
    }

    /// Scoped accessor for files attached to this vector store.
    pub fn files(&self) -> files::VectorStoreFiles<'a> {
        files::VectorStoreFiles::new(self.openai, self.id.clone())
    }
```

with:

```rust
impl<'a> VectorStore<'a> {
    pub(crate) fn new(openai: &'a OpenAI, id: String) -> Self {
        Self { openai, id }
    }

    /// Retrieves a vector store by ID.
    pub async fn retrieve(&self) -> OpenAIResponse<vector_stores::VectorStore> {
        self.openai
            .get(&format!("/vector_stores/{}", self.id), &())
            .await
    }

    /// Updates a vector store's name, expiration policy, or metadata.
    pub async fn update(
        &self,
        req: &vector_stores::UpdateVectorStoreRequest,
    ) -> OpenAIResponse<vector_stores::VectorStore> {
        self.openai
            .post(&format!("/vector_stores/{}", self.id), req)
            .await
    }

    /// Deletes a vector store.
    pub async fn delete(&self) -> OpenAIResponse<vector_stores::DeleteVectorStoreResponse> {
        self.openai
            .delete(&format!("/vector_stores/{}", self.id), &())
            .await
    }

    /// Scoped accessor for files attached to this vector store.
    pub fn files(&self) -> files::VectorStoreFiles<'a> {
        files::VectorStoreFiles::new(self.openai, self.id.clone())
    }
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `NO_PROXY=127.0.0.1,localhost cargo test --workspace --lib apis::vector_stores`
Expected: PASS — all 9 vector_stores tests green (6 from Tasks 1-2, 3 new).

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
feat: add VectorStore::retrieve/update/delete

UpdateVectorStoreRequest accepts exactly name/expires_after/metadata
-- confirmed the real endpoint does not accept
description/file_ids/chunking_strategy. See
docs/superpowers/specs/2026-08-12-vector-stores-design.md.
EOF
)"
```

---

### Task 4: `search`

**Files:**
- Modify: `rs_openai/src/interfaces/vector_stores.rs`
- Modify: `rs_openai/src/apis/vector_stores/mod.rs`

**Interfaces:**
- Consumes: nothing new from earlier tasks beyond the `VectorStore<'a>` accessor struct.
- Produces: `VectorStoreSearchQuery`, `VectorStoreSearchRankingOptions`, `VectorStoreSearchRequest`, `VectorStoreSearchResultContent`, `VectorStoreSearchResult`, `VectorStoreSearchResponse`; `VectorStore::search()`. Nothing later depends on this task's specifics.

- [ ] **Step 1: Write the failing tests**

Add to `rs_openai/src/apis/vector_stores/mod.rs`'s `mod tests` block. Extend the imports — replace:

```rust
    use crate::interfaces::vector_stores::{
        ChunkingStrategy, CreateVectorStoreRequest, ListVectorStoresRequest,
        StaticChunkingStrategy, UpdateVectorStoreRequest,
    };
```

with:

```rust
    use crate::interfaces::vector_stores::{
        ChunkingStrategy, CreateVectorStoreRequest, ListVectorStoresRequest,
        StaticChunkingStrategy, UpdateVectorStoreRequest, VectorStoreSearchQuery,
        VectorStoreSearchRequest,
    };
```

Add these tests:

```rust

    #[tokio::test]
    async fn search_serializes_single_string_query() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/vector_stores/vs_456/search"))
            .and(body_json(serde_json::json!({"query": "refund policy"})))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "object": "vector_store.search_results.page",
                "search_query": ["refund policy"],
                "data": [{
                    "file_id": "file-abc123",
                    "filename": "policy.md",
                    "score": 0.87,
                    "attributes": {"category": "billing"},
                    "content": [{"type": "text", "text": "Refunds are processed within 5 days."}]
                }],
                "has_more": false,
                "next_page": null
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = VectorStoreSearchRequest::builder()
            .query(VectorStoreSearchQuery::Single("refund policy".to_string()))
            .build();

        let res = client.vector_store("vs_456").search(&req).await.unwrap();
        assert_eq!(res.data.len(), 1);
        assert_eq!(res.data[0].file_id, "file-abc123");
        assert_eq!(res.data[0].content[0].text, "Refunds are processed within 5 days.");
    }

    #[tokio::test]
    async fn search_serializes_multiple_string_query() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/vector_stores/vs_456/search"))
            .and(body_json(serde_json::json!({
                "query": ["refund policy", "cancellation policy"]
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "object": "vector_store.search_results.page",
                "search_query": ["refund policy", "cancellation policy"],
                "data": [],
                "has_more": false,
                "next_page": null
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = VectorStoreSearchRequest::builder()
            .query(VectorStoreSearchQuery::Multiple(vec![
                "refund policy".to_string(),
                "cancellation policy".to_string(),
            ]))
            .build();

        let res = client.vector_store("vs_456").search(&req).await.unwrap();
        assert_eq!(res.search_query.len(), 2);
    }
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `NO_PROXY=127.0.0.1,localhost cargo test --workspace --lib apis::vector_stores`
Expected: FAIL with compile errors — `VectorStoreSearchRequest`, `VectorStoreSearchQuery`, and `VectorStore::search` don't exist yet.

- [ ] **Step 3: Implement**

At the end of `rs_openai/src/interfaces/vector_stores.rs`, add:

```rust

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum VectorStoreSearchQuery {
    Single(String),
    Multiple(Vec<String>),
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct VectorStoreSearchRankingOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ranker: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub score_threshold: Option<f32>,
}

#[derive(Builder, Debug, Serialize, Clone)]
pub struct VectorStoreSearchRequest {
    pub query: VectorStoreSearchQuery,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub filters: Option<serde_json::Value>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_num_results: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub ranking_options: Option<VectorStoreSearchRankingOptions>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub rewrite_query: Option<bool>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct VectorStoreSearchResultContent {
    pub r#type: String,
    pub text: String,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct VectorStoreSearchResult {
    pub file_id: String,
    pub filename: String,
    pub score: f32,
    pub attributes: Option<std::collections::HashMap<String, serde_json::Value>>,
    pub content: Vec<VectorStoreSearchResultContent>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct VectorStoreSearchResponse {
    pub object: String,
    pub search_query: Vec<String>,
    pub data: Vec<VectorStoreSearchResult>,
    pub has_more: bool,
    pub next_page: Option<String>,
}
```

In `rs_openai/src/apis/vector_stores/mod.rs`, add the `search` method to `impl<'a> VectorStore<'a>` (after `delete`, before `files`):

```rust

    /// Searches a vector store's attached files.
    pub async fn search(
        &self,
        req: &vector_stores::VectorStoreSearchRequest,
    ) -> OpenAIResponse<vector_stores::VectorStoreSearchResponse> {
        self.openai
            .post(&format!("/vector_stores/{}/search", self.id), req)
            .await
    }
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `NO_PROXY=127.0.0.1,localhost cargo test --workspace --lib apis::vector_stores`
Expected: PASS — all 11 vector_stores tests green (9 from Tasks 1-3, 2 new).

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
feat: add VectorStore::search

query is an untagged Single(String)/Multiple(Vec<String>) union.
filters stays Option<serde_json::Value> this round -- OpenAI's real
filter grammar is a recursive comparison/compound-filter union,
deliberately deferred per the design's Non-goals. See
docs/superpowers/specs/2026-08-12-vector-stores-design.md.
EOF
)"
```

---

### Task 5: Files sub-resource — `create`/`list`/`retrieve`/`delete`

**Files:**
- Modify: `rs_openai/src/interfaces/vector_stores.rs`
- Modify: `rs_openai/src/apis/vector_stores/files.rs`

**Interfaces:**
- Consumes: `ChunkingStrategy` from Task 1; `crate::interfaces::chat::SortOrder` (already exists).
- Produces: `VectorStoreFileStatus`, `VectorStoreFileError`, `VectorStoreFile`, `CreateVectorStoreFileRequest`, `ListVectorStoreFilesRequest`, `ListVectorStoreFilesResponse`, `DeleteVectorStoreFileResponse`; `VectorStoreFiles::create()`/`::list()`/`::file()`; a brand-new `VectorStoreFile<'a>` accessor struct with `::retrieve()`/`::delete()`. Nothing later in this plan depends on this task's specifics — Task 6 is a read-only cross-check.

- [ ] **Step 1: Write the failing tests**

Replace the entire contents of `rs_openai/src/apis/vector_stores/files.rs`:

```rust
//! A vector store's attached files (`/v1/vector_stores/{vector_store_id}/files`). Core
//! lifecycle only this round: attach/list/retrieve/delete -- see
//! docs/superpowers/specs/2026-08-12-vector-stores-design.md.

use crate::client::OpenAI;
use crate::interfaces::vector_stores;
use crate::shared::response_wrapper::OpenAIResponse;

pub struct VectorStoreFiles<'a> {
    openai: &'a OpenAI,
    vector_store_id: String,
}

impl<'a> VectorStoreFiles<'a> {
    pub(crate) fn new(openai: &'a OpenAI, vector_store_id: String) -> Self {
        Self {
            openai,
            vector_store_id,
        }
    }

    /// Attaches a file to a vector store.
    pub async fn create(
        &self,
        req: &vector_stores::CreateVectorStoreFileRequest,
    ) -> OpenAIResponse<vector_stores::VectorStoreFile> {
        self.openai
            .post(
                &format!("/vector_stores/{}/files", self.vector_store_id),
                req,
            )
            .await
    }

    /// Lists a vector store's attached files.
    pub async fn list(
        &self,
        req: &vector_stores::ListVectorStoreFilesRequest,
    ) -> OpenAIResponse<vector_stores::ListVectorStoreFilesResponse> {
        self.openai
            .get(
                &format!("/vector_stores/{}/files", self.vector_store_id),
                req,
            )
            .await
    }

    /// Scoped accessor for a single attached file, mirroring the crate's plural/singular
    /// convention one level deeper (`client.responses()` -> `client.response(id)`, here
    /// `vector_store.files()` -> `.file(id)`).
    pub fn file(&self, file_id: impl Into<String>) -> VectorStoreFile<'a> {
        VectorStoreFile::new(self.openai, self.vector_store_id.clone(), file_id.into())
    }
}

pub struct VectorStoreFile<'a> {
    openai: &'a OpenAI,
    vector_store_id: String,
    file_id: String,
}

impl<'a> VectorStoreFile<'a> {
    pub(crate) fn new(openai: &'a OpenAI, vector_store_id: String, file_id: String) -> Self {
        Self {
            openai,
            vector_store_id,
            file_id,
        }
    }

    /// Retrieves a vector store file by ID.
    pub async fn retrieve(&self) -> OpenAIResponse<vector_stores::VectorStoreFile> {
        self.openai
            .get(
                &format!(
                    "/vector_stores/{}/files/{}",
                    self.vector_store_id, self.file_id
                ),
                &(),
            )
            .await
    }

    /// Deletes a vector store file (detaches it -- does not delete the underlying File object).
    pub async fn delete(&self) -> OpenAIResponse<vector_stores::DeleteVectorStoreFileResponse> {
        self.openai
            .delete(
                &format!(
                    "/vector_stores/{}/files/{}",
                    self.vector_store_id, self.file_id
                ),
                &(),
            )
            .await
    }
}

#[cfg(test)]
mod tests {
    use crate::client::OpenAI;
    use crate::interfaces::chat::SortOrder;
    use crate::interfaces::vector_stores::{CreateVectorStoreFileRequest, ListVectorStoreFilesRequest};
    use wiremock::matchers::{body_json, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn create_serializes_request_and_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/vector_stores/vs_456/files"))
            .and(body_json(serde_json::json!({"file_id": "file-abc123"})))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "vsf_1",
                "object": "vector_store.file",
                "created_at": 1700000000_i64,
                "usage_bytes": 512,
                "vector_store_id": "vs_456",
                "status": "in_progress",
                "last_error": null,
                "chunking_strategy": {
                    "type": "static",
                    "static": {"max_chunk_size_tokens": 800, "chunk_overlap_tokens": 400}
                },
                "attributes": null
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = CreateVectorStoreFileRequest::builder()
            .file_id("file-abc123")
            .build();

        let res = client
            .vector_store("vs_456")
            .files()
            .create(&req)
            .await
            .unwrap();
        assert_eq!(res.id, "vsf_1");
        assert_eq!(res.vector_store_id, "vs_456");
    }

    #[tokio::test]
    async fn list_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/vector_stores/vs_456/files"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "object": "list",
                "data": [{
                    "id": "vsf_1",
                    "object": "vector_store.file",
                    "created_at": 1700000000_i64,
                    "usage_bytes": 512,
                    "vector_store_id": "vs_456",
                    "status": "completed",
                    "last_error": null,
                    "chunking_strategy": {"type": "auto"},
                    "attributes": null
                }],
                "first_id": "vsf_1",
                "last_id": "vsf_1",
                "has_more": false
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = ListVectorStoreFilesRequest::builder()
            .limit(10_u32)
            .order(SortOrder::Desc)
            .build();
        let res = client
            .vector_store("vs_456")
            .files()
            .list(&req)
            .await
            .unwrap();
        assert_eq!(res.data.len(), 1);
    }

    #[tokio::test]
    async fn retrieve_deserializes_response_with_failed_status_and_error() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/vector_stores/vs_456/files/vsf_2"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "vsf_2",
                "object": "vector_store.file",
                "created_at": 1700000000_i64,
                "usage_bytes": 0,
                "vector_store_id": "vs_456",
                "status": "failed",
                "last_error": {"code": "server_error", "message": "internal error"},
                "chunking_strategy": {"type": "other"},
                "attributes": null
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let res = client
            .vector_store("vs_456")
            .files()
            .file("vsf_2")
            .retrieve()
            .await
            .unwrap();

        assert_eq!(res.status.to_string(), "failed");
        assert_eq!(
            res.last_error.as_ref().map(|e| e.message.as_str()),
            Some("internal error")
        );
    }

    #[tokio::test]
    async fn delete_maps_api_error() {
        let mock_server = MockServer::start().await;

        Mock::given(method("DELETE"))
            .and(path("/vector_stores/vs_456/files/does-not-exist"))
            .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
                "error": {
                    "message": "No file found",
                    "type": "invalid_request_error",
                    "param": null,
                    "code": "not_found"
                }
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let err = client
            .vector_store("vs_456")
            .files()
            .file("does-not-exist")
            .delete()
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            crate::shared::response_wrapper::OpenAIError::ApiError(_)
        ));
    }
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `NO_PROXY=127.0.0.1,localhost cargo test --workspace --lib apis::vector_stores`
Expected: FAIL with compile errors — `CreateVectorStoreFileRequest`, `ListVectorStoreFilesRequest`, `VectorStoreFile` (the interface type), `VectorStoreFileStatus`, `VectorStoreFileError`, `ListVectorStoreFilesResponse`, `DeleteVectorStoreFileResponse` don't exist yet.

- [ ] **Step 3: Implement**

At the end of `rs_openai/src/interfaces/vector_stores.rs`, add:

```rust

#[derive(Builder, Debug, Serialize, Clone)]
pub struct CreateVectorStoreFileRequest {
    #[builder(into)]
    pub file_id: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub attributes: Option<std::collections::HashMap<String, serde_json::Value>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub chunking_strategy: Option<ChunkingStrategy>,
}

#[derive(Debug, Serialize, Deserialize, Clone, strum::Display)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum VectorStoreFileStatus {
    InProgress,
    Completed,
    Cancelled,
    Failed,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct VectorStoreFileError {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct VectorStoreFile {
    pub id: String,
    pub object: String,
    pub created_at: i64,
    pub usage_bytes: u64,
    pub vector_store_id: String,
    pub status: VectorStoreFileStatus,
    pub last_error: Option<VectorStoreFileError>,
    pub chunking_strategy: Option<ChunkingStrategy>,
    pub attributes: Option<std::collections::HashMap<String, serde_json::Value>>,
}

#[derive(Builder, Debug, Serialize, Clone)]
pub struct ListVectorStoreFilesRequest {
    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub order: Option<crate::interfaces::chat::SortOrder>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ListVectorStoreFilesResponse {
    pub object: String,
    pub data: Vec<VectorStoreFile>,
    pub first_id: Option<String>,
    pub last_id: Option<String>,
    pub has_more: bool,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct DeleteVectorStoreFileResponse {
    pub id: String,
    pub object: String,
    pub deleted: bool,
}
```

`rs_openai/src/apis/vector_stores/files.rs`'s methods and `VectorStoreFile<'a>` accessor struct were already written in full in Step 1 above (this task builds the file in one pass, unlike Tasks 2-4 which incrementally edit an existing file) — no further changes to that file are needed in this step.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `NO_PROXY=127.0.0.1,localhost cargo test --workspace --lib apis::vector_stores`
Expected: PASS — all 15 vector_stores tests green (11 from Tasks 1-4, 4 new in `files.rs`).

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
feat: add VectorStoreFiles create/list and VectorStoreFile retrieve/delete

VectorStoreFile<'a> is a new ID-scoped accessor beyond what Phase 2a
scaffolded (which only anticipated the collection-level
VectorStoreFiles) -- a natural extension of the plural/singular
convention one level deeper. File Batches and Files' update/content
endpoints remain deliberately out of scope this round. See
docs/superpowers/specs/2026-08-12-vector-stores-design.md.
EOF
)"
```

---

### Task 6: Final cross-check and whole-workspace verification

**Files:**
- None modified (verification only, plus a possible trivial fix if the cross-check finds a gap).

**Interfaces:**
- Consumes: everything from Tasks 1-5.
- Produces: nothing further depends on this; it's the last task.

- [ ] **Step 1: Cross-check against the design spec**

Run: `grep -n "pub struct\|pub enum" rs_openai/src/interfaces/vector_stores.rs`
Expected: every type named in the design spec's Design section is present — `StaticChunkingStrategy`, `ChunkingStrategy`, `VectorStoreExpiresAfterAnchor`, `VectorStoreExpiresAfter`, `VectorStoreStatus`, `VectorStoreFileCounts`, `CreateVectorStoreRequest`, `VectorStore`, `ListVectorStoresRequest`, `ListVectorStoresResponse`, `DeleteVectorStoreResponse`, `UpdateVectorStoreRequest`, `VectorStoreSearchQuery`, `VectorStoreSearchRankingOptions`, `VectorStoreSearchRequest`, `VectorStoreSearchResultContent`, `VectorStoreSearchResult`, `VectorStoreSearchResponse`, `CreateVectorStoreFileRequest`, `VectorStoreFileStatus`, `VectorStoreFileError`, `VectorStoreFile`, `ListVectorStoreFilesRequest`, `ListVectorStoreFilesResponse`, `DeleteVectorStoreFileResponse` — 24 types total.

Run: `grep -n "pub async fn" rs_openai/src/apis/vector_stores/mod.rs rs_openai/src/apis/vector_stores/files.rs`
Expected: exactly 10 methods across both files — `create`, `list`, `retrieve`, `update`, `delete`, `search` on `VectorStores`/`VectorStore` in `mod.rs` (6), plus `create`, `list`, `retrieve`, `delete` on `VectorStoreFiles`/`VectorStoreFile` in `files.rs` (4). `retrieve`/`delete` each appear twice total (once per resource level) — confirm by checking each method name against its owning `impl` block, not just the raw grep count.

Run: `grep -rn "file_batches::\|FileBatch\|ComparisonFilter\|CompoundFilter" rs_openai/src/interfaces/vector_stores.rs rs_openai/src/apis/vector_stores/mod.rs rs_openai/src/apis/vector_stores/files.rs`
Expected: no output — confirms the File Batches sub-resource and the typed filter grammar were not accidentally implemented.

Run: `grep -n "pub async fn" rs_openai/src/apis/vector_stores/file_batches.rs`
Expected: no output — confirms this scaffold file was never touched.

- [ ] **Step 2: Run the full verification suite on the assembled branch**

Run: `cargo fmt --all -- --check`
Expected: clean.

Run: `cargo build --workspace --all-targets`
Expected: clean build.

Run: `cargo clippy --workspace --all-targets --all-features -- -D warnings`
Expected: exit 0.

Run: `NO_PROXY=127.0.0.1,localhost cargo test --workspace`
Expected: PASS — full suite green, with the vector_stores module contributing 15 tests total (3 from Task 1, 3 from Task 2, 3 from Task 3, 2 from Task 4, 4 from Task 5, plus whatever the pre-existing baseline was).

- [ ] **Step 3: Done**

If Steps 1-2 all passed clean, this task made no changes — no commit needed. If any check found a real gap, stop and report exactly what it found rather than silently patching it — the fix belongs in the task that owns the affected file, following that task's own conventions.

---
