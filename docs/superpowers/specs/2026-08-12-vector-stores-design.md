# Vector Stores API: Core (CRUD, Search, Basic File Attach) — Design

Status: Approved
Date: 2026-08-12

## Context

This is the second of Phase 2a's 9 scaffolded-but-empty categories to go from skeleton to working implementation (Responses shipped first). Vector Stores backs the Responses API's `file_search` tool — semantic search over uploaded files — and is likely also a dependency for ChatKit. Chosen next per explicit project-owner direction to keep working through the remaining scaffolds continuously.

Phase 2a already scaffolded the full 3-level accessor shape: `apis/vector_stores/mod.rs` (`VectorStores<'a>` collection + `VectorStore<'a>` ID-scoped, with `.files()`/`.file_batches()` sub-accessors already wired to their own scaffold modules), `apis/vector_stores/files.rs` (`VectorStoreFiles<'a>`, currently just a constructor), `apis/vector_stores/file_batches.rs` (`VectorStoreFileBatches<'a>`, currently just a constructor). No `interfaces/vector_stores.rs` exists yet.

Live research this session (WebFetch against `developers.openai.com`, full field tables successfully retrieved — unlike Responses, these pages were small enough to avoid the fetch tool's size-limit truncation) confirmed: the full `CreateVectorStoreRequest`/`VectorStore` response field sets including `chunking_strategy`'s exact tagged-union shape (`auto` / `static { max_chunk_size_tokens, chunk_overlap_tokens }`, plus a response-only `other` variant for chunking strategies not created via `auto`/`static`); `expires_after`'s exact shape (`anchor: "last_active_at"` — the only valid anchor, `days: number` — notably different from the crate's existing `ExpiresAfter`/`ExpiresAfterAnchor` shared type, which is `anchor: "created_at"`/`seconds: number` for Files/Uploads, so this needs a distinct type, not reuse); `file_counts`' exact sub-fields; the `search` endpoint's full request/response shape including the search-result item's `content` shape (`{type: "text", text: string}`, `type` documented as always `"text"` with no other observed variant); the Files sub-resource's `create` request/response shape; the File Batches sub-resource's `create` request/response shape (researched for completeness even though deferred, see Non-goals); and the `update` endpoint's exact accepted field set (`name`, `metadata`, `expires_after` only — confirmed `file_ids`/`chunking_strategy`/`description` are NOT updatable).

Per explicit project-owner scoping direction (`AskUserQuestion`, confirming the proposed cut over a broader alternative), **this slice covers**: VectorStore CRUD (create/list/retrieve/update/delete) + search, plus the Files sub-resource's create/list/retrieve/delete (no update, no content-retrieval) — everything Responses' `file_search` tool actually needs.

## Goals

