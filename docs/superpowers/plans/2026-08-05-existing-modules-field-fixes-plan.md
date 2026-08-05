# Existing Modules: Field-Level Fixes Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Resolve every remaining field-level gap in `rs_openai`'s already-implemented modules (Fine Tuning, Audio, Embeddings, Files, Uploads, Batches, legacy Completions, Chat), including a real bug fix in Uploads, before Phase 2b moves on to implementing Phase 2a's unimplemented scaffolds.

**Architecture:** Twelve module-focused tasks plus a final cross-check task. Two new shared types (`ExpiresAfter`, `FilePurpose`, `StreamOptions`) go in `shared/types.rs` first since Files/Uploads/Batches/Completions/Chat all consume them. Fine Tuning's Grader union is built before its Method field (which references `Grader`). Everything else is independent and can be built in any order.

**Tech Stack:** Same as the rest of the crate (Rust edition 2024, `bon` builders, `serde`, `strum::Display`, `wiremock` tests) — no new dependencies.

## Global Constraints

- Spec: `docs/superpowers/specs/2026-08-05-existing-modules-field-fixes-design.md`. Background research: `docs/superpowers/research/2026-07-31-field-diff-chat-embeddings-moderations-models.md`, `docs/superpowers/research/2026-07-31-field-diff-images-audio-files-fine-tuning.md` (trust the spec over these where they differ — several items were live-verified/corrected during design).
- **Do not touch**: Audio's `voice_consents`/`voices` scaffolds, Fine Tuning's `checkpoints`/`permissions` scaffold, Audio transcription streaming, Chat's multimodal message content or `Developer` role, custom voice-object references, Realtime, the Assistants cluster. All explicitly out of scope per the spec's Non-goals.
- This pass contains real, intentional breaking changes, each justified individually in its task: `Files.purpose`/`Uploads.purpose` (String → `FilePurpose` enum), `Uploads`' entire request shape (real bug fix — the crate currently can't successfully call this endpoint at all), `Batches.endpoint` (String → `BatchEndpoint` enum), `Audio::TtsModel`'s renamed variants (`Whisper1`/`Whisper1Hd` → `Tts1`/`Tts1Hd`), `Files.status`/`.status_details` removed (confirmed deprecated). Do not treat these as mistakes to avoid.
- Wherever a field becomes a closed enum from a `String`, remove `#[builder(into)]` from that field — `bon`'s `into` attribute is for `Into<String>`-style conversions and does not apply to enum types (matches the existing convention: `Role`, `ImageSize`, etc. never carry `#[builder(into)]`).
- Every task's "done" check includes `cargo build --workspace --all-targets`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace`, and `cargo fmt --all -- --check`.
- Follow TDD: write the failing test first, confirm it fails for the expected reason (compile error, since tests reference types/fields/methods that don't exist yet), then implement.
- Existing tests are edited only where a field/type they directly construct or assert on changed shape — a direct, unavoidable consequence of that same task's own mandated change, not scope creep. This has been pre-authorized consistently across every prior Phase 2b slice; do not ask before making these edits, just make them.

---

### Task 1: Shared types — `ExpiresAfter`, `FilePurpose`, `StreamOptions`

**Files:**
- Modify: `rs_openai/src/shared/types.rs`

**Interfaces:**
- Consumes: nothing (first task).
- Produces: `ExpiresAfterAnchor::CreatedAt`; `ExpiresAfter { anchor: ExpiresAfterAnchor, seconds: u32 }` (with `Builder`); `FilePurpose::{Assistants, AssistantsOutput, Batch, BatchOutput, FineTune, FineTuneResults, Vision, UserData}`; `StreamOptions { include_usage: Option<bool> }` (with `Builder`). Tasks 3 (Files), 4 (Uploads), 5 (Batches) consume `ExpiresAfter`. Tasks 3, 4 consume `FilePurpose`. Tasks 6 (Completions), 10 (Chat) consume `StreamOptions`.

- [ ] **Step 1: Write the failing tests**

Add to the bottom of `rs_openai/src/shared/types.rs` (there's no existing `#[cfg(test)]` block in this file — create one):

```rust

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expires_after_serializes_with_snake_case_anchor() {
        let value = ExpiresAfter::builder()
            .anchor(ExpiresAfterAnchor::CreatedAt)
            .seconds(3600)
            .build();

        let json = serde_json::to_value(&value).unwrap();
        assert_eq!(
            json,
            serde_json::json!({"anchor": "created_at", "seconds": 3600})
        );
    }

    #[test]
    fn file_purpose_serializes_hyphenated_variants_correctly() {
        assert_eq!(
            serde_json::to_value(FilePurpose::FineTune).unwrap(),
            serde_json::json!("fine-tune")
        );
        assert_eq!(
            serde_json::to_value(FilePurpose::FineTuneResults).unwrap(),
            serde_json::json!("fine-tune-results")
        );
        assert_eq!(
            serde_json::to_value(FilePurpose::AssistantsOutput).unwrap(),
            serde_json::json!("assistants_output")
        );
    }

    #[test]
    fn stream_options_omits_absent_include_usage() {
        let value = StreamOptions::builder().build();
        let json = serde_json::to_value(&value).unwrap();
        assert_eq!(json, serde_json::json!({}));
    }
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --workspace --lib shared::types`
Expected: FAIL with compile errors — `ExpiresAfter`, `ExpiresAfterAnchor`, `FilePurpose`, `StreamOptions` don't exist yet.

- [ ] **Step 3: Implement**

At the top of `rs_openai/src/shared/types.rs`, replace:

```rust
use serde::Serialize;
```

with:

```rust
use bon::Builder;
use serde::{Deserialize, Serialize};
```

At the end of the file (after the existing `Stop` enum, before the new `#[cfg(test)]` block added in Step 1), add:

```rust

#[derive(Debug, Serialize, Deserialize, Clone, strum::Display)]
#[serde(rename_all = "snake_case")]
pub enum ExpiresAfterAnchor {
    #[strum(serialize = "created_at")]
    CreatedAt,
}

/// When an uploaded resource should expire. `seconds` is valid from 3600 to 2,592,000 per
/// OpenAI's docs; not enforced client-side.
#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct ExpiresAfter {
    pub anchor: ExpiresAfterAnchor,
    pub seconds: u32,
}

/// The confirmed, closed set of file purposes across Files and Uploads. Files accepts all 8
/// values; Uploads accepts only `Assistants`/`Batch`/`FineTune`/`Vision`, a subset — see each
/// module's own doc comments for the exact per-endpoint validity.
#[derive(Debug, Serialize, Deserialize, Clone, strum::Display)]
#[serde(rename_all = "snake_case")]
pub enum FilePurpose {
    Assistants,
    AssistantsOutput,
    Batch,
    BatchOutput,
    #[strum(serialize = "fine-tune")]
    #[serde(rename = "fine-tune")]
    FineTune,
    #[strum(serialize = "fine-tune-results")]
    #[serde(rename = "fine-tune-results")]
    FineTuneResults,
    Vision,
    UserData,
}

/// Shared between legacy Completions and Chat — both endpoints document the identical shape.
#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct StreamOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_usage: Option<bool>,
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test --workspace --lib shared::types`
Expected: PASS — all 3 new tests green.

Run: `cargo test --workspace`
Expected: PASS — full suite green (same count as before this task plus 3 new tests).

- [ ] **Step 5: Verify**

Run: `cargo build --workspace --all-targets`
Expected: clean build.

Run: `cargo clippy --workspace --all-targets --all-features -- -D warnings`
Expected: exit 0.

Run: `cargo fmt --all -- --check`
Expected: clean (run `cargo fmt --all` first if needed).

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "$(cat <<'EOF'
feat: add shared ExpiresAfter/FilePurpose/StreamOptions types

Foundation for the Files/Uploads/Batches/Completions/Chat field-level
fixes that follow — each of these three types is genuinely reused
across multiple modules per
docs/superpowers/specs/2026-08-05-existing-modules-field-fixes-design.md,
same pattern as Images' shared ImageSize enum.
EOF
)"
```

---

### Task 2: Embeddings — `dimensions`, `encoding_format`

**Files:**
- Modify: `rs_openai/src/interfaces/embeddings.rs`
- Modify: `rs_openai/src/apis/embeddings.rs` (tests only)

**Interfaces:**
- Consumes: nothing from Task 1.
- Produces: `CreateEmbeddingRequest.dimensions: Option<u32>`, `.encoding_format: Option<EncodingFormat>`; `EncodingFormat::{Float, Base64}`. Nothing later depends on this.

- [ ] **Step 1: Write the failing test**

Add to the `#[cfg(test)] mod tests` block in `rs_openai/src/apis/embeddings.rs`. Extend the imports — replace:

```rust
    use crate::interfaces::embeddings::CreateEmbeddingRequest;
```

with:

```rust
    use crate::interfaces::embeddings::{CreateEmbeddingRequest, EncodingFormat};
```

Add this test (place it after `create_serializes_array_input_and_deserializes_response`, before `create_maps_api_error`):

```rust
    #[tokio::test]
    async fn create_serializes_dimensions_and_encoding_format() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/embeddings"))
            .and(body_json(serde_json::json!({
                "model": "text-embedding-3-small",
                "input": "hello",
                "dimensions": 256,
                "encoding_format": "base64"
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "object": "list",
                "data": [{"object": "embedding", "embedding": [0.1], "index": 0}],
                "model": "text-embedding-3-small",
                "usage": {"prompt_tokens": 1, "total_tokens": 1}
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = CreateEmbeddingRequest::builder()
            .model("text-embedding-3-small")
            .input("hello")
            .dimensions(256_u32)
            .encoding_format(EncodingFormat::Base64)
            .build();

        let res = client.embeddings().create(&req).await.unwrap();
        assert_eq!(res.data.len(), 1);
    }
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test --workspace --lib apis::embeddings::tests::create_serializes_dimensions_and_encoding_format`
Expected: FAIL with a compile error — `.dimensions(...)`/`.encoding_format(...)` builder methods and `EncodingFormat` don't exist yet.

- [ ] **Step 3: Implement**

In `rs_openai/src/interfaces/embeddings.rs`, after the existing `EmbeddingInput` enum (before `CreateEmbeddingRequest`), add:

```rust

#[derive(Debug, Serialize, Deserialize, Clone, strum::Display)]
#[serde(rename_all = "snake_case")]
pub enum EncodingFormat {
    Float,
    Base64,
}
```

Update the `use` line at the top of the file — replace:

```rust
use bon::Builder;
use serde::{Deserialize, Serialize};
```

(no change needed — `Deserialize` is already imported; this file already has everything required for the new enum.)

In `CreateEmbeddingRequest`, immediately before the closing `}` (after the existing `user` field), add:

```rust

    /// The number of dimensions the resulting output embeddings should have. Only supported
    /// on `text-embedding-3` and later models.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dimensions: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub encoding_format: Option<EncodingFormat>,
```

- [ ] **Step 4: Run the test to verify it passes**

Run: `cargo test --workspace --lib apis::embeddings`
Expected: PASS — all embeddings tests green, including the new one.

Run: `cargo test --workspace`
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
feat: add dimensions/encoding_format to CreateEmbeddingRequest

Purely additive — closes the two field gaps the field-diff research
found for Embeddings.
EOF
)"
```

---

### Task 3: Files — `purpose` enum, `expires_after`/`expires_at`, remove deprecated fields

**Files:**
- Modify: `rs_openai/src/interfaces/files.rs`
- Modify: `rs_openai/src/apis/files.rs` (tests only)

**Interfaces:**
- Consumes: `FilePurpose`, `ExpiresAfter` from Task 1.
- Produces: `UploadFileRequest.purpose: FilePurpose` (was `String`), `.expires_after: Option<ExpiresAfter>`; `FileResponse.expires_at: Option<i64>`, no `.status`/`.status_details`. Nothing later depends on this.

- [ ] **Step 1: Write the failing tests**

First, update the existing `upload_deserializes_response` test's request construction. In `rs_openai/src/apis/files.rs`, replace:

```rust
        let req = UploadFileRequest::builder()
            .file(File {
                buffer: vec![1, 2, 3],
                filename: "data.jsonl".into(),
            })
            .purpose("fine-tune")
            .build();
