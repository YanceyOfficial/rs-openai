# Containers — Core CRUD + Files (file_id reference only) — Design

Status: Approved
Date: 2026-08-14

## Context

Fifth of the 9 Phase 2a scaffolds to go from skeleton to working implementation. Containers back the Responses API's `code_interpreter` tool — shipping this unblocks that tool, the only Responses tool type still unimplemented. Phase 2a already scaffolded the accessor structure with zero methods: `apis::containers::{mod.rs, files.rs}` — `Containers<'a>` → (implicit collection ops) and `Container<'a>` → `.files()`, matching the plural/singular accessor pattern already used throughout the crate.

Live research this session (WebFetch against `developers.openai.com`'s API reference, 3 targeted pages) found this to be the most completely and precisely documented API surface researched so far in Phase 2b — every field on Container creation/listing/retrieval/deletion and Container Files was confirmed, including nested `network_policy` (a 2-variant union: `disabled` or `allowlist` with `allowed_domains`/`domain_secrets`) and `memory_limit` (a closed 4-value set: `1g`/`4g`/`16g`/`64g`).

**Two scope decisions confirmed via `AskUserQuestion`**: (1) `skills` (an array of `SkillReference`/`InlineSkill` — the latter carrying a base64-encoded zip bundle) is deferred entirely, since it depends on a wholly separate, unresearched `/v1/skills` API and is tangential to the core sandboxed-container use case; (2) Container file creation supports two request shapes (raw multipart bytes, or a JSON `file_id` referencing an already-uploaded file) — this slice models only the `file_id` path, matching Vector Stores' file-attach precedent, deferring raw multipart upload into a container.

**A genuine `expires_after` non-reuse**, same class of finding as Vector Stores': Containers' `expires_after` uses `anchor: "last_active_at"` (not `"created_at"`) and `minutes` (not `seconds`) — structurally different from both `shared::types::ExpiresAfter` (Files/Uploads: `created_at`/`seconds`) and ChatKit's reused version of that same type. A new `ContainerExpiresAfter`/`ContainerExpiresAfterAnchor` pair is needed. This is now the crate's second explicit "don't assume reuse by field name alone" case.

**A real, newly-discovered correctness concern in this crate's existing binary-download infrastructure**: while researching how to model the Container Files `.../content` endpoint (confirmed to return raw, potentially-binary file content — e.g. Code Interpreter-generated chart images), inspection of `client.rs`'s existing `resolve_raw_text_response`/`resolve_file_response` helpers (used today by `Files::retrieve_content` and `Audio::create_speech`'s `post_with_file_response`) found both call `reqwest::Response::text()`, which performs a lossy UTF-8 decode — silently corrupting any response body that isn't valid UTF-8. This is a latent, pre-existing bug affecting binary content (MP3/OPUS audio bytes, arbitrary file content), NOT introduced by this slice and NOT fixed by this slice (fixing `Files`/`Audio`'s existing behavior is out of scope — a real, unrelated-module fix belongs in its own future slice). This slice avoids the bug for its OWN new endpoint by adding a new, genuinely binary-safe `get_raw_bytes` client method (using `.bytes()`, not `.text()`) rather than reusing the existing lossy helpers.

## Goals

- **Containers**: `Containers::create()` (`POST /containers`), `Containers::list()` (`GET /containers`), `Container::retrieve()` (`GET /containers/{id}`), `Container::delete()` (`DELETE /containers/{id}`).
- `CreateContainerRequest`: `name: String` (required), `expires_after: Option<ContainerExpiresAfter>`, `file_ids: Option<Vec<String>>`, `memory_limit: Option<ContainerMemoryLimit>`, `network_policy: Option<ContainerNetworkPolicy>`. `skills` NOT modeled (deferred).
- `ContainerExpiresAfterAnchor`: single-value enum, `LastActiveAt` only (matches the established precedent for Vector Stores' single-fixed-value anchor enums).
- `ContainerExpiresAfter`: `anchor: ContainerExpiresAfterAnchor`, `minutes: u32`.
- `ContainerMemoryLimit`: 4-value closed enum, `OneGb`/`FourGb`/`SixteenGb`/`SixtyFourGb`, each with an explicit `#[serde(rename = "1g"/"4g"/"16g"/"64g")]` (none of these values are derivable via `rename_all` case conversion).
- `ContainerDomainSecret`: `domain: String`, `name: String`, `value: String`.
- `ContainerNetworkPolicy`: 2-variant tagged union (`#[serde(tag = "type", rename_all = "snake_case")]`), `Disabled` (unit variant, serializes to `{"type": "disabled"}` — same pattern as `ResponseTool::Computer`), `Allowlist { allowed_domains: Vec<String>, domain_secrets: Option<Vec<ContainerDomainSecret>> }`.
- `Container` (response): `id: String`, `created_at: i64`, `name: String`, `object: String`, `status: String` (kept a plain string per this crate's established status-field uniformity convention), `expires_after: Option<ContainerExpiresAfter>`, `last_active_at: Option<i64>`, `memory_limit: Option<ContainerMemoryLimit>`, `network_policy: Option<ContainerNetworkPolicy>`.
- `ListContainersRequest`: `after: Option<String>`, `limit: Option<u32>`, `name: Option<String>` (filter), `order: Option<crate::interfaces::chat::SortOrder>` (reused directly — same "asc"/"desc" shape already reused by Vector Stores/Conversations/Responses).
- `ListContainersResponse`: `object: String`, `data: Vec<Container>`, `first_id: Option<String>`, `last_id: Option<String>`, `has_more: bool` — the crate's universal list-response shape.
- `DeleteContainerResponse`: `id: String`, `object: String`, `deleted: bool` — the crate's universal delete-response shape.
- **Container Files**: `ContainerFiles::create()` (`POST /containers/{container_id}/files`, JSON `file_id` only), `ContainerFiles::list()` (`GET /containers/{container_id}/files`), `ContainerFile::retrieve()` (`GET /containers/{container_id}/files/{file_id}`), `ContainerFile::delete()` (`DELETE /containers/{container_id}/files/{file_id}`), `ContainerFile::content()` (`GET /containers/{container_id}/files/{file_id}/content`, raw bytes — the only endpoint in this crate to date that returns non-JSON, non-text binary data directly).
- `CreateContainerFileRequest`: `file_id: String` (required — the only field this slice models, per the confirmed "file_id-only" scope).
- `ContainerFile` (response): `id: String`, `bytes: u64`, `container_id: String`, `created_at: i64`, `object: String`, `path: String`, `source: String` (confirmed 2-value set `"user"`/`"assistant"`, kept plain string per convention).
- `ListContainerFilesRequest`: `after: Option<String>`, `limit: Option<u32>`, `order: Option<crate::interfaces::chat::SortOrder>` (no `name` filter — not confirmed for this endpoint).
- `ListContainerFilesResponse`: `object: String`, `data: Vec<ContainerFile>`, `first_id: Option<String>`, `last_id: Option<String>`, `has_more: bool`.
- `DeleteContainerFileResponse`: `id: String`, `object: String`, `deleted: bool`.
- **`client.rs` addition**: `get_raw_bytes` (a new, generically-named — not container-specific — method returning `OpenAIResponse<Vec<u8>>`, built on the existing `raw_request` plumbing but calling `response.bytes()` instead of `response.text()`, avoiding the lossy-UTF-8 bug described above). `ContainerFile::content()` uses this.

## Non-goals (explicit, deferred to later slices)

- **`skills`** (`SkillReference`/`InlineSkill`) — depends on the entirely separate, unresearched `/v1/skills` API.
- **Raw multipart file upload into a container** — only the `file_id`-reference JSON path is modeled this slice.
- **Fixing the pre-existing lossy-UTF-8 bug in `Files::retrieve_content`/`Audio::create_speech`'s `post_with_file_response`** — a real, found-but-out-of-scope issue in unrelated modules, worth its own future slice.
- **`status`'s closed value set** — not confirmed (docs only give examples, not an enumeration), stays `String`.

## Design

### File organization

New file `rs_openai/src/interfaces/containers.rs` (registered via `pub mod containers;` in `interfaces/mod.rs`) — holds both Containers and Files types in one file, matching the established convention that `interfaces::<module>` stays flat even when `apis::<module>` is split (`apis::containers::{mod.rs, files.rs}`, mirroring Vector Stores).

### `client.rs` addition

```rust
/// Same as `resolve_raw_text_response`, but preserves byte-for-byte fidelity instead of a
/// lossy UTF-8 decode — required for content that may be genuinely binary (e.g. a Code
/// Interpreter-generated chart image inside a container file).
async fn resolve_raw_bytes_response(request: reqwest::RequestBuilder) -> OpenAIResponse<Vec<u8>> {
    let response = request.send().await?;
    let status = response.status();
    let bytes = response.bytes().await?;

    if !status.is_success() {
        let api_error: ApiErrorResponse =
            serde_json::from_slice(bytes.as_ref()).map_err(OpenAIError::JSONDeserialize)?;
        return Err(OpenAIError::ApiError(api_error));
    }

    Ok(bytes.to_vec())
}

pub(crate) async fn get_raw_bytes(&self, route: &str) -> OpenAIResponse<Vec<u8>> {
    let request = self.raw_request(Method::GET, route)?;
    Self::resolve_raw_bytes_response(request).await
}
```

### Containers types

```rust
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum ContainerExpiresAfterAnchor {
    LastActiveAt,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct ContainerExpiresAfter {
    pub anchor: ContainerExpiresAfterAnchor,
    pub minutes: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ContainerMemoryLimit {
    #[serde(rename = "1g")]
    OneGb,
    #[serde(rename = "4g")]
    FourGb,
    #[serde(rename = "16g")]
    SixteenGb,
    #[serde(rename = "64g")]
    SixtyFourGb,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct ContainerDomainSecret {
    #[builder(into)]
    pub domain: String,
    #[builder(into)]
    pub name: String,
    #[builder(into)]
    pub value: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContainerNetworkPolicy {
    Disabled,
    Allowlist {
        allowed_domains: Vec<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        domain_secrets: Option<Vec<ContainerDomainSecret>>,
    },
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct CreateContainerRequest {
    #[builder(into)]
    pub name: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_after: Option<ContainerExpiresAfter>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_ids: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory_limit: Option<ContainerMemoryLimit>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub network_policy: Option<ContainerNetworkPolicy>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct Container {
    pub id: String,
    pub created_at: i64,
    pub name: String,
    pub object: String,
    pub status: String,
    pub expires_after: Option<ContainerExpiresAfter>,
    pub last_active_at: Option<i64>,
    pub memory_limit: Option<ContainerMemoryLimit>,
    pub network_policy: Option<ContainerNetworkPolicy>,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct ListContainersRequest {
    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,

    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub order: Option<crate::interfaces::chat::SortOrder>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ListContainersResponse {
    pub object: String,
    pub data: Vec<Container>,
    pub first_id: Option<String>,
    pub last_id: Option<String>,
    pub has_more: bool,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct DeleteContainerResponse {
    pub id: String,
    pub object: String,
    pub deleted: bool,
}
```

### Container Files types

```rust
#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct CreateContainerFileRequest {
    #[builder(into)]
    pub file_id: String,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ContainerFile {
    pub id: String,
    pub bytes: u64,
    pub container_id: String,
    pub created_at: i64,
    pub object: String,
    pub path: String,
    pub source: String,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct ListContainerFilesRequest {
    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub order: Option<crate::interfaces::chat::SortOrder>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ListContainerFilesResponse {
    pub object: String,
    pub data: Vec<ContainerFile>,
    pub first_id: Option<String>,
    pub last_id: Option<String>,
    pub has_more: bool,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct DeleteContainerFileResponse {
    pub id: String,
    pub object: String,
    pub deleted: bool,
}
```

### `apis::containers` method wiring

```rust
// mod.rs
impl<'a> Containers<'a> {
    pub async fn create(&self, req: &containers::CreateContainerRequest) -> OpenAIResponse<containers::Container> {
        self.openai.post("/containers", req).await
    }

    pub async fn list(&self, req: &containers::ListContainersRequest) -> OpenAIResponse<containers::ListContainersResponse> {
        self.openai.get("/containers", req).await
    }
}

impl<'a> Container<'a> {
    pub async fn retrieve(&self) -> OpenAIResponse<containers::Container> {
        self.openai.get(&format!("/containers/{}", self.id), &()).await
    }

    pub async fn delete(&self) -> OpenAIResponse<containers::DeleteContainerResponse> {
        self.openai.delete(&format!("/containers/{}", self.id), &()).await
    }
}

// files.rs
impl<'a> ContainerFiles<'a> {
    pub async fn create(&self, req: &containers::CreateContainerFileRequest) -> OpenAIResponse<containers::ContainerFile> {
        self.openai.post(&format!("/containers/{}/files", self.container_id), req).await
    }

    pub async fn list(&self, req: &containers::ListContainerFilesRequest) -> OpenAIResponse<containers::ListContainerFilesResponse> {
        self.openai.get(&format!("/containers/{}/files", self.container_id), req).await
    }

    pub fn file(&self, id: impl Into<String>) -> ContainerFile<'a> {
        ContainerFile::new(self.openai, self.container_id.clone(), id.into())
    }
}

// A new ID-scoped accessor, ContainerFile<'a> — Phase 2a's scaffold only modeled the
// collection-level ContainerFiles<'a>, same gap already seen once before (Vector Stores'
// VectorStoreFile<'a>) and resolved the same way: extend the plural/singular convention one
// level deeper than the scaffold anticipated.
pub struct ContainerFile<'a> {
    openai: &'a OpenAI,
    container_id: String,
    id: String,
}

impl<'a> ContainerFile<'a> {
    pub(crate) fn new(openai: &'a OpenAI, container_id: String, id: String) -> Self {
        Self { openai, container_id, id }
    }

    pub async fn retrieve(&self) -> OpenAIResponse<containers::ContainerFile> {
        self.openai.get(&format!("/containers/{}/files/{}", self.container_id, self.id), &()).await
    }

    pub async fn delete(&self) -> OpenAIResponse<containers::DeleteContainerFileResponse> {
        self.openai.delete(&format!("/containers/{}/files/{}", self.container_id, self.id), &()).await
    }

    pub async fn content(&self) -> OpenAIResponse<Vec<u8>> {
        self.openai.get_raw_bytes(&format!("/containers/{}/files/{}/content", self.container_id, self.id)).await
    }
}
```

Note: unlike ChatKit's naming collisions, `interfaces::containers::Container`/`ContainerFile` do NOT collide with `apis::containers::Container<'a>`/`apis::containers::files::ContainerFile<'a>` in a way requiring special handling beyond the crate's already-established qualified-path convention — same resolution as every prior instance (`Response`, `VectorStore`, `Conversation`, `ChatKitSession`, `ChatKitThread`). Sixth and seventh occurrences.

`Containers<'a>` currently has `#[allow(dead_code)]` from Phase 2a (no accessor method existed to read its field) — this becomes stale once `create`/`list` are added and must be removed in the same task. `ContainerFiles<'a>` similarly becomes stale once `create`/`list`/`file` are added.

## Testing

`wiremock` tests covering: `create` serializing a full `CreateContainerRequest` (with `expires_after`, `memory_limit`, `network_policy` in both `disabled` and `allowlist`-with-`domain_secrets` forms) and deserializing the `Container` response; a minimal `create` test proving optional fields are omitted; `list` serializing query params (`after`/`limit`/`name`/`order`) and deserializing a paginated response; `retrieve`; `delete`; container file `create` serializing `file_id` and deserializing a `ContainerFile`; file `list`/`retrieve`/`delete`; a `content` test using `ResponseTemplate::new(200).set_body_bytes(...)` with genuinely non-UTF-8 bytes (proving `get_raw_bytes` round-trips real binary data correctly, not just ASCII text — the actual regression test for the bug this slice's design deliberately avoids).

## Versioning

Purely additive at the crate level — new module, new types, one new `client.rs` method that doesn't touch any existing method's signature or behavior. Ships as a pre-1.0 minor bump alongside whatever else accumulates before the next release, per established practice.
