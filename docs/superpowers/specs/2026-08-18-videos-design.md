# Videos (Sora) — Core CRUD + Content Download — Design

Status: Approved
Date: 2026-08-18

## Context

Seventh of the 9 Phase 2a scaffolds to go from skeleton to working implementation. Videos generate short clips via Sora. Phase 2a scaffolded `apis::videos::{Videos, Video}` with zero methods, anticipating a simple 2-tier CRUD resource.

Live research this session (WebFetch against `developers.openai.com`) found the real API surface has grown well beyond that assumption: alongside basic create/list/retrieve/delete/content-download, there's now `input_reference` (an optional first-frame image, sendable as multipart or JSON `file_id`/`image_url`), `characters` (reusable character references), and three entirely separate endpoints — `POST /videos/extensions` (continue an existing video), `POST /videos/edits` (single-change edit of an existing video), and `POST /videos/characters` (upload an MP4 to register a reusable character). Per `AskUserQuestion` ("Core CRUD + content download only (Recommended)"), this slice models only the original scaffold's scope — `input_reference`, `characters`, extensions, edits, and character upload are all deferred, matching the precedent already set by Containers deferring raw multipart file upload.

**A genuine request/response asymmetry, the same class of finding Evals' `data_source_config` bug taught this session to check for explicitly**: the create request's `seconds` field is documented as an `integer`, but the Video object's own JSON example shows `"seconds": "8"` — a quoted **string** in the response. This is modeled as two genuinely different types (`CreateVideoRequest.seconds: Option<u32>` vs `Video.seconds: Option<String>`), not assumed symmetric. By contrast, `size` (a `VideoSize` enum) and `model` (`String`) really are confirmed identical in both directions — both appear as the same wire values in the response example as in the request — so those ARE reused bidirectionally, same as this crate's many other confirmed-symmetric fields.

**A clean reuse of infrastructure just built for Containers**: video content download (`GET /videos/{id}/content`, with an optional `variant=thumbnail|spritesheet` query param, confirmed to return raw binary data) reuses the `get_raw_bytes` client method added in the Containers slice — no `client.rs` changes needed at all this slice.

## Goals

- **Videos**: `Videos::create()` (`POST /videos`), `Videos::list()` (`GET /videos`), `Video::retrieve()` (`GET /videos/{id}`), `Video::delete()` (`DELETE /videos/{id}`), `Video::content()` (`GET /videos/{id}/content`, optional `variant` query param, returns raw bytes via `get_raw_bytes`).
- `VideoSize` (5-value closed enum, matching `ImageSize`'s established naming/attribute style): `S1280x720`, `S1920x1080`, `S1080x1920`, `S720x480`, `S480x720`.
- `CreateVideoRequest`: `model: String` (required — kept a plain string, not an enum, matching this crate's universal convention for model-name fields), `prompt: String` (required), `size: Option<VideoSize>`, `seconds: Option<u32>` (request-side, numeric).
- `VideoError`: `message: String`.
- `Video` (response): `id: String`, `object: String`, `created_at: i64`, `status: String` (plain string despite a confirmed 4-value set — `queued`/`in_progress`/`completed`/`failed` — matching this crate's status-field uniformity convention), `model: String`, `progress: Option<u32>`, `seconds: Option<String>` (response-side, string — confirmed genuinely different from the request's numeric type), `size: Option<VideoSize>` (reused bidirectionally — confirmed identical wire values both directions), `error: Option<VideoError>`. No `expires_at`/`remixed_from_video_id` fields — not present in the confirmed response example, not guessed at.
- `ListVideosRequest`: `after: Option<String>`, `limit: Option<u32>`, `order: Option<crate::interfaces::chat::SortOrder>` (reused directly).
- `ListVideosResponse`/`DeleteVideoResponse`: the crate's universal list-response/delete-response shapes.
- `VideoContentVariant` (2-value enum, `Thumbnail`/`Spritesheet`) — this is the crate's first enum needed purely for URL query-string construction on a raw-bytes endpoint, so it derives `strum::Display` (with matching `#[strum(serialize_all)]`/`#[serde(rename_all)]`, per the crate's standing rule) specifically to support `format!("...?variant={v}")`.

## Non-goals (explicit, deferred to later slices)

- **`input_reference`** — both its multipart-raw-bytes and JSON `file_id`/`image_url` forms.
- **`characters`** on create, and the standalone `POST /videos/characters` character-upload endpoint.
- **`POST /videos/extensions`** (continue an existing video).
- **`POST /videos/edits`** (single-change edit of an existing video).
- **`expires_at`/`remixed_from_video_id`** or any other Video object field not present in the confirmed response example.

## Design

### File organization

New file `rs_openai/src/interfaces/videos.rs` (registered via `pub mod videos;` in `interfaces/mod.rs`), matching the established single-flat-file convention.

```rust
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

### `apis::videos` method wiring

```rust
impl<'a> Videos<'a> {
    pub async fn create(&self, req: &videos::CreateVideoRequest) -> OpenAIResponse<videos::Video> {
        self.openai.post("/videos", req).await
    }

    pub async fn list(&self, req: &videos::ListVideosRequest) -> OpenAIResponse<videos::ListVideosResponse> {
        self.openai.get("/videos", req).await
    }
}

impl<'a> Video<'a> {
    pub async fn retrieve(&self) -> OpenAIResponse<videos::Video> {
        self.openai.get(&format!("/videos/{}", self.id), &()).await
    }

    pub async fn delete(&self) -> OpenAIResponse<videos::DeleteVideoResponse> {
        self.openai.delete(&format!("/videos/{}", self.id), &()).await
    }

    /// Downloads this video's raw content. `variant` selects a supporting asset
    /// (thumbnail/spritesheet) instead of the video itself; `None` downloads the video.
    pub async fn content(&self, variant: Option<videos::VideoContentVariant>) -> OpenAIResponse<Vec<u8>> {
        let route = match variant {
            Some(v) => format!("/videos/{}/content?variant={v}", self.id),
            None => format!("/videos/{}/content", self.id),
        };
        self.openai.get_raw_bytes(&route).await
    }
}
```

`get_raw_bytes` is unchanged, reused as-is from the Containers slice.

### Naming collision

`interfaces::videos::Video` vs `apis::videos::Video<'a>` — the crate's now eighth instance of this established pattern, resolved via qualified paths.

## Testing

`wiremock` tests covering: `create` serializing `model`/`prompt`/`size`/`seconds` and deserializing a `Video` response (confirming `seconds` round-trips as a request-side number but a response-side string — the asymmetry this design deliberately captures); a minimal `create` test with only required fields; `list` serializing query params; `retrieve`; `delete`; `content` downloading raw bytes with no variant (proving `get_raw_bytes` reuse works for a second, unrelated resource); `content` with `variant=thumbnail` asserting the query string is correctly appended.

## Versioning

Purely additive at the crate level — new module, new types, no client method changes (`get_raw_bytes` already exists). Ships as a pre-1.0 minor bump alongside whatever else accumulates before the next release, per established practice.