```

with:

```rust
        let req = UploadFileRequest::builder()
            .file(File {
                buffer: vec![1, 2, 3],
                filename: "data.jsonl".into(),
            })
            .purpose(FilePurpose::FineTune)
            .build();
```

Extend the imports line — replace:

```rust
    use crate::interfaces::files::UploadFileRequest;
```

with:

```rust
    use crate::interfaces::files::UploadFileRequest;
    use crate::shared::types::FilePurpose;
```

Then add a new test proving `expires_after` serializes and `expires_at` deserializes, right after `upload_deserializes_response`:

```rust

    #[tokio::test]
    async fn upload_serializes_expires_after_and_deserializes_expires_at() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/files"))
            .and(ContainsInBody(br#"name="expires_after[anchor]""#))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "file-456",
                "object": "file",
                "bytes": 42,
                "created_at": 1700000000_u32,
                "filename": "data.jsonl",
                "purpose": "fine-tune",
                "expires_at": 1700003600_i64
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = UploadFileRequest::builder()
            .file(File {
                buffer: vec![1, 2, 3],
                filename: "data.jsonl".into(),
            })
            .purpose(FilePurpose::FineTune)
            .expires_after(
                crate::shared::types::ExpiresAfter::builder()
                    .anchor(crate::shared::types::ExpiresAfterAnchor::CreatedAt)
                    .seconds(3600)
                    .build(),
            )
            .build();

        let res = client.files().upload(&req).await.unwrap();
        assert_eq!(res.expires_at, Some(1700003600));
    }

    struct ContainsInBody(&'static [u8]);

    impl wiremock::Match for ContainsInBody {
        fn matches(&self, request: &wiremock::Request) -> bool {
            request
                .body
                .windows(self.0.len())
                .any(|window| window == self.0)
        }
    }
```

`expires_after` is sent as nested multipart fields (`expires_after[anchor]`, `expires_after[seconds]`), matching the `timestamp_granularities[0]` array-encoding pattern already used in `apis/audio/mod.rs`'s `create_transcription_form` — the `ContainsInBody` matcher above checks for the `expires_after[anchor]` field name in the raw multipart body. This encoding is built in Step 3 below.

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --workspace --lib apis::files`
Expected: FAIL with compile errors — `FilePurpose` import doesn't resolve against the old `purpose: String`, `.expires_after(...)` builder method doesn't exist, `ExpiresAfter`/`ExpiresAfterAnchor` aren't in scope at that path yet (they are, from Task 1 — the error here is specifically about `UploadFileRequest` not having an `expires_after` field to accept it), `res.expires_at` field doesn't exist on `FileResponse`.

- [ ] **Step 3: Implement**

In `rs_openai/src/interfaces/files.rs`, replace the entire file's imports and structs:

```rust
use crate::shared::types::File;
use bon::Builder;
use serde::{Deserialize, Serialize};

#[derive(Builder, Clone, Debug, Serialize)]
pub struct UploadFileRequest {
    /// Name of the [JSON Lines](https://jsonlines.readthedocs.io/en/latest/) file to be uploaded.
    pub file: File,

    /// The intended purpose of the uploaded documents.
    pub purpose: crate::shared::types::FilePurpose,

    /// When this file should expire. If not set, the file has no expiration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_after: Option<crate::shared::types::ExpiresAfter>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct FileResponse {
    pub id: String,
    pub object: String,
    pub bytes: u64,
    pub created_at: u32,
    pub filename: String,
    pub purpose: String,
    pub expires_at: Option<i64>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct FileListResponse {
    pub data: Vec<FileResponse>,
    pub object: String,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct DeleteFileResponse {
    pub id: String,
    pub object: String,
    pub deleted: bool,
}
```

Note `FileResponse.purpose` stays `String` (not `FilePurpose`) — the response echoes back whatever purpose string OpenAI has on file, and there's no reason to force a closed-enum round-trip on a field this crate never constructs, only reads; `status`/`status_details` are gone entirely (confirmed deprecated, no replacement).

In `rs_openai/src/apis/files.rs`, update the `upload` method's multipart form building. Replace:

```rust
    pub async fn upload(
        &self,
        req: &files::UploadFileRequest,
    ) -> OpenAIResponse<files::FileResponse> {
        let file_part = reqwest::multipart::Part::stream(req.file.buffer.clone())
            .file_name(req.file.filename.clone())
            .mime_str("application/octet-stream")
            .expect("\"application/octet-stream\" is always a valid mime type");

        let form = Form::new()
            .part("file", file_part)
            .text("purpose", req.purpose.to_string());

        self.openai.post_form("/files", form).await
    }
```

with:

```rust
    pub async fn upload(
        &self,
        req: &files::UploadFileRequest,
    ) -> OpenAIResponse<files::FileResponse> {
        let file_part = reqwest::multipart::Part::stream(req.file.buffer.clone())
            .file_name(req.file.filename.clone())
            .mime_str("application/octet-stream")
            .expect("\"application/octet-stream\" is always a valid mime type");

        let mut form = Form::new()
            .part("file", file_part)
            .text("purpose", req.purpose.to_string());

        if let Some(expires_after) = req.expires_after.clone() {
            form = form
                .text("expires_after[anchor]", expires_after.anchor.to_string())
                .text("expires_after[seconds]", expires_after.seconds.to_string());
        }

        self.openai.post_form("/files", form).await
    }
```

`expires_after.anchor.to_string()` works because Task 1's `ExpiresAfterAnchor` already derives `strum::Display`.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test --workspace --lib apis::files`
Expected: PASS — all files tests green, including the new one.

Run: `cargo test --workspace`
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
feat!: Files.purpose becomes a closed enum, add expires_after, drop deprecated fields

purpose: String -> FilePurpose (shared type, now-confirmed closed
set). Adds expires_after (request) / expires_at (response). Removes
status/status_details entirely — both confirmed deprecated in
OpenAI's current schema, no replacement. See
docs/superpowers/specs/2026-08-05-existing-modules-field-fixes-design.md.
EOF
)"
```

---

### Task 4: Uploads — real bug fix (`UploadFileRequest` becomes JSON metadata, not multipart)

**Files:**
- Modify: `rs_openai/src/interfaces/uploads.rs`
- Modify: `rs_openai/src/apis/uploads.rs` (production code and tests)

**Interfaces:**
- Consumes: `FilePurpose` from Task 1.
- Produces: `UploadFileRequest { bytes: u64, filename: String, mime_type: String, purpose: FilePurpose }` (no `File` field at all — this is the fix); `UploadFileResponse.status: UploadStatus` (was `String`); `CompleteUploadResponse.file: Option<CompleteUploadFile>` (was non-`Option`). Nothing later depends on this.

This task fixes a real, previously-undiscovered bug: `Upload::upload()` currently builds a multipart form and posts it to `POST /v1/uploads`, but the real endpoint is JSON-only, expecting upload *metadata* to open a session — not a file. Every call to today's `upload()` fails against the real API. See the spec's Context section for full detail.

- [ ] **Step 1: Write the failing tests**

Replace the existing `upload_deserializes_response` test entirely — its current shape (building the request with `.file(File{...})`) can no longer compile once `UploadFileRequest` has no `file` field. In `rs_openai/src/apis/uploads.rs`, replace:

```rust
    #[tokio::test]
    async fn upload_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/uploads"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "upload_123", "object": "upload", "bytes": 1024,
                "created_at": 1700000000_i64, "filename": "data.jsonl",
                "purpose": "fine-tune", "status": "pending", "expires_at": 1700003600_i64
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = UploadFileRequest::builder()
            .file(File {
                buffer: vec![1, 2, 3],
                filename: "data.jsonl".into(),
            })
            .purpose("fine-tune")
            .build();

        let res = client.upload().upload(&req).await.unwrap();
        assert_eq!(res.id, "upload_123");
    }
