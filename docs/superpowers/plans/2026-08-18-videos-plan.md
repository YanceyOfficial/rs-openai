# Videos Core CRUD + Content Download Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement Videos' core CRUD (create/list/retrieve/delete) and content download on top of Phase 2a's zero-method scaffold, reusing the `get_raw_bytes` client method already built for Containers.

**Architecture:** New file `rs_openai/src/interfaces/videos.rs` holds every Video type. Real methods get added to the existing `apis::videos::{Videos, Video}` scaffold. No `client.rs` changes — `get_raw_bytes` already exists. This is a well-contained slice (one task, matching the `code_interpreter` plan's shape).

**Tech Stack:** Rust, `serde`, `bon::Builder`, `strum::Display` (on `VideoContentVariant`, needed for URL query-string construction — the crate's first such use case), `wiremock`.

## Global Constraints

- Spec: `docs/superpowers/specs/2026-08-18-videos-design.md`. Scope confirmed via `AskUserQuestion`: core CRUD + content download only — `input_reference`, `characters`, `/videos/extensions`, `/videos/edits`, and `/videos/characters` (character upload) are all explicitly deferred. Do not model any of them.
- `CreateVideoRequest.seconds` is `Option<u32>` (request-side, numeric) — `Video.seconds` is `Option<String>` (response-side, a quoted string in OpenAI's own example). These are GENUINELY different types on two different structs — do NOT make them the same type or "fix" this into symmetry; it's a confirmed, deliberate asymmetry, the same class of finding the Evals slice's Critical bug taught this session to check for explicitly.
- `VideoSize`/`model` on the other hand ARE reused identically in both directions (confirmed matching wire values in both the request and the response example) — do not split these into separate request/response types, that would be over-applying the `seconds` lesson to a field pair that doesn't need it.
- `Video.status` stays a plain `String`, not a closed enum — matches this crate's status-field uniformity convention.
- Do NOT add `expires_at`/`remixed_from_video_id` or any other Video field not present in the confirmed response example — no guessing.
- `Video::content()` builds its own query string manually (`format!("/videos/{}/content?variant={v}", ...)`) rather than requiring a `client.rs` signature change — `get_raw_bytes(&self, route: &str)` already takes a plain route string.
- Naming collision (`interfaces::videos::Video` vs `apis::videos::Video<'a>`) resolved via qualified paths, not renaming — the crate's now eighth instance of this established pattern.
- Stage commits with **explicit file paths** (`git add path/to/file.rs`), never `git add -A`.
- Run `cargo fmt` and `cargo clippy -p rs_openai --all-targets -- -D warnings` before every commit; both must be clean.
- Environment: this machine's shell routes HTTP through a local proxy that breaks `wiremock` tests. Prefix every `cargo test`/`build`/`clippy` invocation with `NO_PROXY=127.0.0.1,localhost`. This is a local-shell-only concern — never write it into crate code, `Cargo.toml`, or CI config.

---

### Task 1: Videos core CRUD + content download

**Files:**
- Modify: `rs_openai/src/interfaces/mod.rs` (add `pub mod videos;`)
- Create: `rs_openai/src/interfaces/videos.rs`
- Modify: `rs_openai/src/apis/videos.rs` (add methods, remove stale `#[allow(dead_code)]`, update doc comments)
- Test: `rs_openai/src/apis/videos.rs` (add a `#[cfg(test)] mod tests` block — none exists yet)

**Interfaces:**
- Consumes: existing `apis::videos::{Videos, Video}` struct shapes from Phase 2a (`Videos<'a> { openai: &'a OpenAI }`, `Video<'a> { openai: &'a OpenAI, id: String }`); existing `crate::interfaces::chat::SortOrder`; existing `OpenAI::get_raw_bytes` (from the Containers slice, unchanged).
- Produces: `interfaces::videos::{VideoSize, CreateVideoRequest, VideoError, Video, ListVideosRequest, ListVideosResponse, DeleteVideoResponse, VideoContentVariant}`, `Videos::create`/`::list`, `Video::retrieve`/`::delete`/`::content` — this is the only task in this plan; the final cross-check (Step 11 below) verifies scope.

- [ ] **Step 1: Write the failing test — `create` serializes the full request and deserializes the response, proving the `seconds` asymmetry**

Add a test module to `rs_openai/src/apis/videos.rs`:

```rust
#[cfg(test)]
mod tests {
    use crate::client::OpenAI;
    use crate::interfaces::videos::{
        CreateVideoRequest, ListVideosRequest, VideoContentVariant, VideoSize,
    };
    use wiremock::matchers::{body_json, method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn create_serializes_full_request_and_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/videos"))
            .and(body_json(serde_json::json!({
                "model": "sora-2",
                "prompt": "A gray tabby cat playing piano",
                "size": "1280x720",
                "seconds": 8
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "video_1",
                "object": "video",
                "created_at": 1758941485_i64,
                "status": "queued",
                "model": "sora-2",
                "progress": 0,
                "seconds": "8",
                "size": "1280x720",
                "error": null
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = CreateVideoRequest::builder()
            .model("sora-2")
            .prompt("A gray tabby cat playing piano")
            .size(VideoSize::S1280x720)
            .seconds(8)
            .build();

        let res = client.videos().create(&req).await.unwrap();
        assert_eq!(res.id, "video_1");
        assert_eq!(res.status, "queued");
        assert_eq!(res.seconds.as_deref(), Some("8"));
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai create_serializes_full_request_and_deserializes_response 2>&1 | tail -40`
Expected: FAIL to compile — `crate::interfaces::videos` module doesn't exist yet, `client.videos().create(...)` doesn't exist.

- [ ] **Step 3: Create `rs_openai/src/interfaces/videos.rs` with the full type set**

```rust
//! Sora video generation. Core CRUD + content download only this round -- input_reference,
//! characters, extensions, and edits are not yet implemented -- see
//! docs/superpowers/specs/2026-08-18-videos-design.md.

use bon::Builder;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum VideoSize {
    #[serde(rename = "1280x720")]
    S1280x720,
    #[serde(rename = "1920x1080")]
    S1920x1080,
    #[serde(rename = "1080x1920")]
    S1080x1920,
    #[serde(rename = "720x480")]
    S720x480,
    #[serde(rename = "480x720")]
    S480x720,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct CreateVideoRequest {
    #[builder(into)]
    pub model: String,

    #[builder(into)]
    pub prompt: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<VideoSize>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub seconds: Option<u32>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct VideoError {
    pub message: String,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct Video {
    pub id: String,
    pub object: String,
    pub created_at: i64,
    pub status: String,
    pub model: String,
    pub progress: Option<u32>,
    pub seconds: Option<String>,
    pub size: Option<VideoSize>,
    pub error: Option<VideoError>,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct ListVideosRequest {
    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub order: Option<crate::interfaces::chat::SortOrder>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ListVideosResponse {
    pub object: String,
    pub data: Vec<Video>,
    pub first_id: Option<String>,
    pub last_id: Option<String>,
    pub has_more: bool,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct DeleteVideoResponse {
    pub id: String,
    pub object: String,
    pub deleted: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone, strum::Display)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum VideoContentVariant {
    Thumbnail,
    Spritesheet,
}
```

- [ ] **Step 4: Register the new module**

In `rs_openai/src/interfaces/mod.rs`, add this new line as the last line in the file, after the existing `pub mod vector_stores;`:

```rust
pub mod videos;
```

- [ ] **Step 5: Implement `Videos::create`/`::list` and `Video::retrieve`/`::delete`/`::content`, remove stale `#[allow(dead_code)]`**

Replace the full contents of `rs_openai/src/apis/videos.rs` (everything above the `#[cfg(test)]` block from Step 1 — leave that test module in place) with:

```rust
//! Sora video generation -- see docs/superpowers/specs/2026-08-18-videos-design.md.

use crate::client::OpenAI;
use crate::interfaces::videos;
pub use crate::interfaces::videos::*;
use crate::shared::response_wrapper::OpenAIResponse;

pub struct Videos<'a> {
    openai: &'a OpenAI,
}

impl<'a> Videos<'a> {
    pub(crate) fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }

    /// Creates a video generation job.
    pub async fn create(&self, req: &videos::CreateVideoRequest) -> OpenAIResponse<videos::Video> {
        self.openai.post("/videos", req).await
    }

    /// Lists videos.
    pub async fn list(
        &self,
        req: &videos::ListVideosRequest,
    ) -> OpenAIResponse<videos::ListVideosResponse> {
        self.openai.get("/videos", req).await
    }
}

pub struct Video<'a> {
    openai: &'a OpenAI,
    id: String,
}

impl<'a> Video<'a> {
    pub(crate) fn new(openai: &'a OpenAI, id: String) -> Self {
        Self { openai, id }
    }

    /// Retrieves a video by ID.
    pub async fn retrieve(&self) -> OpenAIResponse<videos::Video> {
        self.openai.get(&format!("/videos/{}", self.id), &()).await
    }

    /// Deletes a video.
    pub async fn delete(&self) -> OpenAIResponse<videos::DeleteVideoResponse> {
        self.openai
            .delete(&format!("/videos/{}", self.id), &())
            .await
    }

    /// Downloads this video's raw content. `variant` selects a supporting asset
    /// (thumbnail/spritesheet) instead of the video itself; `None` downloads the video.
    pub async fn content(
        &self,
        variant: Option<videos::VideoContentVariant>,
    ) -> OpenAIResponse<Vec<u8>> {
        let route = match variant {
            Some(v) => format!("/videos/{}/content?variant={v}", self.id),
            None => format!("/videos/{}/content", self.id),
        };
        self.openai.get_raw_bytes(&route).await
    }
}
```

- [ ] **Step 6: Run test to verify it passes**

Run: `NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai create_serializes_full_request_and_deserializes_response 2>&1 | tail -30`
Expected: PASS.

- [ ] **Step 7: Write the failing test — minimal `create` proving optional fields are omitted**

Add to the same test module:

```rust
    #[tokio::test]
    async fn create_with_only_required_fields_omits_optional_ones() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/videos"))
            .and(body_json(serde_json::json!({
                "model": "sora-2",
                "prompt": "A quiet forest at dawn"
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "video_2",
                "object": "video",
                "created_at": 1758941485_i64,
                "status": "queued",
                "model": "sora-2",
                "progress": null,
                "seconds": null,
                "size": null,
                "error": null
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = CreateVideoRequest::builder()
            .model("sora-2")
            .prompt("A quiet forest at dawn")
            .build();

        let res = client.videos().create(&req).await.unwrap();
        assert_eq!(res.id, "video_2");
        assert!(res.seconds.is_none());
    }
```

- [ ] **Step 8: Run test to verify it passes**

Run: `NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai create_with_only_required_fields_omits_optional_ones 2>&1 | tail -20`
Expected: PASS.

- [ ] **Step 9: Write the failing tests for `list`, `retrieve`, `delete`, and `content`**

Add to the same test module:

```rust
    #[tokio::test]
    async fn list_serializes_query_params_and_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/videos"))
            .and(query_param("limit", "5"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "object": "list",
                "data": [{
                    "id": "video_3",
                    "object": "video",
                    "created_at": 1758941485_i64,
                    "status": "completed",
                    "model": "sora-2",
                    "progress": 100,
                    "seconds": "8",
                    "size": "1280x720",
                    "error": null
                }],
                "first_id": "video_3",
                "last_id": "video_3",
                "has_more": false
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = ListVideosRequest::builder().limit(5).build();
        let res = client.videos().list(&req).await.unwrap();
        assert_eq!(res.data.len(), 1);
        assert_eq!(res.data[0].id, "video_3");
    }

    #[tokio::test]
    async fn retrieve_deserializes_video() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/videos/video_4"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "video_4",
                "object": "video",
                "created_at": 1758941485_i64,
                "status": "failed",
                "model": "sora-2",
                "progress": null,
                "seconds": null,
                "size": null,
                "error": {"message": "Content policy violation"}
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let res = client.video("video_4").retrieve().await.unwrap();
        assert_eq!(res.status, "failed");
        assert_eq!(
            res.error.as_ref().map(|e| e.message.as_str()),
            Some("Content policy violation")
        );
    }

    #[tokio::test]
    async fn delete_deserializes_confirmation() {
        let mock_server = MockServer::start().await;

        Mock::given(method("DELETE"))
            .and(path("/videos/video_4"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "video_4",
                "object": "video.deleted",
                "deleted": true
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let res = client.video("video_4").delete().await.unwrap();
        assert!(res.deleted);
    }

    #[tokio::test]
    async fn content_downloads_raw_bytes_with_no_variant() {
        let mock_server = MockServer::start().await;
        let video_bytes: Vec<u8> = vec![0x00, 0x00, 0x00, 0x18, 0x66, 0x74, 0x79, 0x70];

        Mock::given(method("GET"))
            .and(path("/videos/video_5/content"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(video_bytes.clone()))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let res = client.video("video_5").content(None).await.unwrap();
        assert_eq!(res, video_bytes);
    }

    #[tokio::test]
    async fn content_appends_variant_query_param() {
        let mock_server = MockServer::start().await;
        let thumbnail_bytes: Vec<u8> = vec![0xFF, 0xD8, 0xFF, 0xE0];

        Mock::given(method("GET"))
            .and(path("/videos/video_5/content"))
            .and(query_param("variant", "thumbnail"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(thumbnail_bytes.clone()))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let res = client
            .video("video_5")
            .content(Some(VideoContentVariant::Thumbnail))
            .await
            .unwrap();
        assert_eq!(res, thumbnail_bytes);
    }
```

- [ ] **Step 10: Run tests to verify they pass**

Run: `NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai video 2>&1 | tail -30`
Expected: all PASS.

- [ ] **Step 11: Format, lint, full test run (also serves as this plan's final cross-check — no separate task needed given the contained size of this slice)**

Run:
```bash
cargo fmt
NO_PROXY=127.0.0.1,localhost cargo clippy -p rs_openai --all-targets -- -D warnings
NO_PROXY=127.0.0.1,localhost cargo build -p rs_openai
NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai 2>&1 | tail -10
```
Expected: clippy clean, build clean, all tests pass (179 + 7 new = 186).

Also manually verify (read the file, don't just trust the count):
- `rs_openai/src/interfaces/videos.rs` defines exactly these 8 types: `VideoSize`, `CreateVideoRequest`, `VideoError`, `Video`, `ListVideosRequest`, `ListVideosResponse`, `DeleteVideoResponse`, `VideoContentVariant`.
- No `input_reference`/`characters`/`InputReference`/`Character` fields or types exist anywhere in `interfaces/videos.rs` or `apis/videos.rs` (scope check — grep for "input_reference" and "character" case-insensitively should return zero matches).
- No `expires_at`/`remixed_from_video_id` fields exist on `Video` (scope check).
- `CreateVideoRequest.seconds: Option<u32>` and `Video.seconds: Option<String>` are genuinely different types — confirm this wasn't "fixed" into symmetry.
- `Videos`/`Video` structs have NO `#[allow(dead_code)]` (both have real methods now).

- [ ] **Step 12: Commit**

```bash
git add rs_openai/src/interfaces/mod.rs rs_openai/src/interfaces/videos.rs rs_openai/src/apis/videos.rs
git commit -m "feat(videos): add core CRUD and content download"
```
