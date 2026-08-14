# Containers Core CRUD + Files (file_id reference) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement Containers' core CRUD (create/list/retrieve/delete) and its Files sub-resource (create via `file_id` reference, list/retrieve/delete, plus binary content download) on top of Phase 2a's zero-method scaffold. Shipping this unblocks the Responses API's `code_interpreter` tool, the only Responses tool type still unimplemented.

**Architecture:** New file `rs_openai/src/interfaces/containers.rs` holds all request/response types for both Containers and Files (one flat file, matching the `interfaces::vector_stores`/`interfaces::chatkit` precedent even though `apis::containers` is split into `mod.rs`/`files.rs`). Real methods get added to the existing `apis::containers::{mod.rs, files.rs}` scaffold structs. `client.rs` gains one new method, `get_raw_bytes`, needed only by the Files sub-resource's `content()` endpoint (the crate's first genuinely binary-content download).

**Tech Stack:** Rust, `serde`, `bon::Builder`, `wiremock` (including, for the first time, `ResponseTemplate::set_body_bytes` with non-UTF-8 bytes to prove binary content round-trips correctly).

## Global Constraints

- Spec: `docs/superpowers/specs/2026-08-14-containers-design.md`. Two scope decisions confirmed via `AskUserQuestion`: `skills` (SkillReference/InlineSkill) is NOT modeled at all this slice — it depends on a wholly separate, unresearched `/v1/skills` API; container file creation supports ONLY the `file_id`-reference JSON path, NOT raw multipart upload.
- `Container.status` and `ContainerFile.source` both stay plain `String`, not closed enums — matches every other status-like field in this crate.
- `ContainerExpiresAfter`/`ContainerExpiresAfterAnchor` are NEW types, NOT a reuse of `shared::types::ExpiresAfter` — Containers' anchor value (`"last_active_at"`) and duration unit (`minutes`) both genuinely differ from that type's `"created_at"`/`seconds`. Do not reuse it.
- `ContainerMemoryLimit`'s four variants (`OneGb`/`FourGb`/`SixteenGb`/`SixtyFourGb`) each need an EXPLICIT `#[serde(rename = "1g"/"4g"/"16g"/"64g")]` — these values don't follow standard `rename_all` case-conversion from PascalCase, so `rename_all` alone would produce the wrong wire values.
- `ContainerNetworkPolicy::Disabled` is a unit variant (no fields) — under `#[serde(tag = "type", rename_all = "snake_case")]` this serializes to exactly `{"type": "disabled"}`, same pattern already used by `ResponseTool::Computer`.
- `ListContainersRequest.order`/`ListContainerFilesRequest.order` reuse `crate::interfaces::chat::SortOrder` directly — do NOT define a new sort-order enum.
- The new `client.rs` method `get_raw_bytes` MUST use `response.bytes()`, never `response.text()` — the existing `resolve_raw_text_response`/`resolve_file_response` helpers use `.text()`, which performs a lossy UTF-8 decode that silently corrupts genuinely binary content (a real, pre-existing issue in this crate's `Files::retrieve_content`/`Audio::create_speech` path, NOT being fixed by this slice, but NOT to be repeated by this slice's new endpoint either).
- Two naming collisions apply here, same established resolution as `Response`/`VectorStore`/`Conversation`/`ChatKitSession`/`ChatKitThread`: `interfaces::containers::Container` vs `apis::containers::Container<'a>`, and `interfaces::containers::ContainerFile` vs `apis::containers::files::ContainerFile<'a>` (a NEW ID-scoped accessor this slice introduces — Phase 2a's scaffold only modeled the collection-level `ContainerFiles<'a>`, same gap already seen once before for Vector Stores' `VectorStoreFile<'a>`). Resolve via qualified paths inside `apis::containers`'s own method signatures — do not rename either side.
- `#[allow(dead_code)]` on `Containers` (mod.rs) and `ContainerFiles` (files.rs) becomes stale once real methods are added — remove it in the same task.
- Stage commits with **explicit file paths** (`git add path/to/file.rs`), never `git add -A`.
- Run `cargo fmt` and `cargo clippy -p rs_openai --all-targets -- -D warnings` before every commit; both must be clean.
- Environment: this machine's shell routes HTTP through a local proxy that breaks `wiremock` tests. Prefix every `cargo test`/`build`/`clippy` invocation with `NO_PROXY=127.0.0.1,localhost`. This is a local-shell-only concern — never write it into crate code, `Cargo.toml`, or CI config.

---

### Task 1: Container-level CRUD (create/list/retrieve/delete)

**Files:**
- Modify: `rs_openai/src/interfaces/mod.rs` (add `pub mod containers;`)
- Create: `rs_openai/src/interfaces/containers.rs` (Container types only this task — Files types come in Task 2)
- Modify: `rs_openai/src/apis/containers/mod.rs` (add `create`/`list`/`retrieve`/`delete` methods, remove stale `#[allow(dead_code)]` from `Containers`, update doc comments)
- Test: `rs_openai/src/apis/containers/mod.rs` (add a `#[cfg(test)] mod tests` block — none exists yet)

**Interfaces:**
- Consumes: existing `apis::containers::{Containers, Container}` struct shapes from Phase 2a (unchanged — `Containers<'a> { openai: &'a OpenAI }`, `Container<'a> { openai: &'a OpenAI, id: String }`); existing `crate::interfaces::chat::SortOrder` (reused directly).
- Produces: `interfaces::containers::{ContainerExpiresAfterAnchor, ContainerExpiresAfter, ContainerMemoryLimit, ContainerDomainSecret, ContainerNetworkPolicy, CreateContainerRequest, Container, ListContainersRequest, ListContainersResponse, DeleteContainerResponse}` — Task 2 does not depend on these directly (Files types are independent structurally), but the final cross-check does. `Containers::create`/`::list`, `Container::retrieve`/`::delete` methods — no later task calls these, but they establish the file's method-wiring pattern Task 2 mirrors.

- [ ] **Step 1: Write the failing test — `create` serializes the full request and deserializes the response**

Add a test module to `rs_openai/src/apis/containers/mod.rs`:

```rust
#[cfg(test)]
mod tests {
    use crate::client::OpenAI;
    use crate::interfaces::containers::{
        ContainerExpiresAfter, ContainerExpiresAfterAnchor, ContainerMemoryLimit,
        ContainerNetworkPolicy, CreateContainerRequest, ListContainersRequest,
    };
    use wiremock::matchers::{body_json, method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn create_serializes_full_request_and_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/containers"))
            .and(body_json(serde_json::json!({
                "name": "my-container",
                "expires_after": {"anchor": "last_active_at", "minutes": 30},
                "memory_limit": "4g",
                "network_policy": {
                    "type": "allowlist",
                    "allowed_domains": ["example.com"],
                    "domain_secrets": [
                        {"domain": "example.com", "name": "API_KEY", "value": "secret123"}
                    ]
                }
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "cntr_1",
                "created_at": 1700000000_i64,
                "name": "my-container",
                "object": "container",
                "status": "running",
                "expires_after": {"anchor": "last_active_at", "minutes": 30},
                "last_active_at": 1700000000_i64,
                "memory_limit": "4g",
                "network_policy": {
                    "type": "allowlist",
                    "allowed_domains": ["example.com"],
                    "domain_secrets": null
                }
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = CreateContainerRequest::builder()
            .name("my-container")
            .expires_after(
                ContainerExpiresAfter::builder()
                    .anchor(ContainerExpiresAfterAnchor::LastActiveAt)
                    .minutes(30)
                    .build(),
            )
            .memory_limit(ContainerMemoryLimit::FourGb)
            .network_policy(ContainerNetworkPolicy::Allowlist {
                allowed_domains: vec!["example.com".to_string()],
                domain_secrets: Some(vec![
                    crate::interfaces::containers::ContainerDomainSecret::builder()
                        .domain("example.com")
                        .name("API_KEY")
                        .value("secret123")
                        .build(),
                ]),
            })
            .build();

        let res = client.containers().create(&req).await.unwrap();
        assert_eq!(res.id, "cntr_1");
        assert_eq!(res.status, "running");
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai create_serializes_full_request_and_deserializes_response 2>&1 | tail -40`
Expected: FAIL to compile — `crate::interfaces::containers` module doesn't exist yet, `client.containers().create(...)` doesn't exist.

- [ ] **Step 3: Create `rs_openai/src/interfaces/containers.rs` with the Container types**

```rust
//! Sandboxed containers backing the Responses API's `code_interpreter` tool, and their
//! files — see docs/superpowers/specs/2026-08-14-containers-design.md.

use bon::Builder;
use serde::{Deserialize, Serialize};

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

- [ ] **Step 4: Register the new module**

In `rs_openai/src/interfaces/mod.rs`, add (alphabetically, after `pub mod completions;` and before `pub mod conversations;`):

```rust
pub mod containers;
```

- [ ] **Step 5: Implement `Containers::create`/`::list` and `Container::retrieve`/`::delete`, remove stale `#[allow(dead_code)]`**

Replace the full contents of `rs_openai/src/apis/containers/mod.rs` (everything above the `#[cfg(test)]` block from Step 1 — leave that test module in place) with:

```rust
//! Sandboxed containers backing the Responses API's `code_interpreter` tool — see
//! docs/superpowers/specs/2026-08-14-containers-design.md.

pub mod files;

use crate::client::OpenAI;
use crate::interfaces::containers;
pub use crate::interfaces::containers::*;
use crate::shared::response_wrapper::OpenAIResponse;

pub struct Containers<'a> {
    openai: &'a OpenAI,
}

impl<'a> Containers<'a> {
    pub(crate) fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }

    /// Creates a container.
    pub async fn create(
        &self,
        req: &containers::CreateContainerRequest,
    ) -> OpenAIResponse<containers::Container> {
        self.openai.post("/containers", req).await
    }

    /// Lists containers.
    pub async fn list(
        &self,
        req: &containers::ListContainersRequest,
    ) -> OpenAIResponse<containers::ListContainersResponse> {
        self.openai.get("/containers", req).await
    }
}

pub struct Container<'a> {
    openai: &'a OpenAI,
    id: String,
}

impl<'a> Container<'a> {
    pub(crate) fn new(openai: &'a OpenAI, id: String) -> Self {
        Self { openai, id }
    }

    /// Retrieves a container by ID.
    pub async fn retrieve(&self) -> OpenAIResponse<containers::Container> {
        self.openai
            .get(&format!("/containers/{}", self.id), &())
            .await
    }

    /// Deletes a container.
    pub async fn delete(&self) -> OpenAIResponse<containers::DeleteContainerResponse> {
        self.openai
            .delete(&format!("/containers/{}", self.id), &())
            .await
    }

    /// Scoped accessor for files inside this container. Not yet implemented.
    pub fn files(&self) -> files::ContainerFiles<'a> {
        files::ContainerFiles::new(self.openai, self.id.clone())
    }
}
```

- [ ] **Step 6: Run test to verify it passes**

Run: `NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai create_serializes_full_request_and_deserializes_response 2>&1 | tail -30`
Expected: PASS.

- [ ] **Step 7: Write the failing test for `list`**

Add to the same test module in `rs_openai/src/apis/containers/mod.rs`:

```rust
    #[tokio::test]
    async fn list_serializes_query_params_and_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/containers"))
            .and(query_param("limit", "10"))
            .and(query_param("name", "my-container"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "object": "list",
                "data": [{
                    "id": "cntr_2",
                    "created_at": 1700000000_i64,
                    "name": "my-container",
                    "object": "container",
                    "status": "running",
                    "expires_after": null,
                    "last_active_at": 1700000000_i64,
                    "memory_limit": "1g",
                    "network_policy": {"type": "disabled"}
                }],
                "first_id": "cntr_2",
                "last_id": "cntr_2",
                "has_more": false
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = ListContainersRequest::builder()
            .limit(10)
            .name("my-container")
            .build();

        let res = client.containers().list(&req).await.unwrap();
        assert_eq!(res.data.len(), 1);
        assert_eq!(res.data[0].id, "cntr_2");
        assert!(matches!(
            res.data[0].network_policy,
            Some(ContainerNetworkPolicy::Disabled)
        ));
    }
```

- [ ] **Step 8: Run test to verify it passes**

Run: `NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai list_serializes_query_params_and_deserializes_response 2>&1 | tail -20`
Expected: PASS.

- [ ] **Step 9: Write the failing tests for `retrieve` and `delete`**

Add to the same test module:

```rust
    #[tokio::test]
    async fn retrieve_deserializes_container() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/containers/cntr_3"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "cntr_3",
                "created_at": 1700000000_i64,
                "name": "my-container",
                "object": "container",
                "status": "running",
                "expires_after": null,
                "last_active_at": null,
                "memory_limit": null,
                "network_policy": null
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let res = client.container("cntr_3").retrieve().await.unwrap();
        assert_eq!(res.id, "cntr_3");
    }

    #[tokio::test]
    async fn delete_deserializes_confirmation() {
        let mock_server = MockServer::start().await;

        Mock::given(method("DELETE"))
            .and(path("/containers/cntr_3"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "cntr_3",
                "object": "container.deleted",
                "deleted": true
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let res = client.container("cntr_3").delete().await.unwrap();
        assert!(res.deleted);
    }
```

- [ ] **Step 10: Run tests to verify they pass**

Run: `NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai retrieve_deserializes_container delete_deserializes_confirmation 2>&1 | tail -20`
Expected: both PASS.

- [ ] **Step 11: Format, lint, full test run**

Run:
```bash
cargo fmt
NO_PROXY=127.0.0.1,localhost cargo clippy -p rs_openai --all-targets -- -D warnings
NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai 2>&1 | tail -10
```
Expected: clippy clean, all tests pass (158 + 4 new = 162).

- [ ] **Step 12: Commit**

```bash
git add rs_openai/src/interfaces/mod.rs rs_openai/src/interfaces/containers.rs rs_openai/src/apis/containers/mod.rs
git commit -m "feat(containers): add container create/list/retrieve/delete"
```

---

### Task 2: Container Files (create via file_id, list/retrieve/delete/content)

**Files:**
- Modify: `rs_openai/src/interfaces/containers.rs` (add Files types to the same file, after the Container types)
- Modify: `rs_openai/src/client.rs` (add `resolve_raw_bytes_response`/`get_raw_bytes`, immediately after the existing `resolve_raw_text_response` method)
- Modify: `rs_openai/src/apis/containers/files.rs` (add `create`/`list`/`file` methods to `ContainerFiles`, add a NEW `ContainerFile<'a>` ID-scoped accessor with `retrieve`/`delete`/`content`, remove stale `#[allow(dead_code)]` from `ContainerFiles`)
- Test: `rs_openai/src/apis/containers/files.rs` (add a `#[cfg(test)] mod tests` block — none exists yet)

**Interfaces:**
- Consumes: `OpenAI::get`/`post`/`delete` (unchanged, used for all Files endpoints except `content`); existing `apis::containers::files::ContainerFiles` struct shape from Phase 2a (`ContainerFiles<'a> { openai: &'a OpenAI, container_id: String }`, unchanged).
- Produces: `interfaces::containers::{CreateContainerFileRequest, ContainerFile, ListContainerFilesRequest, ListContainerFilesResponse, DeleteContainerFileResponse}`; `OpenAI::get_raw_bytes(&self, route: &str) -> OpenAIResponse<Vec<u8>>` on `client.rs` — this is the last new client method in this plan; `apis::containers::files::ContainerFile<'a>` (new struct, not in the Phase 2a scaffold) with `retrieve`/`delete`/`content` methods.

- [ ] **Step 1: Write the failing test — `create` serializes `file_id` and deserializes the response**

Add a test module to `rs_openai/src/apis/containers/files.rs`:

```rust
#[cfg(test)]
mod tests {
    use crate::client::OpenAI;
    use crate::interfaces::containers::{CreateContainerFileRequest, ListContainerFilesRequest};
    use wiremock::matchers::{body_json, method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn create_serializes_file_id_and_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/containers/cntr_1/files"))
            .and(body_json(serde_json::json!({"file_id": "file_abc"})))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "cfile_1",
                "bytes": 1024,
                "container_id": "cntr_1",
                "created_at": 1700000000_i64,
                "object": "container.file",
                "path": "/mnt/data/file_abc.csv",
                "source": "user"
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = CreateContainerFileRequest::builder()
            .file_id("file_abc")
            .build();

        let res = client
            .container("cntr_1")
            .files()
            .create(&req)
            .await
            .unwrap();
        assert_eq!(res.id, "cfile_1");
        assert_eq!(res.container_id, "cntr_1");
        assert_eq!(res.source, "user");
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai create_serializes_file_id_and_deserializes_response 2>&1 | tail -40`
Expected: FAIL to compile — `CreateContainerFileRequest` doesn't exist, `.files().create(...)` doesn't exist.

- [ ] **Step 3: Add the Files types to `rs_openai/src/interfaces/containers.rs`**

Append to the end of `rs_openai/src/interfaces/containers.rs` (after the `DeleteContainerResponse` struct from Task 1):

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

- [ ] **Step 4: Add `get_raw_bytes` to `client.rs`**

In `rs_openai/src/client.rs`, insert immediately after the `resolve_raw_text_response` method (search for it — it ends with `Ok(text) }` before `async fn resolve_file_response`):

```rust
    /// Same as `resolve_raw_text_response`, but preserves byte-for-byte fidelity instead of a
    /// lossy UTF-8 decode — required for content that may be genuinely binary (e.g. a Code
    /// Interpreter-generated chart image inside a container file). Do not reuse
    /// `resolve_raw_text_response`/`resolve_file_response` for binary content — both call
    /// `response.text()`, which silently corrupts non-UTF-8 bytes.
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

- [ ] **Step 5: Implement `ContainerFiles::create`/`::list`/`::file`, add `ContainerFile<'a>`, remove stale `#[allow(dead_code)]`**

Replace the full contents of `rs_openai/src/apis/containers/files.rs` (everything above the `#[cfg(test)]` block from Step 1 — leave that test module in place) with:

```rust
//! A container's files (`/containers/{container_id}/files`), including binary content
//! retrieve — see docs/superpowers/specs/2026-08-14-containers-design.md.

use crate::client::OpenAI;
use crate::interfaces::containers;
use crate::shared::response_wrapper::OpenAIResponse;

pub struct ContainerFiles<'a> {
    openai: &'a OpenAI,
    container_id: String,
}

impl<'a> ContainerFiles<'a> {
    pub(crate) fn new(openai: &'a OpenAI, container_id: String) -> Self {
        Self {
            openai,
            container_id,
        }
    }

    /// Attaches an already-uploaded file (by `file_id`) to this container.
    pub async fn create(
        &self,
        req: &containers::CreateContainerFileRequest,
    ) -> OpenAIResponse<containers::ContainerFile> {
        self.openai
            .post(&format!("/containers/{}/files", self.container_id), req)
            .await
    }

    /// Lists this container's files.
    pub async fn list(
        &self,
        req: &containers::ListContainerFilesRequest,
    ) -> OpenAIResponse<containers::ListContainerFilesResponse> {
        self.openai
            .get(&format!("/containers/{}/files", self.container_id), req)
            .await
    }

    /// Scoped accessor for a specific file in this container.
    pub fn file(&self, id: impl Into<String>) -> ContainerFile<'a> {
        ContainerFile::new(self.openai, self.container_id.clone(), id.into())
    }
}

pub struct ContainerFile<'a> {
    openai: &'a OpenAI,
    container_id: String,
    id: String,
}

impl<'a> ContainerFile<'a> {
    pub(crate) fn new(openai: &'a OpenAI, container_id: String, id: String) -> Self {
        Self {
            openai,
            container_id,
            id,
        }
    }

    /// Retrieves this file's metadata.
    pub async fn retrieve(&self) -> OpenAIResponse<containers::ContainerFile> {
        self.openai
            .get(
                &format!("/containers/{}/files/{}", self.container_id, self.id),
                &(),
            )
            .await
    }

    /// Deletes this file from the container.
    pub async fn delete(&self) -> OpenAIResponse<containers::DeleteContainerFileResponse> {
        self.openai
            .delete(
                &format!("/containers/{}/files/{}", self.container_id, self.id),
                &(),
            )
            .await
    }

    /// Downloads this file's raw content as bytes (may be binary, e.g. a generated image).
    pub async fn content(&self) -> OpenAIResponse<Vec<u8>> {
        self.openai
            .get_raw_bytes(&format!(
                "/containers/{}/files/{}/content",
                self.container_id, self.id
            ))
            .await
    }
}
```

- [ ] **Step 6: Run test to verify it passes**

Run: `NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai create_serializes_file_id_and_deserializes_response 2>&1 | tail -30`
Expected: PASS.

- [ ] **Step 7: Write the failing tests for `list`, `retrieve`, `delete`, and `content`**

Add to the same test module in `rs_openai/src/apis/containers/files.rs`:

```rust
    #[tokio::test]
    async fn list_serializes_query_params_and_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/containers/cntr_1/files"))
            .and(query_param("limit", "5"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "object": "list",
                "data": [{
                    "id": "cfile_2",
                    "bytes": 512,
                    "container_id": "cntr_1",
                    "created_at": 1700000000_i64,
                    "object": "container.file",
                    "path": "/mnt/data/output.png",
                    "source": "assistant"
                }],
                "first_id": "cfile_2",
                "last_id": "cfile_2",
                "has_more": false
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = ListContainerFilesRequest::builder().limit(5).build();
        let res = client
            .container("cntr_1")
            .files()
            .list(&req)
            .await
            .unwrap();
        assert_eq!(res.data.len(), 1);
        assert_eq!(res.data[0].source, "assistant");
    }

    #[tokio::test]
    async fn retrieve_deserializes_file() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/containers/cntr_1/files/cfile_3"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "cfile_3",
                "bytes": 256,
                "container_id": "cntr_1",
                "created_at": 1700000000_i64,
                "object": "container.file",
                "path": "/mnt/data/notes.txt",
                "source": "user"
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let res = client
            .container("cntr_1")
            .files()
            .file("cfile_3")
            .retrieve()
            .await
            .unwrap();
        assert_eq!(res.id, "cfile_3");
    }

    #[tokio::test]
    async fn delete_deserializes_confirmation() {
        let mock_server = MockServer::start().await;

        Mock::given(method("DELETE"))
            .and(path("/containers/cntr_1/files/cfile_3"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "cfile_3",
                "object": "container.file.deleted",
                "deleted": true
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let res = client
            .container("cntr_1")
            .files()
            .file("cfile_3")
            .delete()
            .await
            .unwrap();
        assert!(res.deleted);
    }

    #[tokio::test]
    async fn content_returns_raw_bytes_without_utf8_corruption() {
        let mock_server = MockServer::start().await;

        // Deliberately invalid UTF-8 (0xFF, 0xFE are never valid standalone UTF-8 bytes) --
        // proves `content()` round-trips genuinely binary data byte-for-byte, unlike this
        // crate's existing `.text()`-based download helpers, which would silently replace
        // these bytes during a lossy UTF-8 decode.
        let binary_content: Vec<u8> = vec![0xFF, 0xFE, 0x00, 0x01, 0x89, 0x50, 0x4E, 0x47];

        Mock::given(method("GET"))
            .and(path("/containers/cntr_1/files/cfile_4/content"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(binary_content.clone()))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let res = client
            .container("cntr_1")
            .files()
            .file("cfile_4")
            .content()
            .await
            .unwrap();
        assert_eq!(res, binary_content);
    }
```

- [ ] **Step 8: Run tests to verify they pass**

Run: `NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai list_serializes_query_params_and_deserializes_response retrieve_deserializes_file delete_deserializes_confirmation content_returns_raw_bytes_without_utf8_corruption 2>&1 | tail -30`
Expected: all 4 PASS.

- [ ] **Step 9: Format, lint, full test run**

Run:
```bash
cargo fmt
NO_PROXY=127.0.0.1,localhost cargo clippy -p rs_openai --all-targets -- -D warnings
NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai 2>&1 | tail -10
```
Expected: clippy clean, all tests pass (162 + 5 new = 167).

- [ ] **Step 10: Commit**

```bash
git add rs_openai/src/interfaces/containers.rs rs_openai/src/client.rs rs_openai/src/apis/containers/files.rs
git commit -m "feat(containers): add container files create/list/retrieve/delete/content"
```

---

### Task 3: Final cross-check

**Files:**
- No new code changes expected — this task verifies the whole slice, and only touches files if it finds a real gap.

**Interfaces:**
- Consumes: everything from Task 1 and Task 2.
- Produces: nothing new — verification-only, same role as the final task in every prior Phase 2b slice's plan.

- [ ] **Step 1: Run the full test suite, clippy, and build**

Run:
```bash
cargo fmt --check
NO_PROXY=127.0.0.1,localhost cargo clippy -p rs_openai --all-targets -- -D warnings
NO_PROXY=127.0.0.1,localhost cargo build -p rs_openai
NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai 2>&1 | tail -10
```
Expected: fmt clean (no diff), clippy clean, build clean, 167/167 tests passing.

- [ ] **Step 2: Manually verify scope and cleanup, reading the actual files (don't just trust the step count)**

- `rs_openai/src/apis/containers/mod.rs`: `Containers` struct has NO `#[allow(dead_code)]` (has real methods now).
- `rs_openai/src/apis/containers/files.rs`: `ContainerFiles` struct has NO `#[allow(dead_code)]`; `ContainerFile<'a>` is a genuinely new struct not present in the Phase 2a scaffold.
- `rs_openai/src/interfaces/containers.rs` defines exactly these 15 types: `ContainerExpiresAfterAnchor`, `ContainerExpiresAfter`, `ContainerMemoryLimit`, `ContainerDomainSecret`, `ContainerNetworkPolicy`, `CreateContainerRequest`, `Container`, `ListContainersRequest`, `ListContainersResponse`, `DeleteContainerResponse`, `CreateContainerFileRequest`, `ContainerFile`, `ListContainerFilesRequest`, `ListContainerFilesResponse`, `DeleteContainerFileResponse` — no `skills`-related types (`SkillReference`, `InlineSkill`, etc.) exist anywhere in the file (grep for "Skill" should return zero matches).
- `rs_openai/src/client.rs`: confirm `get_raw_bytes`/`resolve_raw_bytes_response` exist and use `.bytes()`, not `.text()` (grep the new method bodies for `.text()` should return zero matches); confirm no existing `get`/`post`/`delete`/`raw_request`/`resolve_raw_text_response`/`resolve_file_response` method body was modified.
- `rs_openai/src/apis/containers/mod.rs` contains `pub use crate::interfaces::containers::*;` (matching the `apis::vector_stores::mod.rs`/`apis::chatkit::mod.rs` precedent for a split apis module) — confirm `rs_openai::containers::Container` and `rs_openai::containers::files::ContainerFile` (via the `files` submodule) are reachable from outside the crate.

If any of these checks fail, fix it directly and re-run Step 1.

- [ ] **Step 3: No commit needed for this task if Step 2 found nothing to fix** (if it did, commit the fix with an explicit-path `git add` and a message describing what was found).