```

with:

```rust
    /// Regression test for a real bug: `upload()` used to send a multipart file to
    /// `POST /uploads`, but that endpoint is JSON-only — it takes upload *metadata* to open a
    /// session, not a file (actual bytes go through the separate, already-correct
    /// `add_upload_part`). This asserts the request body is JSON with exactly the four
    /// metadata fields, proving the fix rather than just exercising a happy path that would
    /// have passed under the old, broken implementation too.
    #[tokio::test]
    async fn upload_sends_json_metadata_not_multipart() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/uploads"))
            .and(body_json(serde_json::json!({
                "bytes": 1024,
                "filename": "data.jsonl",
                "mime_type": "application/jsonl",
                "purpose": "fine-tune"
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "upload_123", "object": "upload", "bytes": 1024,
                "created_at": 1700000000_i64, "filename": "data.jsonl",
                "purpose": "fine-tune", "status": "pending", "expires_at": 1700003600_i64
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = UploadFileRequest::builder()
            .bytes(1024_u64)
            .filename("data.jsonl")
            .mime_type("application/jsonl")
            .purpose(FilePurpose::FineTune)
            .build();

        let res = client.upload().upload(&req).await.unwrap();
        assert_eq!(res.id, "upload_123");
        assert_eq!(res.status, UploadStatus::Pending);
    }
```

Extend the imports — replace:

```rust
    use crate::client::OpenAI;
    use crate::interfaces::uploads::{CompleteUploadRequest, UploadFileRequest};
    use crate::shared::response_wrapper::OpenAIError;
    use crate::shared::types::File;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};
```

with:

```rust
    use crate::client::OpenAI;
    use crate::interfaces::uploads::{CompleteUploadRequest, UploadFileRequest, UploadStatus};
    use crate::shared::response_wrapper::OpenAIError;
    use crate::shared::types::FilePurpose;
    use wiremock::matchers::{body_json, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};
```

(`File` is no longer used by this test module once `upload_sends_json_metadata_not_multipart` no longer constructs one — confirm nothing else in this file's test module still needs the `File` import before removing it; if `complete_upload_maps_api_error` doesn't use it either, which it doesn't per the current file, this removal is correct.)

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --workspace --lib apis::uploads`
Expected: FAIL with compile errors — `.bytes(...)`/`.filename(...)`/`.mime_type(...)` builder methods don't exist on `UploadFileRequest` yet (it still has `.file(...)`), `UploadStatus` doesn't exist, `res.status` isn't comparable via `assert_eq!` yet (no `PartialEq`).

- [ ] **Step 3: Implement — `interfaces/uploads.rs`**

Replace the entire file:

```rust
use bon::Builder;
use serde::{Deserialize, Serialize};

#[derive(Builder, Clone, Debug, Serialize)]
pub struct UploadFileRequest {
    /// The number of bytes in the file you are uploading.
    pub bytes: u64,

    /// The name of the file to upload.
    #[builder(into)]
    pub filename: String,

    /// The MIME type of the file.
    ///
    /// This must fall within the supported MIME types for your file purpose. See the
    /// supported MIME types for assistants and vision.
    #[builder(into)]
    pub mime_type: String,

    /// The intended purpose of the uploaded file. Only `Assistants`, `Batch`, `FineTune`, and
    /// `Vision` are valid here — a subset of the full set `FilePurpose` supports for Files.
    pub purpose: crate::shared::types::FilePurpose,

    /// When this Upload should expire. If not set, the Upload has no expiration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_after: Option<crate::shared::types::ExpiresAfter>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, strum::Display)]
#[serde(rename_all = "snake_case")]
pub enum UploadStatus {
    Pending,
    Completed,
    Cancelled,
    Expired,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct UploadFileResponse {
    pub id: String,
    pub object: String,
    pub bytes: u64,
    pub created_at: i64,
    pub filename: String,
    pub purpose: String,
    pub status: UploadStatus,
    pub expires_at: i64,
}

#[derive(Builder, Clone, Debug, Serialize)]
pub struct AddUploadPartRequest {
    /// The chunk of bytes for this Part.
    pub data: crate::shared::types::File,
}

/// The upload [Part](https://platform.openai.com/docs/api-reference/uploads/part-object) object.
#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct AddUploadPartResponse {
    pub id: String,
    pub object: String,
    pub created_at: i64,
    pub upload_id: String,
}

#[derive(Builder, Clone, Debug, Serialize)]
pub struct CompleteUploadRequest {
    /// The ordered list of Part IDs.
    pub part_ids: Vec<String>,

    /// The optional md5 checksum for the file contents to verify if the bytes uploaded matches what you expect.
    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub md5: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct CompleteUploadFile {
    pub id: String,
    pub object: String,
    pub bytes: u64,
    pub created_at: i64,
    pub filename: String,
    pub purpose: String,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct CompleteUploadResponse {
    pub id: String,
    pub object: String,
    pub bytes: u64,
    pub created_at: i64,
    pub filename: String,
    pub purpose: String,
    pub status: UploadStatus,
    pub expires_at: i64,
    /// `None` until the upload actually completes — this response object is also returned by
    /// the create/cancel endpoints, before a file exists.
    pub file: Option<CompleteUploadFile>,
}
```

Note `expires_after` is added here too (request-side) — the Uploads create endpoint accepts it identically to Files, per the spec.

- [ ] **Step 4: Implement — `apis/uploads.rs`**

Replace the `upload` method:

```rust
    pub async fn upload(
        &self,
        req: &uploads::UploadFileRequest,
    ) -> OpenAIResponse<uploads::UploadFileResponse> {
        let file_part = reqwest::multipart::Part::stream(req.file.buffer.clone())
            .file_name(req.file.filename.clone())
            .mime_str("application/octet-stream")
            .expect("\"application/octet-stream\" is always a valid mime type");

        let form = Form::new()
            .part("file", file_part)
            .text("purpose", req.purpose.to_string());

        self.openai.post_form("/uploads", form).await
    }
```

with:

```rust
    pub async fn upload(
        &self,
        req: &uploads::UploadFileRequest,
    ) -> OpenAIResponse<uploads::UploadFileResponse> {
        self.openai.post("/uploads", req).await
    }
```

Remove the now-unused `Form` import — replace:

```rust
use crate::client::OpenAI;
use crate::interfaces::uploads;
pub use crate::interfaces::uploads::*;
use crate::shared::response_wrapper::OpenAIResponse;
use reqwest::multipart::Form;
```

with:

```rust
use crate::client::OpenAI;
use crate::interfaces::uploads;
pub use crate::interfaces::uploads::*;
use crate::shared::response_wrapper::OpenAIResponse;
```

`add_upload_part` still legitimately uses multipart (it sends actual file bytes) — do not touch that method or its `reqwest::multipart::Part` usage.

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cargo test --workspace --lib apis::uploads`
Expected: PASS — both uploads tests green (the fixed `upload_sends_json_metadata_not_multipart`, and the untouched `complete_upload_maps_api_error`).

Run: `cargo test --workspace`
Expected: PASS — full suite green.

- [ ] **Step 6: Verify**

Run: `cargo build --workspace --all-targets`
Expected: clean build.

Run: `cargo clippy --workspace --all-targets --all-features -- -D warnings`
Expected: exit 0.

Run: `cargo fmt --all -- --check`
Expected: clean.

- [ ] **Step 7: Commit**

```bash
git add -A
git commit -m "$(cat <<'EOF'
fix!: Upload::upload() sends a multipart file to a JSON-only endpoint

Real bug, not just a field gap: POST /v1/uploads expects upload
metadata (bytes/filename/mime_type/purpose) as JSON to open a
session — it does not accept a file at all. Actual bytes go through
the separate POST /uploads/{id}/parts call, which was already
implemented correctly. Every call to the old upload() failed against
the real API.

UploadFileRequest no longer has a File field — it's pure metadata.
UploadFileResponse.status and CompleteUploadResponse.file also fixed
to their real shapes (closed enum; Option, since file is null until
completion). See
docs/superpowers/specs/2026-08-05-existing-modules-field-fixes-design.md.
EOF
)"
```

---

### Task 5: Batches — `endpoint` enum, `output_expires_after`

**Files:**
- Modify: `rs_openai/src/interfaces/batch.rs`
- Modify: `rs_openai/src/apis/batch.rs` (tests only)

**Interfaces:**
- Consumes: `ExpiresAfter` from Task 1.
- Produces: `CreateBatchRequest.endpoint: BatchEndpoint` (was `String`), `.output_expires_after: Option<ExpiresAfter>`; `BatchEndpoint::{Responses, ChatCompletions, Embeddings, Completions, Moderations, ImagesGenerations, ImagesEdits, Videos}`. Nothing later depends on this.

- [ ] **Step 1: Write the failing tests**

Update both existing tests' request construction and the mock's `body_json` matcher. In `rs_openai/src/apis/batch.rs`, replace:

```rust
    #[tokio::test]
    async fn create_serializes_request_and_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/batches"))
            .and(body_json(serde_json::json!({
                "input_file_id": "file-abc",
                "endpoint": "/v1/chat/completions",
                "completion_window": "24h"
            })))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(batch_response_json("batch_123", "validating")),
            )
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = CreateBatchRequest::builder()
            .input_file_id("file-abc")
            .endpoint("/v1/chat/completions")
            .completion_window("24h")
            .build();

        let res = client.batch().create(&req).await.unwrap();
        assert_eq!(res.id, "batch_123");
    }
```

with:

```rust
    #[tokio::test]
    async fn create_serializes_request_and_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/batches"))
            .and(body_json(serde_json::json!({
                "input_file_id": "file-abc",
                "endpoint": "/v1/chat/completions",
                "completion_window": "24h"
            })))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(batch_response_json("batch_123", "validating")),
            )
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = CreateBatchRequest::builder()
            .input_file_id("file-abc")
            .endpoint(BatchEndpoint::ChatCompletions)
            .completion_window("24h")
            .build();

        let res = client.batch().create(&req).await.unwrap();
        assert_eq!(res.id, "batch_123");
    }
```

Replace:

```rust
    #[tokio::test]
    async fn create_maps_api_error() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/batches"))
            .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
                "error": {
                    "message": "input_file_id is required",
                    "type": "invalid_request_error",
                    "param": "input_file_id",
                    "code": null
                }
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = CreateBatchRequest::builder()
            .input_file_id("")
            .endpoint("/v1/chat/completions")
            .completion_window("24h")
            .build();

        let err = client.batch().create(&req).await.unwrap_err();
        match err {
            OpenAIError::ApiError(e) => {
                assert_eq!(e.error.param, Some(serde_json::json!("input_file_id")));
            }
            other => panic!("expected ApiError, got {other:?}"),
        }
    }
```

with:

```rust
    #[tokio::test]
    async fn create_maps_api_error() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/batches"))
            .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
                "error": {
                    "message": "input_file_id is required",
                    "type": "invalid_request_error",
                    "param": "input_file_id",
                    "code": null
                }
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = CreateBatchRequest::builder()
            .input_file_id("")
            .endpoint(BatchEndpoint::ChatCompletions)
            .completion_window("24h")
            .build();

        let err = client.batch().create(&req).await.unwrap_err();
        match err {
            OpenAIError::ApiError(e) => {
                assert_eq!(e.error.param, Some(serde_json::json!("input_file_id")));
            }
            other => panic!("expected ApiError, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn create_serializes_videos_endpoint_and_output_expires_after() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/batches"))
            .and(body_json(serde_json::json!({
                "input_file_id": "file-vid",
                "endpoint": "/v1/videos",
                "completion_window": "24h",
                "output_expires_after": {"anchor": "created_at", "seconds": 7200}
            })))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(batch_response_json("batch_456", "validating")),
            )
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = CreateBatchRequest::builder()
            .input_file_id("file-vid")
            .endpoint(BatchEndpoint::Videos)
            .completion_window("24h")
            .output_expires_after(
                crate::shared::types::ExpiresAfter::builder()
                    .anchor(crate::shared::types::ExpiresAfterAnchor::CreatedAt)
                    .seconds(7200)
                    .build(),
            )
            .build();

        let res = client.batch().create(&req).await.unwrap();
        assert_eq!(res.id, "batch_456");
    }
```

Extend the imports — replace:

```rust
    use crate::interfaces::batch::CreateBatchRequest;
```

with:

```rust
    use crate::interfaces::batch::{BatchEndpoint, CreateBatchRequest};
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --workspace --lib apis::batch`
Expected: FAIL with compile errors — `BatchEndpoint` doesn't exist yet, `.endpoint(BatchEndpoint::ChatCompletions)` doesn't compile against the old `endpoint: String` field, `.output_expires_after(...)` builder method doesn't exist.

- [ ] **Step 3: Implement**

In `rs_openai/src/interfaces/batch.rs`, replace:

```rust
use bon::Builder;
use serde::{Deserialize, Serialize};

#[derive(Builder, Clone, Debug, Serialize)]
pub struct CreateBatchRequest {
    /// The ID of an uploaded file that contains training data. Must be uploaded with purpose `batch`.
    #[builder(into)]
    pub input_file_id: String,

    /// The endpoint to be used for all requests in the batch: `/v1/chat/completions`, `/v1/embeddings`, or `/v1/completions`.
    #[builder(into)]
    pub endpoint: String,

    /// The time frame within which the batch should be processed. Currently only `24h` is supported.
    #[builder(into)]
    pub completion_window: String,

    /// Optional custom metadata for the batch.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}
```

with:

```rust
use bon::Builder;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, strum::Display)]
pub enum BatchEndpoint {
    #[strum(serialize = "/v1/responses")]
    #[serde(rename = "/v1/responses")]
    Responses,
    #[strum(serialize = "/v1/chat/completions")]
    #[serde(rename = "/v1/chat/completions")]
    ChatCompletions,
    #[strum(serialize = "/v1/embeddings")]
    #[serde(rename = "/v1/embeddings")]
    Embeddings,
    #[strum(serialize = "/v1/completions")]
    #[serde(rename = "/v1/completions")]
    Completions,
    #[strum(serialize = "/v1/moderations")]
    #[serde(rename = "/v1/moderations")]
    Moderations,
    #[strum(serialize = "/v1/images/generations")]
    #[serde(rename = "/v1/images/generations")]
    ImagesGenerations,
    #[strum(serialize = "/v1/images/edits")]
    #[serde(rename = "/v1/images/edits")]
    ImagesEdits,
    #[strum(serialize = "/v1/videos")]
    #[serde(rename = "/v1/videos")]
    Videos,
}

#[derive(Builder, Clone, Debug, Serialize)]
pub struct CreateBatchRequest {
    /// The ID of an uploaded file that contains training data. Must be uploaded with purpose `batch`.
    #[builder(into)]
    pub input_file_id: String,

    /// The endpoint to be used for all requests in the batch.
    pub endpoint: BatchEndpoint,

    /// The time frame within which the batch should be processed. Currently only `24h` is supported.
    #[builder(into)]
    pub completion_window: String,

    /// Optional custom metadata for the batch.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,