- `VectorStores`: `create`, `list`.
- `VectorStore` (ID-scoped): `retrieve`, `update`, `delete`, `search`.
- `VectorStoreFiles` (scoped under a `VectorStore`, via `.files()`): `create` (attach a file), `list`.
- `VectorStoreFile` (new ID-scoped accessor, `.files().file(file_id)`, mirroring the crate's plural/singular convention one level deeper): `retrieve`, `delete`.
- `ChunkingStrategy`: a 3-variant tagged union (`Auto`, `Static`, `Other`) — `Other` exists because the response side can echo back a strategy not expressible via the request-side `auto`/`static` union (e.g. dashboard-created).
- A new, distinct `VectorStoreExpiresAfter`/`VectorStoreExpiresAfterAnchor` type pair (NOT a reuse of the existing `shared::types::ExpiresAfter`/`ExpiresAfterAnchor`) — see Design for the reasoning.
- `search`'s `filters` param kept as `Option<serde_json::Value>` this round (see Non-goals) — everything else on the request/response is fully typed.

## Non-goals (explicit, deferred to later slices)

- **The File Batches sub-resource entirely** (`create`/`retrieve`/`cancel`/`list files` on `VectorStoreFileBatches`) — bulk-attach convenience, not essential when files can already be attached one at a time via `VectorStoreFiles::create`. The scaffold module (`apis/vector_stores/file_batches.rs`) is left untouched.
- **`VectorStoreFiles::update`** (updates a file's `attributes` only) and **`VectorStoreFiles::content`** (retrieves a file's parsed-chunk content) — both real endpoints, both lower-priority than the core attach/list/retrieve/delete lifecycle.
- **`search`'s `filters` param's full typed shape** — OpenAI documents this as a recursive `ComparisonFilter | CompoundFilter` grammar (attribute comparisons combinable via AND/OR). Modeling this precisely is a meaningful chunk of design work on its own with limited immediate payoff (advanced users can still pass raw JSON via `Option<serde_json::Value>`); revisit if a future consumer actually needs typed filter construction.
- **`ranking_options.ranker`'s exact enum values** — not confirmed this session (the fetch returned the field's existence and `score_threshold`'s presence but not an enumerated closed set of ranker names); kept as `Option<String>` rather than guessing at a closed enum.

## Design

### File organization

New file `rs_openai/src/interfaces/vector_stores.rs` (doesn't exist yet). Add `pub mod vector_stores;` to `interfaces/mod.rs` — `vector_stores` sorts after every module currently listed there (`uploads` is presently the last line), so this is a new final line. `apis/mod.rs` and `client.rs` already have `vector_stores` fully wired (Phase 2a scaffold, confirmed unchanged) — no changes needed to either. `apis/vector_stores/mod.rs` and `apis/vector_stores/files.rs` (both already scaffolded) get real methods added; `apis/vector_stores/file_batches.rs` is untouched this round.

### Core types

```rust
#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct StaticChunkingStrategy {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_chunk_size_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chunk_overlap_tokens: Option<u32>,
}

/// How a file is split into searchable chunks. `Other` is response-only — OpenAI echoes this
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
```

```rust
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
```

```rust
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

### `CreateVectorStoreRequest`, `UpdateVectorStoreRequest`, `VectorStore`

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
```

`UpdateVectorStoreRequest` deliberately omits `description`/`file_ids`/`chunking_strategy` — confirmed this session as not accepted by the real `update` endpoint (see Context).

### `list` (VectorStores)

```rust
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

Reuses `crate::interfaces::chat::SortOrder`, same shape/reasoning as every other List endpoint's `order` param across this crate.

```rust
#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct DeleteVectorStoreResponse {
    pub id: String,
    pub object: String,
    pub deleted: bool,
}
```

### `search`

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

`VectorStoreSearchResultContent.r#type` stays a plain `String`, not a tagged-enum-with-one-variant (unlike `ResponseTool`/`ResponseOutputItem` in the Responses slice) — those were designed as extensible unions because multiple real variants were already confirmed to exist even in their scoped-down slice; here, OpenAI documents `type` as *always* `"text"` with no other observed or hinted variant, so a plain field is the YAGNI-consistent choice, not premature extensibility.

### Files sub-resource

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

### Accessor wiring

```rust
impl<'a> VectorStores<'a> {
    pub async fn create(&self, req: &vector_stores::CreateVectorStoreRequest) -> OpenAIResponse<vector_stores::VectorStore> {
        self.openai.post("/vector_stores", req).await
    }

    pub async fn list(&self, req: &vector_stores::ListVectorStoresRequest) -> OpenAIResponse<vector_stores::ListVectorStoresResponse> {
        self.openai.get("/vector_stores", req).await
    }
}

impl<'a> VectorStore<'a> {
    pub async fn retrieve(&self) -> OpenAIResponse<vector_stores::VectorStore> {
        self.openai.get(&format!("/vector_stores/{}", self.id), &()).await
    }

    pub async fn update(&self, req: &vector_stores::UpdateVectorStoreRequest) -> OpenAIResponse<vector_stores::VectorStore> {
        self.openai.post(&format!("/vector_stores/{}", self.id), req).await
    }

    pub async fn delete(&self) -> OpenAIResponse<vector_stores::DeleteVectorStoreResponse> {
        self.openai.delete(&format!("/vector_stores/{}", self.id), &()).await
    }

    pub async fn search(&self, req: &vector_stores::VectorStoreSearchRequest) -> OpenAIResponse<vector_stores::VectorStoreSearchResponse> {
        self.openai.post(&format!("/vector_stores/{}/search", self.id), req).await
    }

    // existing .files()/.file_batches() accessors unchanged
}
```

**Naming note (same resolution as the Responses core slice's `Response`/`Response<'a>` collision, applied twice here):** `interfaces::vector_stores::VectorStore` collides with `apis::vector_stores::VectorStore<'a>` (the ID-scoped accessor, fixed by Phase 2a's scaffold); `interfaces::vector_stores::VectorStoreFile` will likewise collide with the new `apis::vector_stores::files::VectorStoreFile<'a>` accessor introduced below. Both resolved identically: qualified paths (`vector_stores::VectorStore`, `vector_stores::VectorStoreFile`) in the `apis` modules' own method signatures — no renaming of either type.

```rust
impl<'a> VectorStoreFiles<'a> {
    pub async fn create(&self, req: &vector_stores::CreateVectorStoreFileRequest) -> OpenAIResponse<vector_stores::VectorStoreFile> {
        self.openai.post(&format!("/vector_stores/{}/files", self.vector_store_id), req).await
    }

    pub async fn list(&self, req: &vector_stores::ListVectorStoreFilesRequest) -> OpenAIResponse<vector_stores::ListVectorStoreFilesResponse> {
        self.openai.get(&format!("/vector_stores/{}/files", self.vector_store_id), req).await
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
        Self { openai, vector_store_id, file_id }
    }

    pub async fn retrieve(&self) -> OpenAIResponse<vector_stores::VectorStoreFile> {
        self.openai
            .get(&format!("/vector_stores/{}/files/{}", self.vector_store_id, self.file_id), &())
            .await
    }

    pub async fn delete(&self) -> OpenAIResponse<vector_stores::DeleteVectorStoreFileResponse> {
        self.openai
            .delete(&format!("/vector_stores/{}/files/{}", self.vector_store_id, self.file_id), &())
            .await
    }
}
```

This adds one new struct (`VectorStoreFile<'a>`) beyond what Phase 2a scaffolded, since the scaffold only anticipated the collection-level `VectorStoreFiles<'a>` — a natural, additive extension of the established plural/singular pattern, not a deviation from it.

## Testing

`wiremock` tests covering: `create`/`list`/`retrieve`/`update`/`delete` on `VectorStores`/`VectorStore` (standard CRUD round-trips plus one API-error-mapping test); `search` serializing both a single-string and a multi-string `query`, and deserializing a result with populated `content`/`attributes`; a `ChunkingStrategy` deserialization test proving all three variants (`Auto`/`Static`/`Other`) discriminate correctly, including the response-only `Other` variant with no `static` field present; `VectorStoreFiles::create`/`list` plus `VectorStoreFile::retrieve`/`delete` on the new nested accessor.

## Versioning

Purely additive at the crate level (a previously-scaffolded, zero-method category now has real methods) — no existing type or method changes elsewhere in the crate. Ships as a pre-1.0 minor bump alongside whatever else accumulates before the next release, per established practice.