    /// When the batch's output/error files should expire. If not set, they have no expiration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_expires_after: Option<crate::shared::types::ExpiresAfter>,
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test --workspace --lib apis::batch`
Expected: PASS — all batch tests green, including the new one.

Run: `cargo test --workspace`
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
feat!: Batches.endpoint becomes a closed enum, add output_expires_after

endpoint: String -> BatchEndpoint, now 8 values (was documented as 3
in this crate) — confirmed via live docs, includes the new /v1/videos
and /v1/responses endpoints. Adds output_expires_after (reuses the
shared ExpiresAfter type). See
docs/superpowers/specs/2026-08-05-existing-modules-field-fixes-design.md.
EOF
)"
```

---

### Task 6: Completions (legacy) — `seed`, `stream_options`, `system_fingerprint`

**Files:**
- Modify: `rs_openai/src/interfaces/completions.rs`
- Modify: `rs_openai/src/apis/completions.rs` (tests only)

**Interfaces:**
- Consumes: `StreamOptions` from Task 1.
- Produces: `CreateCompletionRequest.seed: Option<i64>`, `.stream_options: Option<StreamOptions>`; `CompletionResponse.system_fingerprint: Option<String>`. Nothing later depends on this.

- [ ] **Step 1: Write the failing test**

Add to the `#[cfg(test)] mod tests` block in `rs_openai/src/apis/completions.rs`. Extend the imports — replace:

```rust
    use crate::interfaces::completions::CreateCompletionRequest;
```

with:

```rust
    use crate::interfaces::completions::CreateCompletionRequest;
    use crate::shared::types::StreamOptions;
```

Add this test (place it after `create_serializes_request_and_deserializes_response`, before `create_rejects_stream_true`):

```rust
    #[tokio::test]
    async fn create_serializes_seed_and_stream_options_deserializes_system_fingerprint() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/completions"))
            .and(body_json(serde_json::json!({
                "model": "text-davinci-003",
                "prompt": "hi",
                "seed": 42,
                "stream_options": {"include_usage": true}
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "cmpl-456",
                "object": "text_completion",
                "created": 1700000000_u32,
                "model": "text-davinci-003",
                "system_fingerprint": "fp_abc123",
                "choices": [{"text": "hello", "index": 0, "logprobs": null, "finish_reason": "stop"}],
                "usage": {"prompt_tokens": 1, "completion_tokens": 1, "total_tokens": 2}
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = CreateCompletionRequest::builder()
            .model("text-davinci-003")
            .prompt("hi")
            .seed(42_i64)
            .stream_options(StreamOptions::builder().include_usage(true).build())
            .build();

        let res = client.completions().create(&req).await.unwrap();
        assert_eq!(res.system_fingerprint.as_deref(), Some("fp_abc123"));
    }
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test --workspace --lib apis::completions::tests::create_serializes_seed_and_stream_options_deserializes_system_fingerprint`
Expected: FAIL with a compile error — `.seed(...)`/`.stream_options(...)` builder methods don't exist yet, `res.system_fingerprint` field doesn't exist.

- [ ] **Step 3: Implement**

In `rs_openai/src/interfaces/completions.rs`, in `CreateCompletionRequest`, immediately before the closing `}` (after the existing `user` field), add:

```rust

    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<i64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream_options: Option<crate::shared::types::StreamOptions>,
```

In `CompletionResponse`, immediately before the closing `}` (after the existing `usage` field), add:

```rust
    pub system_fingerprint: Option<String>,
```

- [ ] **Step 4: Run the test to verify it passes**

Run: `cargo test --workspace --lib apis::completions`
Expected: PASS — all completions tests green, including the new one.

Run: `cargo test --workspace`
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
feat: add seed/stream_options to legacy Completions, system_fingerprint to response

Purely additive. Reuses the shared StreamOptions type from
docs/superpowers/specs/2026-08-05-existing-modules-field-fixes-design.md.
EOF
)"
```

---

### Task 7: Audio — model/voice expansion, naming fix, new request fields

**Files:**
- Modify: `rs_openai/src/interfaces/audio.rs`
- Modify: `rs_openai/src/apis/audio/mod.rs` (tests only)

**Interfaces:**
- Consumes: nothing from Task 1.
- Produces: `TtsModel::{Tts1, Tts1Hd, Gpt4oMiniTts}` (was `Whisper1`/`Whisper1Hd`); `SttModel` gains `Gpt4oTranscribe`/`Gpt4oMiniTranscribe`/`Gpt4oTranscribeDiarize`; `Voice` gains `Ash`/`Ballad`/`Coral`/`Sage`/`Verse`/`Marin`/`Cedar`; `SttResponseFormat` gains `DiarizedJson`; `CreateSpeechRequest.instructions`/`.stream_format`; `CreateTranscriptionRequest.include`/`.chunking_strategy`/`.known_speaker_names`/`.known_speaker_references`. Nothing later depends on this.

This task renames `TtsModel::Whisper1`/`Whisper1Hd` to `Tts1`/`Tts1Hd` — a pre-identified naming bug fix (the wire values `"tts-1"`/`"tts-1-hd"` are unchanged; only the misleading Rust identifiers change, since Whisper is OpenAI's speech-*to*-text family, not text-to-speech).

- [ ] **Step 1: Write the failing tests**

First, confirm no existing test needs editing for the rename itself: the only existing test, `create_transcription_sends_correctly_spelled_language_field`, uses `SttModel::Whisper1` (correctly named already, untouched by this task) — `TtsModel` isn't referenced anywhere in the current test file.

Add these tests to the `#[cfg(test)] mod tests` block in `rs_openai/src/apis/audio/mod.rs`, after the existing test. Extend the imports — replace:

```rust
    use crate::interfaces::audio::{CreateTranscriptionRequest, Language, SttModel};
```

with:

```rust
    use crate::interfaces::audio::{
        AudioStreamFormat, CreateSpeechRequest, CreateTranscriptionRequest, Language, SttModel,
        TranscriptionInclude, TtsModel, Voice,
    };
```

```rust

    #[tokio::test]
    async fn create_speech_sends_new_tts_model_and_instructions_and_stream_format() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/audio/speech"))
            .and(body_json(serde_json::json!({
                "model": "gpt-4o-mini-tts",
                "input": "hello",
                "voice": "cedar",
                "instructions": "Speak cheerfully",
                "stream_format": "sse"
            })))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(b"fake-audio-bytes".to_vec()))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = CreateSpeechRequest::builder()
            .model(TtsModel::Gpt4oMiniTts)
            .input("hello")
            .voice(Voice::Cedar)
            .instructions("Speak cheerfully")
            .stream_format(AudioStreamFormat::Sse)
            .build();

        client.audio().create_speech(&req).await.unwrap();
    }

    #[tokio::test]
    async fn create_transcription_sends_include_and_known_speaker_fields() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/audio/transcriptions"))
            .and(ContainsInBody("name=\"include[0]\""))
            .and(ContainsInBody("name=\"known_speaker_names[0]\""))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "text": "hello"
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = CreateTranscriptionRequest::builder()
            .file(File {
                buffer: vec![1, 2, 3],
                filename: "audio.mp4".into(),
            })
            .model(SttModel::Gpt4oTranscribe)
            .include(vec![TranscriptionInclude::Logprobs])
            .known_speaker_names(vec!["Alice".to_string()])
            .known_speaker_references(vec!["base64-audio-data".to_string()])
            .build();

        let res = client.audio().create_transcription(&req).await.unwrap();
        assert_eq!(res.text, "hello");
    }
```

Note: `create_transcription_form` currently loops `timestamp_granularities` with `format!("timestamp_granularities[{}]", index)` — Step 3 mirrors that exact pattern for `include`/`known_speaker_names`/`known_speaker_references`, so the `ContainsInBody("name=\"include[0]\"")`/`"name=\"known_speaker_names[0]\""` assertions above match what Step 3 will actually produce.

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --workspace --lib apis::audio`
Expected: FAIL with compile errors — `TtsModel::Gpt4oMiniTts`, `Voice::Cedar`, `AudioStreamFormat`, `.instructions(...)`, `.stream_format(...)` don't exist; `SttModel::Gpt4oTranscribe`, `TranscriptionInclude`, `.include(...)`, `.known_speaker_names(...)`, `.known_speaker_references(...)` don't exist.

- [ ] **Step 3: Implement — interface types**

In `rs_openai/src/interfaces/audio.rs`, replace:

```rust
#[derive(Debug, Serialize, Default, Clone, strum::Display)]
pub enum Voice {
    #[default]
    #[strum(serialize = "alloy")]
    Alloy,
    #[strum(serialize = "echo")]
    Echo,
    #[strum(serialize = "fable")]
    Fable,
    #[strum(serialize = "onyx")]
    Onyx,
    #[strum(serialize = "nova")]
    Nova,
    #[strum(serialize = "shimmer")]
    Shimmer,
}
```

with:

```rust
#[derive(Debug, Serialize, Default, Clone, strum::Display)]
pub enum Voice {
    #[default]
    #[strum(serialize = "alloy")]
    Alloy,
    #[strum(serialize = "ash")]
    Ash,
    #[strum(serialize = "ballad")]
    Ballad,
    #[strum(serialize = "coral")]
    Coral,
    #[strum(serialize = "echo")]
    Echo,
    #[strum(serialize = "fable")]
    Fable,
    #[strum(serialize = "onyx")]
    Onyx,
    #[strum(serialize = "nova")]
    Nova,
    #[strum(serialize = "sage")]
    Sage,
    #[strum(serialize = "shimmer")]
    Shimmer,
    #[strum(serialize = "verse")]
    Verse,
    #[strum(serialize = "marin")]
    Marin,
    #[strum(serialize = "cedar")]
    Cedar,
}
```

Replace:

```rust
#[derive(Debug, Serialize, Default, Clone, strum::Display)]
pub enum SttModel {
    #[default]
    #[strum(serialize = "whisper-1")]
    Whisper1,
}

#[derive(Debug, Serialize, Default, Clone, strum::Display)]
pub enum TtsModel {
    #[default]
    #[strum(serialize = "tts-1")]
    Whisper1,
    #[strum(serialize = "tts-1-hd")]
    Whisper1Hd,
}
```

with:

```rust
#[derive(Debug, Serialize, Default, Clone, strum::Display)]
pub enum SttModel {
    #[default]
    #[strum(serialize = "whisper-1")]
    Whisper1,
    #[strum(serialize = "gpt-4o-transcribe")]
    Gpt4oTranscribe,
    #[strum(serialize = "gpt-4o-mini-transcribe")]
    Gpt4oMiniTranscribe,
    #[strum(serialize = "gpt-4o-transcribe-diarize")]
    Gpt4oTranscribeDiarize,
}

#[derive(Debug, Serialize, Default, Clone, strum::Display)]
pub enum TtsModel {
    #[default]
    #[strum(serialize = "tts-1")]
    Tts1,
    #[strum(serialize = "tts-1-hd")]
    Tts1Hd,
    #[strum(serialize = "gpt-4o-mini-tts")]
    Gpt4oMiniTts,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone, strum::Display)]
#[serde(rename_all = "snake_case")]
pub enum AudioStreamFormat {
    #[default]
    Sse,
    Audio,
}

#[derive(Debug, Serialize, Default, Clone, strum::Display)]
#[serde(rename_all = "snake_case")]
pub enum TranscriptionInclude {
    #[default]
    Logprobs,
}
```

Replace:

```rust
#[derive(Debug, Serialize, Default, Clone, strum::Display)]
pub enum SttResponseFormat {
    #[default]
    #[strum(serialize = "json")]
    Json,
    #[strum(serialize = "text")]
    Text,
    #[strum(serialize = "srt")]
    Srt,
    #[strum(serialize = "verbose_json")]
    VerboseJson,
    #[strum(serialize = "vtt")]
    Vtt,
}
```

with:

```rust
#[derive(Debug, Serialize, Default, Clone, strum::Display)]
pub enum SttResponseFormat {
    #[default]
    #[strum(serialize = "json")]
    Json,
    #[strum(serialize = "text")]
    Text,
    #[strum(serialize = "srt")]
    Srt,
    #[strum(serialize = "verbose_json")]
    VerboseJson,
    #[strum(serialize = "vtt")]
    Vtt,
    #[strum(serialize = "diarized_json")]
    DiarizedJson,
}
```

Replace `CreateSpeechRequest`:

```rust
#[derive(Builder, Clone, Debug, Serialize)]
pub struct CreateSpeechRequest {
    /// One of the available [TTS models](https://platform.openai.com/docs/models/tts): `tts-1` or `tts-1-hd`
    pub model: TtsModel,

    /// The text to generate audio for. The maximum length is 4096 characters.
    #[builder(into)]
    pub input: String,

    /// The voice to use when generating the audio.
    pub voice: Voice,

    /// The format to audio in.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<TtsResponseFormat>, // default: mp3

    /// The speed of the generated audio. Select a value from `0.25` to `4.0`. `1.0` is the default.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speed: Option<f32>,
}
```

with:

```rust
#[derive(Builder, Clone, Debug, Serialize)]
pub struct CreateSpeechRequest {
    /// One of the available TTS models: `tts-1`, `tts-1-hd`, or `gpt-4o-mini-tts`.
    pub model: TtsModel,

    /// The text to generate audio for. The maximum length is 4096 characters.
    #[builder(into)]
    pub input: String,

    /// The voice to use when generating the audio.
    pub voice: Voice,

    /// The format to audio in.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<TtsResponseFormat>, // default: mp3

    /// The speed of the generated audio. Select a value from `0.25` to `4.0`. `1.0` is the default.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speed: Option<f32>,

    /// Control the voice's style. Does not work with `tts-1`/`tts-1-hd`.
    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instructions: Option<String>,

    /// Not supported for `tts-1`/`tts-1-hd`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream_format: Option<AudioStreamFormat>,
}
```

Replace `CreateTranscriptionRequest`:

```rust
#[derive(Builder, Clone, Debug, Serialize)]
pub struct CreateTranscriptionRequest {
    /// The audio file object (not file name) to transcribe.
    pub file: File,

    /// ID of the model to use. Only `whisper-1` is currently available.
    pub model: SttModel,

    /// The language of the input audio, in ISO-639-1 format.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<Language>,

    /// An optional text to guide the model's style or continue a previous audio segment.
    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,

    /// The format of the transcript output.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<SttResponseFormat>, // default: "json"

    /// The sampling temperature, between 0 and 1.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,

    /// The timestamp granularities to populate for this transcription.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp_granularities: Option<Vec<TimestampGranularity>>,
}
```

with:

```rust
#[derive(Builder, Clone, Debug, Serialize)]
pub struct CreateTranscriptionRequest {
    /// The audio file object (not file name) to transcribe.
    pub file: File,

    /// ID of the model to use.
    pub model: SttModel,

    /// The language of the input audio, in ISO-639-1 format.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<Language>,

    /// An optional text to guide the model's style or continue a previous audio segment.
    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,

    /// The format of the transcript output.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<SttResponseFormat>, // default: "json"

    /// The sampling temperature, between 0 and 1.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,

    /// The timestamp granularities to populate for this transcription.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp_granularities: Option<Vec<TimestampGranularity>>,

    /// Additional information to include in the transcription response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include: Option<Vec<TranscriptionInclude>>,

    /// How the audio is cut into chunks before transcribing.
    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chunking_strategy: Option<String>,

    /// Names for speakers you already know in the recording, for diarization.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub known_speaker_names: Option<Vec<String>>,

    /// Base64-encoded audio references for the speakers in `known_speaker_names`, in the same order.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub known_speaker_references: Option<Vec<String>>,
}
```

- [ ] **Step 4: Implement — multipart form building in `apis/audio/mod.rs`**

In `create_transcription_form`, add the new fields' form-encoding after the existing `timestamp_granularities` loop. Replace:

```rust
        if let Some(timestamp_granularities) = req.timestamp_granularities.clone() {
            for (index, value) in timestamp_granularities.iter().enumerate() {
                form = form.text(
                    format!("timestamp_granularities[{}]", index),
                    value.to_string(),
                );
            }
        }

        form
    }
```

with:

```rust
        if let Some(timestamp_granularities) = req.timestamp_granularities.clone() {
            for (index, value) in timestamp_granularities.iter().enumerate() {
                form = form.text(
                    format!("timestamp_granularities[{}]", index),
                    value.to_string(),
                );
            }
        }

        if let Some(include) = req.include.clone() {
            for (index, value) in include.iter().enumerate() {
                form = form.text(format!("include[{}]", index), value.to_string());
            }
        }

        if let Some(chunking_strategy) = req.chunking_strategy.clone() {
            form = form.text("chunking_strategy", chunking_strategy);
        }

        if let Some(known_speaker_names) = req.known_speaker_names.clone() {
            for (index, value) in known_speaker_names.iter().enumerate() {
                form = form.text(format!("known_speaker_names[{}]", index), value.clone());
            }
        }

        if let Some(known_speaker_references) = req.known_speaker_references.clone() {
            for (index, value) in known_speaker_references.iter().enumerate() {
                form = form.text(
                    format!("known_speaker_references[{}]", index),
                    value.clone(),
                );
            }
        }

        form
    }
```

`CreateSpeechRequest` needs no `apis/audio/mod.rs` changes — `create_speech` already serializes the whole request as JSON (`self.openai.post_with_file_response(...)`), so the two new fields serialize automatically like every other field already on that struct.

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cargo test --workspace --lib apis::audio`
Expected: PASS — all audio tests green, including the two new ones.

Run: `cargo test --workspace`
Expected: PASS — full suite green.

- [ ] **Step 6: Verify**

Run: `cargo build --workspace --all-targets`
Expected: clean build.

Run: `cargo clippy --workspace --all-targets --all-features -- -D warnings`
Expected: exit 0.

Run: `cargo fmt --all -- --check`
Expected: clean.

- [ ] **Step 7: Commit**

```bash
git add -A
git commit -m "$(cat <<'EOF'
feat!: Audio model/voice expansion, fix TtsModel naming bug, new request fields

TtsModel::Whisper1/Whisper1Hd renamed to Tts1/Tts1Hd (wire values
unchanged) — Whisper is OpenAI's speech-to-text family, these are TTS
models, a pre-identified naming bug. Adds Gpt4oMiniTts;
SttModel gains Gpt4oTranscribe/Gpt4oMiniTranscribe/
Gpt4oTranscribeDiarize; Voice gains 7 new values; SttResponseFormat
gains DiarizedJson. CreateSpeechRequest gains instructions/
stream_format; CreateTranscriptionRequest gains include/
chunking_strategy/known_speaker_names/known_speaker_references. See
docs/superpowers/specs/2026-08-05-existing-modules-field-fixes-design.md.
EOF
)"
```

---

### Task 8: Fine Tuning — Grader union

**Files:**
- Modify: `rs_openai/src/interfaces/fine_tuning.rs`
- Modify: `rs_openai/src/apis/fine_tuning/mod.rs` (tests only)

**Interfaces:**
- Consumes: nothing from earlier tasks.
- Produces: `Grader::{StringCheck(StringCheckGrader), TextSimilarity(TextSimilarityGrader), Python(PythonGrader), ScoreModel(ScoreModelGrader), LabelModel(LabelModelGrader), Multi(MultiGrader)}` and its 6 payload structs, plus `StringCheckOperation`, `TextSimilarityMetric`, `GraderMessage`, `GraderMessageRole`, `GraderSamplingParams`, `GraderReasoningEffort`. Task 9 consumes `Grader` (used as `ReinforcementMethod.grader: Grader`, a required field).

Six variants, internally tagged on `type`. `MultiGrader.graders: Vec<Grader>` is recursive — no `Box` needed, `Vec` already provides the indirection.

- [ ] **Step 1: Write the failing tests**

Add a new test module import and tests to `rs_openai/src/apis/fine_tuning/mod.rs`'s existing `#[cfg(test)] mod tests` block. Extend the imports — replace:

```rust
    use crate::client::OpenAI;
    use crate::interfaces::fine_tuning::CreateFineTuningRequest;
    use crate::shared::response_wrapper::OpenAIError;
    use wiremock::matchers::{body_json, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};
```

with:

```rust
    use crate::client::OpenAI;
    use crate::interfaces::fine_tuning::{
        CreateFineTuningRequest, Grader, GraderMessage, GraderMessageRole, MultiGrader,
        PythonGrader, ScoreModelGrader, StringCheckGrader, StringCheckOperation,
        TextSimilarityGrader, TextSimilarityMetric,
    };
    use crate::shared::response_wrapper::OpenAIError;
    use wiremock::matchers::{body_json, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};
```

Add these tests (place them after `create_maps_api_error`, at the end of the `mod tests` block) — these test `Grader`'s serialization shape directly via `serde_json::to_value`, not through a full fine-tuning-job HTTP round trip (that wiring happens in Task 9, which is what actually attaches a `Grader` to a request):

```rust

    #[test]
    fn grader_string_check_serializes_with_type_tag() {
        let grader = Grader::StringCheck(StringCheckGrader {
            name: "exact_match".to_string(),
            input: "{{sample.output}}".to_string(),
            reference: "{{item.expected}}".to_string(),
            operation: StringCheckOperation::Eq,
        });

        let json = serde_json::to_value(&grader).unwrap();
        assert_eq!(
            json,
            serde_json::json!({
                "type": "string_check",
                "name": "exact_match",
                "input": "{{sample.output}}",
                "reference": "{{item.expected}}",
                "operation": "eq"
            })
        );
    }

    #[test]
    fn grader_text_similarity_serializes_numbered_rouge_variant_correctly() {
        let grader = Grader::TextSimilarity(TextSimilarityGrader {
            name: "similarity".to_string(),
            input: "{{sample.output}}".to_string(),
            reference: "{{item.expected}}".to_string(),
            evaluation_metric: TextSimilarityMetric::Rouge1,
        });

        let json = serde_json::to_value(&grader).unwrap();
        assert_eq!(json["evaluation_metric"], serde_json::json!("rouge_1"));
    }

    #[test]
    fn grader_python_serializes_optional_image_tag_as_absent() {
        let grader = Grader::Python(PythonGrader {
            name: "custom".to_string(),
            source: "def grade(sample, item): return 1.0".to_string(),
            image_tag: None,
        });

        let json = serde_json::to_value(&grader).unwrap();
        assert_eq!(
            json,
            serde_json::json!({
                "type": "python",
                "name": "custom",
                "source": "def grade(sample, item): return 1.0"
            })
        );
    }

    #[test]
    fn grader_score_model_deserializes_with_sampling_params() {
        let json = serde_json::json!({
            "type": "score_model",
            "name": "scorer",
            "model": "gpt-4o-mini",
            "input": [{"role": "user", "content": "grade this"}],
            "sampling_params": {"temperature": 0.0}
        });

        let grader: Grader = serde_json::from_value(json).unwrap();
        match grader {
            Grader::ScoreModel(ScoreModelGrader {
                model,
                input,
                sampling_params,
                ..
            }) => {
                assert_eq!(model, "gpt-4o-mini");
                assert_eq!(input[0].role, GraderMessageRole::User);
                assert_eq!(
                    sampling_params.unwrap().temperature,
                    Some(0.0)
                );
            }
            other => panic!("expected ScoreModel, got {other:?}"),
        }
    }

    #[test]
    fn grader_multi_recursively_holds_other_graders() {
        let grader = Grader::Multi(MultiGrader {
            name: "combined".to_string(),
            graders: vec![
                Grader::StringCheck(StringCheckGrader {
                    name: "a".to_string(),
                    input: "x".to_string(),
                    reference: "y".to_string(),
                    operation: StringCheckOperation::Eq,
                }),
                Grader::Python(PythonGrader {
                    name: "b".to_string(),
                    source: "...".to_string(),
                    image_tag: None,
                }),
            ],
            calculate_output: "0.5 * a + 0.5 * b".to_string(),
        });

        let json = serde_json::to_value(&grader).unwrap();
        assert_eq!(json["graders"].as_array().unwrap().len(), 2);
        assert_eq!(json["graders"][0]["type"], serde_json::json!("string_check"));
        assert_eq!(json["graders"][1]["type"], serde_json::json!("python"));
    }
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --workspace --lib apis::fine_tuning`
Expected: FAIL with compile errors — `Grader` and every grader payload type don't exist yet.

- [ ] **Step 3: Implement**

In `rs_openai/src/interfaces/fine_tuning.rs`, add these types after the existing `Checkpoint`/`FineTuningCheckpointResponse` structs, at the end of the file:

```rust

#[derive(Debug, Serialize, Deserialize, Clone, strum::Display)]
#[serde(rename_all = "snake_case")]
pub enum StringCheckOperation {
    Eq,
    Ne,
    Like,
    Ilike,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StringCheckGrader {
    pub name: String,
    pub input: String,
    pub reference: String,
    pub operation: StringCheckOperation,
}

#[derive(Debug, Serialize, Deserialize, Clone, strum::Display)]
#[serde(rename_all = "snake_case")]
pub enum TextSimilarityMetric {
    Cosine,
    FuzzyMatch,
    Bleu,
    Gleu,
    Meteor,
    #[strum(serialize = "rouge_1")]
    #[serde(rename = "rouge_1")]
    Rouge1,
    #[strum(serialize = "rouge_2")]
    #[serde(rename = "rouge_2")]
    Rouge2,
    #[strum(serialize = "rouge_3")]
    #[serde(rename = "rouge_3")]
    Rouge3,
    #[strum(serialize = "rouge_4")]
    #[serde(rename = "rouge_4")]
    Rouge4,
    #[strum(serialize = "rouge_5")]
    #[serde(rename = "rouge_5")]
    Rouge5,
    RougeL,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TextSimilarityGrader {
    pub name: String,
    pub input: String,
    pub reference: String,
    pub evaluation_metric: TextSimilarityMetric,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PythonGrader {
    pub name: String,
    pub source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_tag: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, strum::Display)]
#[serde(rename_all = "snake_case")]
pub enum GraderMessageRole {
    User,
    Assistant,
    System,
    Developer,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GraderMessage {
    pub role: GraderMessageRole,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, strum::Display)]
#[serde(rename_all = "snake_case")]
pub enum GraderReasoningEffort {
    None,
    Minimal,
    Low,
    Medium,
    High,
    Xhigh,
    Max,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GraderSamplingParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_completions_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_effort: Option<GraderReasoningEffort>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ScoreModelGrader {
    pub name: String,
    pub model: String,
    pub input: Vec<GraderMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub range: Option<[f32; 2]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sampling_params: Option<GraderSamplingParams>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LabelModelGrader {
    pub name: String,
    pub model: String,
    pub input: Vec<GraderMessage>,
    pub labels: Vec<String>,
    pub passing_labels: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MultiGrader {
    pub name: String,
    pub graders: Vec<Grader>,
    pub calculate_output: String,
}

/// A grading strategy for reinforcement fine-tuning. Six variants, internally tagged on
/// `type`. `Multi` recursively holds other graders via `Vec<Grader>` (no `Box` needed — `Vec`
/// already provides the indirection).
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Grader {
    StringCheck(StringCheckGrader),
    TextSimilarity(TextSimilarityGrader),
    Python(PythonGrader),
    ScoreModel(ScoreModelGrader),
    LabelModel(LabelModelGrader),
    Multi(MultiGrader),
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test --workspace --lib apis::fine_tuning`
Expected: PASS — all fine_tuning tests green, including the 5 new ones.

Run: `cargo test --workspace`
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
feat: add the Grader union for reinforcement fine-tuning

Six internally-tagged variants (StringCheck/TextSimilarity/Python/
ScoreModel/LabelModel/Multi); Multi recursively holds other graders
via Vec<Grader>. Standalone — Task 9 wires this into
CreateFineTuningRequest.method's reinforcement configuration. See
docs/superpowers/specs/2026-08-05-existing-modules-field-fixes-design.md.
EOF
)"
```

---

### Task 9: Fine Tuning — `method` field, response additions

**Files:**
- Modify: `rs_openai/src/interfaces/fine_tuning.rs`
- Modify: `rs_openai/src/apis/fine_tuning/mod.rs` (tests only)

**Interfaces:**
- Consumes: `Grader` from Task 8 (used as `ReinforcementMethod.grader: Grader`, required).
- Produces: `CreateFineTuningRequest.method: Option<FineTuningMethod>`, `.metadata: Option<HashMap<String, String>>`; `FineTuningMethod`/`FineTuningMethodType`/`SupervisedMethod`/`SupervisedHyperparameters`/`DpoMethod`/`DpoHyperparameters`/`ReinforcementMethod`/`ReinforcementHyperparameters`/`FineTuningReasoningEffort`; `FineTuningResponse.method`/`.metadata`/`.error: Option<FineTuningError>`. Nothing later depends on this.

`FineTuningMethod` is a flat struct with three optional sibling objects (`supervised`/`dpo`/`reinforcement`), not a Rust `enum` — this mirrors the real wire shape exactly (`{"type": "dpo", "dpo": {...}}`, `type` as a plain sibling field) and needs no custom serde tag/content handling.

- [ ] **Step 1: Write the failing tests**

Extend the imports in `rs_openai/src/apis/fine_tuning/mod.rs`'s test module — replace:

```rust
    use crate::interfaces::fine_tuning::{
        CreateFineTuningRequest, Grader, GraderMessage, GraderMessageRole, MultiGrader,
        PythonGrader, ScoreModelGrader, StringCheckGrader, StringCheckOperation,
        TextSimilarityGrader, TextSimilarityMetric,
    };
```

with:

```rust
    use crate::interfaces::fine_tuning::{
        CreateFineTuningRequest, DpoHyperparameters, DpoMethod, FineTuningMethod,
        FineTuningMethodType, Grader, GraderMessage, GraderMessageRole, MultiGrader,
        PythonGrader, ReinforcementMethod, ScoreModelGrader,
        StringCheckGrader, StringCheckOperation, TextSimilarityGrader, TextSimilarityMetric,
    };
```

Add these tests (place them after the 5 grader tests from Task 8, at the end of the `mod tests` block):

```rust

    #[tokio::test]
    async fn create_serializes_dpo_method_and_metadata() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/fine-tuning/jobs"))
            .and(body_json(serde_json::json!({
                "model": "gpt-4o-mini",
                "training_file": "file-abc",
                "method": {
                    "type": "dpo",
                    "dpo": {"hyperparameters": {"beta": "auto"}}
                },
                "metadata": {"team": "eval"}
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "object": "fine_tuning.job",
                "id": "ftjob-dpo",
                "model": "gpt-4o-mini",
                "created_at": 1700000000_u64,
                "finished_at": null,
                "fine_tuned_model": null,
                "organization_id": "org-123",
                "result_files": [],
                "status": "queued",
                "validation_file": null,
                "training_file": "file-abc",
                "trained_tokens": null,
                "hyperparameters": {"batch_size": "auto", "learning_rate_multiplier": "auto", "n_epochs": "auto"},
                "integrations": null,
                "seed": 42,
                "estimated_finish": null,
                "method": {"type": "dpo", "dpo": {"hyperparameters": {"beta": "auto"}}},
                "metadata": {"team": "eval"},
                "error": null
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let mut metadata = std::collections::HashMap::new();
        metadata.insert("team".to_string(), "eval".to_string());

        let req = CreateFineTuningRequest::builder()
            .model("gpt-4o-mini")
            .training_file("file-abc")
            .method(FineTuningMethod {
                r#type: FineTuningMethodType::Dpo,
                supervised: None,
                dpo: Some(DpoMethod {
                    hyperparameters: Some(DpoHyperparameters {
                        batch_size: None,
                        beta: Some(crate::interfaces::fine_tuning::AutoOrInteger::Auto(
                            "auto".to_string(),
                        )),
                        learning_rate_multiplier: None,
                        n_epochs: None,
                    }),
                }),
                reinforcement: None,
            })
            .metadata(metadata)
            .build();

        let res = client.fine_tuning().create(&req).await.unwrap();
        assert_eq!(res.id, "ftjob-dpo");
        assert!(res.error.is_none());
        assert_eq!(
            res.metadata.unwrap().get("team").map(String::as_str),
            Some("eval")
        );
    }

    #[tokio::test]
    async fn create_serializes_reinforcement_method_with_grader() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/fine-tuning/jobs"))
            .and(body_json(serde_json::json!({
                "model": "gpt-4o-mini",
                "training_file": "file-abc",
                "method": {
                    "type": "reinforcement",
                    "reinforcement": {
                        "grader": {
                            "type": "string_check",
                            "name": "exact",
                            "input": "{{sample.output}}",
                            "reference": "{{item.expected}}",
                            "operation": "eq"
                        }
                    }
                }
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "object": "fine_tuning.job",
                "id": "ftjob-rl",
                "model": "gpt-4o-mini",
                "created_at": 1700000000_u64,
                "finished_at": null,
                "fine_tuned_model": null,
                "organization_id": "org-123",
                "result_files": [],
                "status": "failed",
                "validation_file": null,
                "training_file": "file-abc",
                "trained_tokens": null,
                "hyperparameters": {"batch_size": "auto", "learning_rate_multiplier": "auto", "n_epochs": "auto"},
                "integrations": null,
                "seed": 42,
                "estimated_finish": null,
                "error": {"code": "invalid_grader", "message": "grader misconfigured", "param": "method.reinforcement.grader"}
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = CreateFineTuningRequest::builder()
            .model("gpt-4o-mini")
            .training_file("file-abc")
            .method(FineTuningMethod {
                r#type: FineTuningMethodType::Reinforcement,
                supervised: None,
                dpo: None,
                reinforcement: Some(ReinforcementMethod {
                    grader: Grader::StringCheck(StringCheckGrader {
                        name: "exact".to_string(),
                        input: "{{sample.output}}".to_string(),
                        reference: "{{item.expected}}".to_string(),
                        operation: StringCheckOperation::Eq,
                    }),
                    hyperparameters: None,
                }),
            })
            .build();

        let res = client.fine_tuning().create(&req).await.unwrap();
        let error = res.error.expect("expected error to be present");
        assert_eq!(error.code, "invalid_grader");
        assert_eq!(error.param.as_deref(), Some("method.reinforcement.grader"));
    }
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --workspace --lib apis::fine_tuning`
Expected: FAIL with compile errors — `FineTuningMethod`, `FineTuningMethodType`, `DpoMethod`, `DpoHyperparameters`, `ReinforcementMethod` don't exist yet; `.method(...)`/`.metadata(...)` builder methods don't exist on `CreateFineTuningRequest`; `res.method`/`res.metadata`/`res.error` don't exist on `FineTuningResponse`.

- [ ] **Step 3: Implement**

In `rs_openai/src/interfaces/fine_tuning.rs`, add `use std::collections::HashMap;` to the top imports — replace:

```rust
use bon::Builder;
use serde::{Deserialize, Serialize};
```

with:

```rust
use bon::Builder;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
```

In `CreateFineTuningRequest`, immediately before the closing `}` (after the existing `seed` field), add:

```rust

    /// The method used for fine-tuning. Supersedes `hyperparameters` — if both are set,
    /// `method` takes precedence.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<FineTuningMethod>,

    /// Set of up to 16 key-value pairs for structured metadata storage. Keys up to 64
    /// characters, values up to 512 characters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,
```

In `FineTuningResponse`, immediately before the closing `}` (after the existing `estimated_finish` field), add:

```rust
    /// The method used for this fine-tuning job.
    pub method: Option<FineTuningMethod>,
    pub metadata: Option<HashMap<String, String>>,
    /// Present when `status` is `"failed"`.
    pub error: Option<FineTuningError>,
```

At the end of the file, after the `Grader` enum added in Task 8 (before its own `#[cfg(test)]` block, if this file has one — it doesn't currently, tests live in `apis/fine_tuning/mod.rs` for this module), add:

```rust

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FineTuningError {
    pub code: String,
    pub message: String,
    pub param: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, strum::Display)]
#[serde(rename_all = "snake_case")]
pub enum FineTuningMethodType {
    Supervised,
    Dpo,
    Reinforcement,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SupervisedHyperparameters {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub batch_size: Option<AutoOrInteger>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub learning_rate_multiplier: Option<AutoOrInteger>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub n_epochs: Option<AutoOrInteger>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SupervisedMethod {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hyperparameters: Option<SupervisedHyperparameters>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DpoHyperparameters {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub batch_size: Option<AutoOrInteger>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub beta: Option<AutoOrInteger>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub learning_rate_multiplier: Option<AutoOrInteger>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub n_epochs: Option<AutoOrInteger>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DpoMethod {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hyperparameters: Option<DpoHyperparameters>,
}

#[derive(Debug, Serialize, Deserialize, Clone, strum::Display)]
#[serde(rename_all = "snake_case")]
pub enum FineTuningReasoningEffort {
    Default,
    Low,
    Medium,
    High,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ReinforcementHyperparameters {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub batch_size: Option<AutoOrInteger>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compute_multiplier: Option<AutoOrInteger>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eval_interval: Option<AutoOrInteger>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eval_samples: Option<AutoOrInteger>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub learning_rate_multiplier: Option<AutoOrInteger>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub n_epochs: Option<AutoOrInteger>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_effort: Option<FineTuningReasoningEffort>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ReinforcementMethod {
    pub grader: Grader,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hyperparameters: Option<ReinforcementHyperparameters>,
}

/// A flat struct with three optional sibling objects — mirrors the real wire shape exactly
/// (`{"type": "dpo", "dpo": {...}}`), not a Rust `enum`, so it needs no custom serde
/// tag/content handling.
#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct FineTuningMethod {
    pub r#type: FineTuningMethodType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supervised: Option<SupervisedMethod>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dpo: Option<DpoMethod>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reinforcement: Option<ReinforcementMethod>,
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test --workspace --lib apis::fine_tuning`
Expected: PASS — all fine_tuning tests green, including the 2 new ones.

Run: `cargo test --workspace`
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
feat: add CreateFineTuningRequest.method (supervised/dpo/reinforcement)

Fills the crate's single biggest fine-tuning gap — until now only
implicit supervised fine-tuning was reachable at all. method is a
flat struct with three optional sibling objects (type, supervised,
dpo, reinforcement), mirroring the real wire shape exactly, wired to
Task 8's Grader union for the reinforcement case. Also adds
metadata (request+response) and response-side error. hyperparameters
stays (OpenAI marks it deprecated in favor of method, but it's still
live — no removal). See
docs/superpowers/specs/2026-08-05-existing-modules-field-fixes-design.md.
EOF
)"
```

---

### Task 10: Chat — new request fields

**Files:**
- Modify: `rs_openai/src/interfaces/chat.rs`
- Modify: `rs_openai/src/apis/chat.rs` (tests only)

**Interfaces:**
- Consumes: `StreamOptions` from Task 1.
- Produces: `CreateChatRequest.response_format: Option<ChatResponseFormat>`, `.reasoning_effort: Option<ChatReasoningEffort>`, `.service_tier: Option<ServiceTier>`, `.stream_options: Option<StreamOptions>`, `.seed: Option<i64>`, `.logprobs: Option<bool>`, `.top_logprobs: Option<u8>`, `.store: Option<bool>`, `.metadata: Option<HashMap<String,String>>`, `.max_completion_tokens: Option<u32>`, `.safety_identifier: Option<String>`, `.prompt_cache_key: Option<String>`; `ChatResponseFormat::{Text, JsonObject, JsonSchema { json_schema: JsonSchemaFormat }}`; `JsonSchemaFormat`; `ChatReasoningEffort::{None, Minimal, Low, Medium, High, Xhigh, Max}`; `ServiceTier::{Auto, Default, Flex, Scale, Priority, Fast}`. Tasks 11, 12 don't depend on this task's new types — they touch different parts of `chat.rs`.

- [ ] **Step 1: Write the failing tests**

Extend the imports in `rs_openai/src/apis/chat.rs`'s test module — replace:

```rust
    use crate::interfaces::chat::{
        ChatCompletionMessage, ChatCompletionTool, CreateChatRequest, FunctionCall,
        FunctionDefinition, Role, ToolCall, ToolChoice, ToolChoiceMode,
    };
```

with:

```rust
    use crate::interfaces::chat::{
        ChatCompletionMessage, ChatCompletionTool, ChatReasoningEffort, ChatResponseFormat,
        CreateChatRequest, FunctionCall, FunctionDefinition, JsonSchemaFormat, Role,
        ServiceTier, ToolCall, ToolChoice, ToolChoiceMode,
    };
    use crate::shared::types::StreamOptions;
```

Add these tests (place them after `create_with_stream_yields_tool_call_deltas`, at the end of the `mod tests` block):

```rust

    #[tokio::test]
    async fn create_serializes_json_schema_response_format_and_reasoning_fields() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .and(body_json(serde_json::json!({
                "model": "gpt-5-mini",
                "messages": [{"role": "user", "content": "hi", "name": null, "tool_calls": null, "tool_call_id": null}],
                "response_format": {
                    "type": "json_schema",
                    "json_schema": {"name": "answer", "schema": {"type": "object"}, "strict": true}
                },
                "reasoning_effort": "low",
                "service_tier": "flex",
                "seed": 7,
                "logprobs": true,
                "top_logprobs": 3,
                "store": true,
                "max_completion_tokens": 100,
                "metadata": {"team": "eval"},
                "safety_identifier": "user-hash-123",
                "prompt_cache_key": "cache-key-abc",
                "stream_options": {"include_usage": true}
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "chatcmpl-999",
                "object": "chat.completion",
                "created": 1700000000_u32,
                "choices": [{
                    "message": {"role": "assistant", "content": "{}", "name": null, "tool_calls": null, "tool_call_id": null},
                    "finish_reason": "stop",
                    "index": 0
                }],
                "usage": {"prompt_tokens": 5, "completion_tokens": 2, "total_tokens": 7}
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let mut metadata = std::collections::HashMap::new();
        metadata.insert("team".to_string(), "eval".to_string());

        let req = CreateChatRequest::builder()
            .model("gpt-5-mini")
            .messages(vec![
                ChatCompletionMessage::builder()
                    .role(Role::User)
                    .content("hi")
                    .build(),
            ])
            .response_format(ChatResponseFormat::JsonSchema {
                json_schema: JsonSchemaFormat::builder()
                    .name("answer")
                    .schema(serde_json::json!({"type": "object"}))
                    .strict(true)
                    .build(),
            })
            .reasoning_effort(ChatReasoningEffort::Low)
            .service_tier(ServiceTier::Flex)
            .seed(7_i64)
            .logprobs(true)
            .top_logprobs(3_u8)
            .store(true)
            .max_completion_tokens(100_u32)
            .metadata(metadata)
            .safety_identifier("user-hash-123")
            .prompt_cache_key("cache-key-abc")
            .stream_options(StreamOptions::builder().include_usage(true).build())
            .build();

        let res = client.chat().create(&req).await.unwrap();
        assert_eq!(res.id, "chatcmpl-999");
    }

    #[tokio::test]
    async fn create_serializes_text_and_json_object_response_format_variants() {
        assert_eq!(
            serde_json::to_value(ChatResponseFormat::Text).unwrap(),
            serde_json::json!({"type": "text"})
        );
        assert_eq!(
            serde_json::to_value(ChatResponseFormat::JsonObject).unwrap(),
            serde_json::json!({"type": "json_object"})
        );
    }
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --workspace --lib apis::chat`
Expected: FAIL with compile errors — `ChatResponseFormat`, `ChatReasoningEffort`, `ServiceTier`, `JsonSchemaFormat` don't exist yet; none of the new builder methods exist on `CreateChatRequest`.

- [ ] **Step 3: Implement**

In `rs_openai/src/interfaces/chat.rs`, add `use std::collections::HashMap;` — this is already imported (`use std::collections::HashMap;` is present in the existing top-of-file `use` block) — confirm before adding a duplicate; if it's already there (it is, per the file's current content), skip this sub-step.

After the existing `ToolChoice` enum (before `CreateChatRequest`), add:

```rust

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct JsonSchemaFormat {
    #[builder(into)]
    pub name: String,
    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub schema: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strict: Option<bool>,
}

/// Structured-output controls. `Text`/`JsonObject` are unit variants (serialize as
/// `{"type": "text"}`/`{"type": "json_object"}` — internally-tagged enums support unit
/// variants this way); `JsonSchema` carries the actual schema.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ChatResponseFormat {
    Text,
    JsonObject,
    JsonSchema { json_schema: JsonSchemaFormat },
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum ChatReasoningEffort {
    None,
    Minimal,
    Low,
    Medium,
    High,
    Xhigh,
    Max,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum ServiceTier {
    Auto,
    Default,
    Flex,
    Scale,
    Priority,
    Fast,
}
```

In `CreateChatRequest`, immediately before the closing `}` (after the existing `parallel_tool_calls` field from the tool-calling slice), add:

```rust

    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<ChatResponseFormat>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_effort: Option<ChatReasoningEffort>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_tier: Option<ServiceTier>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream_options: Option<crate::shared::types::StreamOptions>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<i64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub logprobs: Option<bool>,

    /// 0-20. Requires `logprobs: true`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_logprobs: Option<u8>,

    /// Persist this completion for later retrieval via `Chat::retrieve`/`list`/`delete`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub store: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,

    /// Newer, reasoning-model-compatible replacement for `max_tokens`. `max_tokens` stays
    /// supported — no removal.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_completion_tokens: Option<u32>,

    /// Newer, privacy-friendlier replacement for `user`. `user` stays supported.
    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub safety_identifier: Option<String>,

    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_cache_key: Option<String>,
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test --workspace --lib apis::chat`
Expected: PASS — all chat tests green, including the 2 new ones.

Run: `cargo test --workspace`
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
feat: add remaining CreateChatRequest fields (response_format, reasoning_effort, etc.)

Closes the rest of the Chat field-diff gaps not covered by the
tool-calling slice: structured outputs (response_format),
reasoning_effort, service_tier, stream_options (reuses the shared
type), seed, logprobs/top_logprobs, store, metadata,
max_completion_tokens, safety_identifier, prompt_cache_key. All
purely additive — max_tokens and user both stay, no removals. See
docs/superpowers/specs/2026-08-05-existing-modules-field-fixes-design.md.
EOF
)"
```

---

### Task 11: Chat — response fields (`refusal`, usage token details)

**Files:**
- Modify: `rs_openai/src/interfaces/chat.rs`
- Modify: `rs_openai/src/apis/chat.rs` (tests only)

**Interfaces:**
- Consumes: nothing from Task 10 — this touches `ChatCompletionMessage`/`ChatUsage`, which Task 10 doesn't modify.
- Produces: `ChatCompletionMessage.refusal: Option<String>`; `ChatUsage.completion_tokens_details: Option<CompletionTokensDetails>`, `.prompt_tokens_details: Option<PromptTokensDetails>`; `CompletionTokensDetails`, `PromptTokensDetails`. Nothing later depends on this.

`refusal` follows `ChatCompletionMessage`'s own established always-serialize convention (no `skip_serializing_if`, matching `content`/`tool_calls`/`tool_call_id`/`name`) — internal consistency with this struct's existing fields, even though it's realistically response-only.

- [ ] **Step 1: Write the failing test**

Add to the `#[cfg(test)] mod tests` block in `rs_openai/src/apis/chat.rs` (imports from Task 10 already cover what's needed — no import changes required here). Add this test at the end of the `mod tests` block:

```rust

    #[tokio::test]
    async fn create_deserializes_refusal_and_usage_token_details() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "chatcmpl-refusal",
                "object": "chat.completion",
                "created": 1700000000_u32,
                "choices": [{
                    "message": {
                        "role": "assistant", "content": null, "refusal": "I can't help with that.",
                        "name": null, "tool_calls": null, "tool_call_id": null
                    },
                    "finish_reason": "stop",
                    "index": 0
                }],
                "usage": {
                    "prompt_tokens": 10, "completion_tokens": 5, "total_tokens": 15,
                    "completion_tokens_details": {
                        "reasoning_tokens": 2, "audio_tokens": 0,
                        "accepted_prediction_tokens": 0, "rejected_prediction_tokens": 0
                    },
                    "prompt_tokens_details": {
                        "audio_tokens": 0, "cached_tokens": 8, "cache_write_tokens": 0
                    }
                }
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = CreateChatRequest::builder()
            .model("gpt-4o-mini")
            .messages(vec![
                ChatCompletionMessage::builder()
                    .role(Role::User)
                    .content("say something you can't")
                    .build(),
            ])
            .build();

        let res = client.chat().create(&req).await.unwrap();
        assert_eq!(
            res.choices[0].message.refusal.as_deref(),
            Some("I can't help with that.")
        );

        let completion_details = res
            .usage
            .completion_tokens_details
            .as_ref()
            .expect("expected completion_tokens_details");
        assert_eq!(completion_details.reasoning_tokens, 2);

        let prompt_details = res
            .usage
            .prompt_tokens_details
            .as_ref()
            .expect("expected prompt_tokens_details");
        assert_eq!(prompt_details.cached_tokens, 8);
    }
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test --workspace --lib apis::chat::tests::create_deserializes_refusal_and_usage_token_details`
Expected: FAIL with a compile error — `message.refusal`, `usage.completion_tokens_details`, `usage.prompt_tokens_details` don't exist yet.

- [ ] **Step 3: Implement**

In `rs_openai/src/interfaces/chat.rs`, in `ChatCompletionMessage`, immediately before the closing `}` (after the existing `tool_call_id` field), add:

```rust

    /// The refusal message generated by the model, if it declined to comply with the request.
    pub refusal: Option<String>,
```

Replace `ChatUsage`:

```rust
#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ChatUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}
```

with:

```rust
#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ChatUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
    pub completion_tokens_details: Option<CompletionTokensDetails>,
    pub prompt_tokens_details: Option<PromptTokensDetails>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct CompletionTokensDetails {
    pub reasoning_tokens: u32,
    pub audio_tokens: u32,
    pub accepted_prediction_tokens: u32,
    pub rejected_prediction_tokens: u32,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct PromptTokensDetails {
    pub audio_tokens: u32,
    pub cached_tokens: u32,
    pub cache_write_tokens: u32,
}
```

Every existing test in `apis/chat.rs` that constructs a mock `usage` response body (e.g. `"usage": {"prompt_tokens": 5, "completion_tokens": 2, "total_tokens": 7}`, with no `completion_tokens_details`/`prompt_tokens_details` keys) continues to deserialize correctly without edits — both new fields are `Option`, so their absence from a mock JSON body is not an error, matching how `ChatCompletionMessage.name`/`tool_calls`/etc. already tolerate omission in older mock bodies. Every existing test that constructs a `ChatCompletionMessage` via the builder (`.role(...).content(...).build()`) also continues to compile unchanged, since `refusal` is only ever set on responses this crate *deserializes*, never on messages this crate *constructs* — `bon`'s generated builder makes every field without an explicit default effectively required only when the caller calls `.build()` on a struct meant to be fully populated by hand, but since `refusal: Option<String>` has no `#[builder(into)]`/required marker forcing a call, confirm this compiles: `Option<T>` fields in `bon`-derived builders are optional by default (do not need to be set before `.build()`), so no existing `ChatCompletionMessage::builder()...build()` call site needs a `.refusal(...)` call added.

- [ ] **Step 4: Run the test to verify it passes**

Run: `cargo test --workspace --lib apis::chat`
Expected: PASS — all chat tests green, including the new one.

Run: `cargo test --workspace`
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
feat: add refusal to ChatCompletionMessage, usage token details to ChatUsage

refusal follows this struct's own always-serialize convention
(matching content/tool_calls/tool_call_id/name), even though it's
realistically response-only. completion_tokens_details/
prompt_tokens_details are both Option, so every existing mock
response body without them still deserializes unchanged. See
docs/superpowers/specs/2026-08-05-existing-modules-field-fixes-design.md.
EOF
)"
```

---

### Task 12: Chat — new `retrieve`/`list`/`delete` operations for stored completions

**Files:**
- Modify: `rs_openai/src/interfaces/chat.rs`
- Modify: `rs_openai/src/apis/chat.rs`

**Interfaces:**
- Consumes: nothing from Tasks 10/11 — new types, new methods, no shared state.
- Produces: `Chat::retrieve(&self, completion_id: &str) -> OpenAIResponse<ChatResponse>`; `Chat::list(&self, req: &ListChatCompletionsRequest) -> OpenAIResponse<ListChatCompletionsResponse>`; `Chat::delete(&self, completion_id: &str) -> OpenAIResponse<DeleteChatCompletionResponse>`; `ListChatCompletionsRequest`, `ListChatCompletionsResponse`, `SortOrder`, `DeleteChatCompletionResponse`. Nothing later depends on this — last Chat task in this plan.

These three operations fill out gaps in the already-implemented Chat resource (gated by `store: true` at creation time, per Task 10) — not a new top-level category, so this is in scope for this pass.

- [ ] **Step 1: Write the failing tests**

Add to the `#[cfg(test)] mod tests` block in `rs_openai/src/apis/chat.rs`. Extend the imports — replace:

```rust
    use crate::interfaces::chat::{
        ChatCompletionMessage, ChatCompletionTool, ChatReasoningEffort, ChatResponseFormat,
        CreateChatRequest, FunctionCall, FunctionDefinition, JsonSchemaFormat, Role,
        ServiceTier, ToolCall, ToolChoice, ToolChoiceMode,
    };
```

with:

```rust
    use crate::interfaces::chat::{
        ChatCompletionMessage, ChatCompletionTool, ChatReasoningEffort, ChatResponseFormat,
        CreateChatRequest, FunctionCall, FunctionDefinition, JsonSchemaFormat,
        ListChatCompletionsRequest, Role, ServiceTier, SortOrder, ToolCall, ToolChoice,
        ToolChoiceMode,
    };
```

Add these tests (place them at the end of the `mod tests` block):

```rust

    #[tokio::test]
    async fn retrieve_deserializes_stored_completion() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/chat/completions/chatcmpl-stored"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "chatcmpl-stored",
                "object": "chat.completion",
                "created": 1700000000_u32,
                "choices": [{
                    "message": {"role": "assistant", "content": "hi", "name": null, "tool_calls": null, "tool_call_id": null, "refusal": null},
                    "finish_reason": "stop",
                    "index": 0
                }],
                "usage": {"prompt_tokens": 1, "completion_tokens": 1, "total_tokens": 2}
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let res = client.chat().retrieve("chatcmpl-stored").await.unwrap();
        assert_eq!(res.id, "chatcmpl-stored");
    }

    #[tokio::test]
    async fn list_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "object": "list",
                "data": [],
                "first_id": null,
                "last_id": null,
                "has_more": false
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = ListChatCompletionsRequest::builder()
            .limit(10_u32)
            .order(SortOrder::Desc)
            .build();
        let res = client.chat().list(&req).await.unwrap();
        assert!(!res.has_more);
    }

    #[tokio::test]
    async fn delete_maps_api_error() {
        let mock_server = MockServer::start().await;

        Mock::given(method("DELETE"))
            .and(path("/chat/completions/does-not-exist"))
            .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
                "error": {
                    "message": "No completion found",
                    "type": "invalid_request_error",
                    "param": null,
                    "code": "not_found"
                }
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let err = client
            .chat()
            .delete("does-not-exist")
            .await
            .unwrap_err();
        assert!(matches!(err, OpenAIError::ApiError(_)));
    }
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --workspace --lib apis::chat`
Expected: FAIL with compile errors — `ListChatCompletionsRequest`, `SortOrder` don't exist yet; `.retrieve(...)`, `.list(...)`, `.delete(...)` methods don't exist on `Chat<'a>`.

- [ ] **Step 3: Implement — `interfaces/chat.rs`**

At the end of the file, add:

```rust

#[derive(Debug, Serialize, Deserialize, Clone, strum::Display)]
#[serde(rename_all = "snake_case")]
pub enum SortOrder {
    Asc,
    Desc,
}

#[derive(Builder, Clone, Debug, Serialize)]
pub struct ListChatCompletionsRequest {
    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,

    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub order: Option<SortOrder>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ListChatCompletionsResponse {
    pub object: String,
    pub data: Vec<ChatResponse>,
    pub first_id: Option<String>,
    pub last_id: Option<String>,
    pub has_more: bool,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct DeleteChatCompletionResponse {
    pub id: String,
    pub deleted: bool,
    pub object: String,
}
```

- [ ] **Step 4: Implement — `apis/chat.rs`**

In `impl<'a> Chat<'a>`, immediately after the existing `create_with_stream` method (before the closing `}`), add:

```rust

    /// Retrieves a stored chat completion. Only completions created with `store: true`
    /// are retrievable.
    pub async fn retrieve(&self, completion_id: &str) -> OpenAIResponse<chat::ChatResponse> {
        self.openai
            .get(&format!("/chat/completions/{completion_id}"), &())
            .await
    }

    /// Lists stored chat completions.
    pub async fn list(
        &self,
        req: &chat::ListChatCompletionsRequest,
    ) -> OpenAIResponse<chat::ListChatCompletionsResponse> {
        self.openai.get("/chat/completions", req).await
    }

    /// Deletes a stored chat completion.
    pub async fn delete(
        &self,
        completion_id: &str,
    ) -> OpenAIResponse<chat::DeleteChatCompletionResponse> {
        self.openai
            .delete(&format!("/chat/completions/{completion_id}"), &())
            .await
    }
```

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cargo test --workspace --lib apis::chat`
Expected: PASS — all chat tests green, including the 3 new ones.

Run: `cargo test --workspace`
Expected: PASS — full suite green.

- [ ] **Step 6: Verify**

Run: `cargo build --workspace --all-targets`
Expected: clean build.

Run: `cargo clippy --workspace --all-targets --all-features -- -D warnings`
Expected: exit 0.

Run: `cargo fmt --all -- --check`
Expected: clean.

- [ ] **Step 7: Commit**

```bash
git add -A
git commit -m "$(cat <<'EOF'
feat: add Chat::retrieve/list/delete for stored completions

Fills out the last gap in Chat's already-implemented resource — a
completion created with store: true couldn't be retrieved, listed,
or deleted afterward. Not a new top-level category, so in scope for
this existing-module fixes pass per
docs/superpowers/specs/2026-08-05-existing-modules-field-fixes-design.md.
EOF
)"
```

---

### Task 13: Final cross-check and whole-workspace verification

**Files:**
- None modified (verification only, plus a possible trivial fix if the cross-check finds a gap).

**Interfaces:**
- Consumes: everything from Tasks 1-12.
- Produces: nothing further depends on this; it's the last task.

Each earlier task verified itself in isolation. This task verifies the *whole* branch together — in particular, whether every planned change from the spec actually landed, and whether the many tasks that touched the same file in sequence (Tasks 8+9 both touch `interfaces/fine_tuning.rs`; Tasks 10+11+12 both touch `interfaces/chat.rs`/`apis/chat.rs`) composed correctly.

- [ ] **Step 1: Cross-check every module against the spec**

Run: `grep -n "Whisper1" rs_openai/src/interfaces/audio.rs`
Expected: no output (Task 7's rename is complete — the misleadingly-named `TtsModel::Whisper1`/`Whisper1Hd` variants must not remain anywhere).

Run: `grep -rn "status_details\|pub status: String" rs_openai/src/interfaces/files.rs`
Expected: no output (Task 3 removed both deprecated fields entirely).

Run: `grep -n "reqwest::multipart::Part\|multipart::Form" rs_openai/src/apis/uploads.rs`
Expected: no output in the `upload` method specifically — `add_upload_part` legitimately still uses multipart; confirm by reading the file that only `add_upload_part` references multipart types, not `upload`.

Run: `grep -c "^pub struct\|^pub enum" rs_openai/src/interfaces/fine_tuning.rs`
Expected: a large number (the original file had roughly 10 types; Tasks 8+9 add ~20 more — confirms both tasks' additions are present, not silently dropped by a bad merge of sequential edits).

- [ ] **Step 2: Run the full verification suite on the assembled branch**

Run: `cargo fmt --all -- --check`
Expected: clean.

Run: `cargo build --workspace --all-targets`
Expected: clean build — this compiles every example crate too, confirming none of the 6 breaking changes (Files/Uploads/Batches purpose-and-endpoint enums, Uploads' whole request shape, Audio's `TtsModel` rename) broke an example. If any example fails, check `examples/files`, `examples/audio` (if either exists) for a call site using an old shape — this is the same class of fix already pre-authorized for `examples/images/src/main.rs` in the Images slice.

Run: `cargo clippy --workspace --all-targets --all-features -- -D warnings`
Expected: exit 0.

Run: `cargo test --workspace`
Expected: PASS — every test from every task, full count. Compare against the sum: Task 1 (+3), Task 2 (+1), Task 3 (+1), Task 4 (+0 net — 1 replaced), Task 5 (+1), Task 6 (+1), Task 7 (+2), Task 8 (+5), Task 9 (+2), Task 10 (+2), Task 11 (+1), Task 12 (+3) — roughly +22 tests over the pre-Task-1 baseline, since Task 4 replaces rather than adds.

- [ ] **Step 3: Done**

If Steps 1-2 all passed clean, this task made no changes — no commit needed. If any of the `grep` checks in Step 1 found a leftover reference, or Step 2's build/clippy/test/fmt found a break, that means an earlier task's own verification step should have caught it and didn't — stop and report exactly which check failed and what it found, rather than silently patching it here. The correct fix belongs in the task that owns the affected file, following that task's own established convention (e.g. a stale example call site follows the same fix pattern already used for `examples/images/src/main.rs` in the Images slice — a minimal, single-purpose edit, plus a commit describing which task's change it completes).

---
