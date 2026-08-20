# Organization Usage Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement sub-slice 3 of 4 for the Organization/admin API: all 11 usage-reporting endpoints (`completions`, `embeddings`, `moderations`, `images`, `audio_speeches`, `audio_transcriptions`, `vector_stores`, `code_interpreter_sessions`, `file_search_calls`, `web_search_calls`, `costs`).

**Architecture:** One shared response envelope/result union defined once in Task 1 (`UsageResult`, `UsageBucket`, `UsageResponse`), plus one new `client.rs` method (`get_with_pairs`) working around this crate's `serde_urlencoded` backend not supporting `Vec<String>` struct fields. Each of the remaining 10 endpoints (Tasks 2-4) adds only its own request type + `to_query_pairs()` method + API method + tests — no further shared-type or `client.rs` changes needed.

**Tech Stack:** Rust, `bon::Builder`, `serde`, `wiremock`, `tokio::test` — same stack as every prior slice.

## Global Constraints

- Spec: `docs/superpowers/specs/2026-08-20-org-usage-design.md`. This is sub-slice 3 of 4 for Organization/admin; sub-slices 1-2 (Admin API Keys + Audit Logs; Users + Invites + Groups) are already shipped and merged; sub-slice 4 (Projects + its 5 nested sub-resources) is out of scope here.
- The ONE `client.rs` change in this entire plan is `get_with_pairs`, added in Task 1. Tasks 2-4 must NOT touch `client.rs` at all — they only add request types + `to_query_pairs()` methods + API methods that call the already-added `get_with_pairs`.
- The shared response types (`UsageResult`, `UsageBucket`, `UsageResponse`, and all 11 `UsageXxxResult` structs + `UsageCostsAmount`) are defined ONCE, in Task 1's `interfaces/organization/usage.rs`. Tasks 2-4 must NOT redefine, duplicate, or modify any of these — they only add new request types to the same file and reference the existing response types by name.
- `apis::organization` does NOT do a flat `pub use crate::interfaces::organization::<name>::*;` re-export. `apis/organization/usage.rs` imports `use crate::interfaces::organization::usage;` and references types via qualified paths (e.g. `usage::UsageResponse`), matching every other file in `apis/organization/`.
- `to_query_pairs(&self) -> Vec<(String, String)>` is a hand-written, per-request-type method (not a shared trait) — each field gets its own explicit `if let`/`for` block. Do not introduce a generic/trait-based abstraction for this across endpoints.
- `group_by` and every `Vec<String>` filter field stay plain `Option<Vec<String>>` (not closed enums) — valid values are documented per-field in doc comments only, not enforced by the type system. `bucket_width` stays a plain `Option<String>`, not an enum.
- Route paths: 10 of the 11 endpoints live under `/organization/usage/{name}` (using the endpoint's own name, e.g. `/organization/usage/completions`, `/organization/usage/file_search_calls`, `/organization/usage/web_search_calls`). `costs` is the one exception — its route is `/organization/costs`, NOT `/organization/usage/costs`.
- `UsageResult`'s enum variant names for `file_search_calls`/`web_search_calls` are `FileSearches`/`WebSearches` (matching the confirmed `object` tag values `organization.usage.file_searches.result`/`organization.usage.web_searches.result`) — note these differ from the endpoint/route names (`file_search_calls`/`web_search_calls`). Do not rename either side to match the other; both are independently confirmed from the source.
- Stage commits with **explicit file paths** (`git add path/to/file.rs`), never `git add -A`.
- Run `cargo fmt` and `cargo clippy -p rs_openai --all-targets -- -D warnings` before every commit; both must be clean.
- Environment: this machine's shell routes HTTP through a local proxy that breaks `wiremock` tests. Prefix every `cargo test`/`build`/`clippy` invocation with `NO_PROXY=127.0.0.1,localhost`. This is a local-shell-only concern — never write it into crate code, `Cargo.toml`, or CI config.
- Test client construction: `OpenAI::new("test-key").with_base_url(mock_server.uri())` — NOT `OpenAI::new_with_base_url(...)`, which does not exist (this exact bug was caught and fixed in sub-slice 2's plan; do not repeat it).
- Baseline: 217/217 tests passing before Task 1 starts (verified directly against the live repo, not assumed).

---

### Task 1: Foundation (shared response types + `client.rs` fix) + `completions` endpoint

**Files:**
- Modify: `rs_openai/src/interfaces/organization/mod.rs` (add `pub mod usage;`, alphabetically — insert after `roles` and before... check the current file; as of sub-slice 2 the list is `admin_api_keys, audit_logs, groups, invites, users` — `usage` sorts after `users`... no, alphabetically `usage` < `users` since `'a' < 'r'` at the 4th character (`usa` vs `use`... compare character by character: `usage` vs `users` — `u`,`s` match, then `a` vs `e` — `a` < `e`, so `usage` sorts BEFORE `users`. Insert `pub mod usage;` between `invites` and `users`.)
- Create: `rs_openai/src/interfaces/organization/usage.rs`
- Modify: `rs_openai/src/client.rs` (add `get_with_pairs` method, right after the existing `get<T, F>` method around line 248)
- Modify: `rs_openai/src/apis/organization/usage.rs` (add the `completions` method, remove stale `#[allow(dead_code)]`, update doc comment)
- Test: `rs_openai/src/apis/organization/usage.rs` (add a `#[cfg(test)] mod tests` block — none exists yet)

**Interfaces:**
- Consumes: existing `apis::organization::OrganizationUsage` struct shape from Phase 2a (`OrganizationUsage<'a> { openai: &'a OpenAI }`, unchanged); existing `client.rs` `get<T, F>`/`openai_request`/`resolve_response` (read directly to match style, do not guess signatures).
- Produces: `interfaces::organization::usage::{UsageResult, UsageBucket, UsageResponse, UsageCompletionsResult, UsageEmbeddingsResult, UsageModerationsResult, UsageImagesResult, UsageAudioSpeechesResult, UsageAudioTranscriptionsResult, UsageVectorStoresResult, UsageCodeInterpreterSessionsResult, UsageFileSearchesResult, UsageWebSearchesResult, UsageCostsResult, UsageCostsAmount, UsageCompletionsRequest}`, `client::OpenAI::get_with_pairs`, `OrganizationUsage::completions`. Tasks 2-4 depend on every response type in this list existing exactly as defined here — none of them may be redefined later.

**Test-design note (this task makes a judgment call left open by the spec):** the spec asks whether the "tagged enum dispatches to the correct variant" proof belongs in this task or a later one, once a second endpoint's result type exists. Since ALL 11 `UsageXxxResult` types are defined in THIS task (only the request types and API methods are added incrementally in Tasks 2-4), this task already has every result type it needs to construct a synthetic response containing two different variants (`Completions` and `Embeddings`) in one `results` array and assert both deserialize correctly — even though `Embeddings`' own request type/API method don't exist until Task 2. This keeps the foundation task fully self-certifying rather than splitting one conceptual proof across two tasks.

- [ ] **Step 1: Write the failing tests — full request round-trip, minimal request, and tagged-variant dispatch**

Add a test module to `rs_openai/src/apis/organization/usage.rs`:

```rust
#[cfg(test)]
mod tests {
    use crate::client::OpenAI;
    use crate::interfaces::organization::usage::UsageCompletionsRequest;
    use wiremock::matchers::{method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn completions_serializes_full_request_and_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/organization/usage/completions"))
            .and(query_param("start_time", "1700000000"))
            .and(query_param("end_time", "1700086400"))
            .and(query_param("bucket_width", "1d"))
            .and(query_param("limit", "7"))
            .and(query_param("group_by", "model"))
            .and(query_param("group_by", "project_id"))
            .and(query_param("project_ids", "proj_1"))
            .and(query_param("project_ids", "proj_2"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "object": "page",
                "has_more": false,
                "next_page": null,
                "data": [{
                    "object": "bucket",
                    "start_time": 1700000000_i64,
                    "end_time": 1700086400_i64,
                    "results": [{
                        "object": "organization.usage.completions.result",
                        "input_tokens": 100_i64,
                        "output_tokens": 50_i64,
                        "num_model_requests": 2_i64,
                        "model": "gpt-4o"
                    }]
                }]
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("test-key").with_base_url(mock_server.uri());
        let req = UsageCompletionsRequest::builder()
            .start_time(1700000000)
            .end_time(1700086400)
            .bucket_width("1d")
            .limit(7)
            .group_by(vec!["model".to_string(), "project_id".to_string()])
            .project_ids(vec!["proj_1".to_string(), "proj_2".to_string()])
            .build();
        let result = client.organization().usage().completions(&req).await.unwrap();

        assert!(!result.has_more);
        assert_eq!(result.data.len(), 1);
        assert_eq!(result.data[0].results.len(), 1);
    }

    #[tokio::test]
    async fn completions_with_only_required_field_omits_optionals() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/organization/usage/completions"))
            .and(query_param("start_time", "1700000000"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "object": "page",
                "has_more": false,
                "next_page": null,
                "data": []
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("test-key").with_base_url(mock_server.uri());
        let req = UsageCompletionsRequest::builder().start_time(1700000000).build();
        let result = client.organization().usage().completions(&req).await.unwrap();

        assert!(result.data.is_empty());
    }

    #[tokio::test]
    async fn usage_result_deserializes_into_correct_variant_by_object_tag() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/organization/usage/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "object": "page",
                "has_more": false,
                "next_page": null,
                "data": [{
                    "object": "bucket",
                    "start_time": 1700000000_i64,
                    "end_time": 1700086400_i64,
                    "results": [
                        {
                            "object": "organization.usage.completions.result",
                            "input_tokens": 100_i64,
                            "output_tokens": 50_i64,
                            "num_model_requests": 2_i64
                        },
                        {
                            "object": "organization.usage.embeddings.result",
                            "input_tokens": 30_i64,
                            "num_model_requests": 1_i64
                        }
                    ]
                }]
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("test-key").with_base_url(mock_server.uri());
        let req = UsageCompletionsRequest::builder().start_time(1700000000).build();
        let result = client.organization().usage().completions(&req).await.unwrap();

        let results = &result.data[0].results;
        assert_eq!(results.len(), 2);
        match &results[0] {
            crate::interfaces::organization::usage::UsageResult::Completions(r) => {
                assert_eq!(r.input_tokens, 100);
            }
            other => panic!("expected Completions variant, got {other:?}"),
        }
        match &results[1] {
            crate::interfaces::organization::usage::UsageResult::Embeddings(r) => {
                assert_eq!(r.input_tokens, 30);
            }
            other => panic!("expected Embeddings variant, got {other:?}"),
        }
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd rs_openai && NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai apis::organization::usage::tests`
Expected: FAIL to compile — `interfaces::organization::usage` module and `completions()` method don't exist yet.

- [ ] **Step 3: Create the shared types + `completions` request type**

Create `rs_openai/src/interfaces/organization/usage.rs`:

```rust
//! Organization usage reporting (`/organization/usage/*`, `/organization/costs`) -- see
//! docs/superpowers/specs/2026-08-20-org-usage-design.md.

use bon::Builder;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Clone, Serialize)]
#[serde(tag = "object")]
pub enum UsageResult {
    #[serde(rename = "organization.usage.completions.result")]
    Completions(UsageCompletionsResult),
    #[serde(rename = "organization.usage.embeddings.result")]
    Embeddings(UsageEmbeddingsResult),
    #[serde(rename = "organization.usage.moderations.result")]
    Moderations(UsageModerationsResult),
    #[serde(rename = "organization.usage.images.result")]
    Images(UsageImagesResult),
    #[serde(rename = "organization.usage.audio_speeches.result")]
    AudioSpeeches(UsageAudioSpeechesResult),
    #[serde(rename = "organization.usage.audio_transcriptions.result")]
    AudioTranscriptions(UsageAudioTranscriptionsResult),
    #[serde(rename = "organization.usage.vector_stores.result")]
    VectorStores(UsageVectorStoresResult),
    #[serde(rename = "organization.usage.code_interpreter_sessions.result")]
    CodeInterpreterSessions(UsageCodeInterpreterSessionsResult),
    #[serde(rename = "organization.usage.file_searches.result")]
    FileSearches(UsageFileSearchesResult),
    #[serde(rename = "organization.usage.web_searches.result")]
    WebSearches(UsageWebSearchesResult),
    #[serde(rename = "organization.costs.result")]
    Costs(UsageCostsResult),
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct UsageBucket {
    pub start_time: i64,
    pub end_time: i64,
    pub object: String,
    pub results: Vec<UsageResult>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct UsageResponse {
    pub data: Vec<UsageBucket>,
    pub has_more: bool,
    pub next_page: Option<String>,
    pub object: String,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct UsageCompletionsResult {
    pub input_tokens: i64,
    pub num_model_requests: i64,
    pub output_tokens: i64,
    pub api_key_id: Option<String>,
    pub batch: Option<bool>,
    pub input_audio_tokens: Option<i64>,
    pub input_cache_write_tokens: Option<i64>,
    pub input_cached_audio_tokens: Option<i64>,
    pub input_cached_image_tokens: Option<i64>,
    pub input_cached_text_tokens: Option<i64>,
    pub input_cached_tokens: Option<i64>,
    pub input_image_tokens: Option<i64>,
    pub input_text_tokens: Option<i64>,
    pub input_uncached_tokens: Option<i64>,
    pub model: Option<String>,
    pub output_audio_tokens: Option<i64>,
    pub output_image_tokens: Option<i64>,
    pub output_text_tokens: Option<i64>,
    pub project_id: Option<String>,
    pub service_tier: Option<String>,
    pub user_id: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct UsageEmbeddingsResult {
    pub input_tokens: i64,
    pub num_model_requests: i64,
    pub api_key_id: Option<String>,
    pub model: Option<String>,
    pub project_id: Option<String>,
    pub user_id: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct UsageModerationsResult {
    pub input_tokens: i64,
    pub num_model_requests: i64,
    pub api_key_id: Option<String>,
    pub model: Option<String>,
    pub project_id: Option<String>,
    pub user_id: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct UsageImagesResult {
    pub images: i64,
    pub num_model_requests: i64,
    pub api_key_id: Option<String>,
    pub model: Option<String>,
    pub project_id: Option<String>,
    pub size: Option<String>,
    pub source: Option<String>,
    pub user_id: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct UsageAudioSpeechesResult {
    pub characters: i64,
    pub num_model_requests: i64,
    pub api_key_id: Option<String>,
    pub model: Option<String>,
    pub project_id: Option<String>,
    pub user_id: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct UsageAudioTranscriptionsResult {
    pub num_model_requests: i64,
    pub seconds: i64,
    pub api_key_id: Option<String>,
    pub model: Option<String>,
    pub project_id: Option<String>,
    pub user_id: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct UsageVectorStoresResult {
    pub usage_bytes: i64,
    pub project_id: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct UsageCodeInterpreterSessionsResult {
    pub num_sessions: i64,
    pub project_id: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct UsageFileSearchesResult {
    pub num_requests: i64,
    pub api_key_id: Option<String>,
    pub project_id: Option<String>,
    pub user_id: Option<String>,
    pub vector_store_id: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct UsageWebSearchesResult {
    pub num_model_requests: i64,
    pub num_requests: i64,
    pub api_key_id: Option<String>,
    pub context_level: Option<String>,
    pub model: Option<String>,
    pub project_id: Option<String>,
    pub user_id: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct UsageCostsAmount {
    pub currency: Option<String>,
    pub value: Option<f64>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct UsageCostsResult {
    pub amount: Option<UsageCostsAmount>,
    pub api_key_id: Option<String>,
    pub line_item: Option<String>,
    pub project_id: Option<String>,
    pub quantity: Option<f64>,
}

/// Query params for `GET /organization/usage/completions`. `group_by` accepts any
/// combination of `project_id`, `user_id`, `api_key_id`, `model`, `batch`,
/// `service_tier`.
#[derive(Builder, Debug, Clone, Default)]
pub struct UsageCompletionsRequest {
    pub start_time: i64,
    pub end_time: Option<i64>,
    #[builder(into)]
    pub bucket_width: Option<String>,
    pub limit: Option<u32>,
    #[builder(into)]
    pub page: Option<String>,
    pub project_ids: Option<Vec<String>>,
    pub api_key_ids: Option<Vec<String>>,
    pub user_ids: Option<Vec<String>>,
    pub models: Option<Vec<String>>,
    pub batch: Option<bool>,
    pub group_by: Option<Vec<String>>,
}

impl UsageCompletionsRequest {
    pub(crate) fn to_query_pairs(&self) -> Vec<(String, String)> {
        let mut pairs = Vec::new();
        pairs.push(("start_time".to_string(), self.start_time.to_string()));
        if let Some(v) = self.end_time {
            pairs.push(("end_time".to_string(), v.to_string()));
        }
        if let Some(v) = &self.bucket_width {
            pairs.push(("bucket_width".to_string(), v.clone()));
        }
        if let Some(v) = self.limit {
            pairs.push(("limit".to_string(), v.to_string()));
        }
        if let Some(v) = &self.page {
            pairs.push(("page".to_string(), v.clone()));
        }
        if let Some(v) = self.batch {
            pairs.push(("batch".to_string(), v.to_string()));
        }
        if let Some(values) = &self.project_ids {
            for v in values {
                pairs.push(("project_ids".to_string(), v.clone()));
            }
        }
        if let Some(values) = &self.api_key_ids {
            for v in values {
                pairs.push(("api_key_ids".to_string(), v.clone()));
            }
        }
        if let Some(values) = &self.models {
            for v in values {
                pairs.push(("models".to_string(), v.clone()));
            }
        }
        if let Some(values) = &self.group_by {
            for v in values {
                pairs.push(("group_by".to_string(), v.clone()));
            }
        }
        pairs
    }
}
```

Note: `UsageCompletionsRequest` does NOT derive `Serialize`/`Deserialize` (unlike every other request type in this crate) — it is never serialized as a whole struct (only via its own `to_query_pairs()` method) and never deserialized at all. `#[derive(Builder, Debug, Clone, Default)]` only.

- [ ] **Step 4: Register the module**

In `rs_openai/src/interfaces/organization/mod.rs`, add `pub mod usage;` in alphabetical order among the existing `pub mod` lines (between `invites` and `users`).

- [ ] **Step 5: Add the `get_with_pairs` client method**

In `rs_openai/src/client.rs`, immediately after the existing `get<T, F>` method (ends around line 248, right before `pub(crate) async fn post`), add:

```rust
    pub(crate) async fn get_with_pairs<T>(
        &self,
        route: &str,
        pairs: &[(String, String)],
    ) -> OpenAIResponse<T>
    where
        T: DeserializeOwned + Debug,
    {
        let request = self.openai_request(Method::GET, route)?.query(pairs);
        Self::resolve_response(request).await
    }
```

Read the actual current `get<T, F>` method first to confirm `openai_request`/`resolve_response`'s exact names and signatures before pasting this — do not guess.

- [ ] **Step 6: Implement the `completions` method**

Replace the full contents of `rs_openai/src/apis/organization/usage.rs` with:

```rust
//! Organization usage reporting (`/organization/usage/*`, `/organization/costs`) -- see
//! docs/superpowers/specs/2026-08-20-org-usage-design.md.

use crate::client::OpenAI;
use crate::interfaces::organization::usage;
use crate::shared::response_wrapper::OpenAIResponse;

pub struct OrganizationUsage<'a> {
    openai: &'a OpenAI,
}

impl<'a> OrganizationUsage<'a> {
    pub(crate) fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }

    pub async fn completions(
        &self,
        req: &usage::UsageCompletionsRequest,
    ) -> OpenAIResponse<usage::UsageResponse> {
        self.openai
            .get_with_pairs("/organization/usage/completions", &req.to_query_pairs())
            .await
    }
}
```

- [ ] **Step 7: Run tests to verify they pass**

Run: `cd rs_openai && NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai apis::organization::usage::tests`
Expected: 3 tests PASS

- [ ] **Step 8: Update the doc comment**

In `rs_openai/src/apis/organization/mod.rs`, change:

```rust
    /// Scoped accessor for organization usage reporting. Not yet implemented.
    pub fn usage(&self) -> usage::OrganizationUsage<'a> {
        usage::OrganizationUsage::new(self.openai)
    }
```

to:

```rust
    /// Scoped accessor for organization usage reporting.
    pub fn usage(&self) -> usage::OrganizationUsage<'a> {
        usage::OrganizationUsage::new(self.openai)
    }
```

- [ ] **Step 9: Run fmt, clippy, and the full test suite**

```bash
cd rs_openai
cargo fmt
NO_PROXY=127.0.0.1,localhost cargo clippy -p rs_openai --all-targets -- -D warnings
NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai
```
Expected: fmt clean, clippy clean, 220/220 tests passing (217 baseline + 3 new).

- [ ] **Step 10: Commit**

```bash
git add rs_openai/src/interfaces/organization/mod.rs
git add rs_openai/src/interfaces/organization/usage.rs
git add rs_openai/src/client.rs
git add rs_openai/src/apis/organization/usage.rs
git add rs_openai/src/apis/organization/mod.rs
git commit -m "feat(organization): add usage foundation (shared types + get_with_pairs) and completions endpoint"
```

---

### Task 2: `embeddings`, `moderations`, `audio_speeches`, `audio_transcriptions`

**Files:**
- Modify: `rs_openai/src/interfaces/organization/usage.rs` (append 4 request types + their `to_query_pairs()` impls — do not touch anything Task 1 added)
- Modify: `rs_openai/src/apis/organization/usage.rs` (append 4 methods to `OrganizationUsage`)
- Test: `rs_openai/src/apis/organization/usage.rs` (extend the existing `#[cfg(test)] mod tests` block from Task 1)

**Interfaces:**
- Consumes: `interfaces::organization::usage::{UsageEmbeddingsResult, UsageModerationsResult, UsageAudioSpeechesResult, UsageAudioTranscriptionsResult, UsageResponse}` from Task 1 (already defined, do not redefine).
- Produces: `interfaces::organization::usage::{UsageEmbeddingsRequest, UsageModerationsRequest, UsageAudioSpeechesRequest, UsageAudioTranscriptionsRequest}`, `OrganizationUsage::{embeddings, moderations, audio_speeches, audio_transcriptions}`.

These four endpoints share an identical request field shape: `start_time` (required), `end_time`, `bucket_width`, `limit`, `page`, `project_ids`, `api_key_ids`, `models`, `group_by` (valid values: `project_id`, `user_id`, `api_key_id`, `model`) — no endpoint-specific extra fields (unlike `completions`' `batch`).

- [ ] **Step 1: Write the failing tests — one per endpoint**

Add to the existing test module in `rs_openai/src/apis/organization/usage.rs` (add this import to the existing `use` list: `crate::interfaces::organization::usage::{UsageAudioSpeechesRequest, UsageAudioTranscriptionsRequest, UsageEmbeddingsRequest, UsageModerationsRequest}`):

```rust
    #[tokio::test]
    async fn embeddings_serializes_request_and_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/organization/usage/embeddings"))
            .and(query_param("start_time", "1700000000"))
            .and(query_param("models", "text-embedding-3-small"))
            .and(query_param("group_by", "model"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "object": "page",
                "has_more": false,
                "next_page": null,
                "data": [{
                    "object": "bucket",
                    "start_time": 1700000000_i64,
                    "end_time": 1700086400_i64,
                    "results": [{
                        "object": "organization.usage.embeddings.result",
                        "input_tokens": 42_i64,
                        "num_model_requests": 3_i64,
                        "model": "text-embedding-3-small"
                    }]
                }]
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("test-key").with_base_url(mock_server.uri());
        let req = UsageEmbeddingsRequest::builder()
            .start_time(1700000000)
            .models(vec!["text-embedding-3-small".to_string()])
            .group_by(vec!["model".to_string()])
            .build();
        let result = client.organization().usage().embeddings(&req).await.unwrap();

        assert_eq!(result.data[0].results.len(), 1);
        match &result.data[0].results[0] {
            crate::interfaces::organization::usage::UsageResult::Embeddings(r) => {
                assert_eq!(r.input_tokens, 42);
            }
            other => panic!("expected Embeddings variant, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn moderations_serializes_request_and_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/organization/usage/moderations"))
            .and(query_param("start_time", "1700000000"))
            .and(query_param("api_key_ids", "key_1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "object": "page",
                "has_more": false,
                "next_page": null,
                "data": [{
                    "object": "bucket",
                    "start_time": 1700000000_i64,
                    "end_time": 1700086400_i64,
                    "results": [{
                        "object": "organization.usage.moderations.result",
                        "input_tokens": 10_i64,
                        "num_model_requests": 1_i64,
                        "api_key_id": "key_1"
                    }]
                }]
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("test-key").with_base_url(mock_server.uri());
        let req = UsageModerationsRequest::builder()
            .start_time(1700000000)
            .api_key_ids(vec!["key_1".to_string()])
            .build();
        let result = client.organization().usage().moderations(&req).await.unwrap();

        match &result.data[0].results[0] {
            crate::interfaces::organization::usage::UsageResult::Moderations(r) => {
                assert_eq!(r.api_key_id.as_deref(), Some("key_1"));
            }
            other => panic!("expected Moderations variant, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn audio_speeches_serializes_request_and_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/organization/usage/audio_speeches"))
            .and(query_param("start_time", "1700000000"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "object": "page",
                "has_more": false,
                "next_page": null,
                "data": [{
                    "object": "bucket",
                    "start_time": 1700000000_i64,
                    "end_time": 1700086400_i64,
                    "results": [{
                        "object": "organization.usage.audio_speeches.result",
                        "characters": 500_i64,
                        "num_model_requests": 2_i64
                    }]
                }]
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("test-key").with_base_url(mock_server.uri());
        let req = UsageAudioSpeechesRequest::builder().start_time(1700000000).build();
        let result = client.organization().usage().audio_speeches(&req).await.unwrap();

        match &result.data[0].results[0] {
            crate::interfaces::organization::usage::UsageResult::AudioSpeeches(r) => {
                assert_eq!(r.characters, 500);
            }
            other => panic!("expected AudioSpeeches variant, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn audio_transcriptions_serializes_request_and_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/organization/usage/audio_transcriptions"))
            .and(query_param("start_time", "1700000000"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "object": "page",
                "has_more": false,
                "next_page": null,
                "data": [{
                    "object": "bucket",
                    "start_time": 1700000000_i64,
                    "end_time": 1700086400_i64,
                    "results": [{
                        "object": "organization.usage.audio_transcriptions.result",
                        "seconds": 120_i64,
                        "num_model_requests": 4_i64
                    }]
                }]
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("test-key").with_base_url(mock_server.uri());
        let req = UsageAudioTranscriptionsRequest::builder().start_time(1700000000).build();
        let result = client.organization().usage().audio_transcriptions(&req).await.unwrap();

        match &result.data[0].results[0] {
            crate::interfaces::organization::usage::UsageResult::AudioTranscriptions(r) => {
                assert_eq!(r.seconds, 120);
            }
            other => panic!("expected AudioTranscriptions variant, got {other:?}"),
        }
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd rs_openai && NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai apis::organization::usage::tests`
Expected: FAIL to compile — the 4 new request types and methods don't exist yet.

- [ ] **Step 3: Append the 4 request types**

Append to the end of `rs_openai/src/interfaces/organization/usage.rs`:

```rust

/// Query params for `GET /organization/usage/embeddings`. `group_by` accepts any
/// combination of `project_id`, `user_id`, `api_key_id`, `model`.
#[derive(Builder, Debug, Clone, Default)]
pub struct UsageEmbeddingsRequest {
    pub start_time: i64,
    pub end_time: Option<i64>,
    #[builder(into)]
    pub bucket_width: Option<String>,
    pub limit: Option<u32>,
    #[builder(into)]
    pub page: Option<String>,
    pub project_ids: Option<Vec<String>>,
    pub api_key_ids: Option<Vec<String>>,
    pub user_ids: Option<Vec<String>>,
    pub models: Option<Vec<String>>,
    pub group_by: Option<Vec<String>>,
}

impl UsageEmbeddingsRequest {
    pub(crate) fn to_query_pairs(&self) -> Vec<(String, String)> {
        let mut pairs = Vec::new();
        pairs.push(("start_time".to_string(), self.start_time.to_string()));
        if let Some(v) = self.end_time {
            pairs.push(("end_time".to_string(), v.to_string()));
        }
        if let Some(v) = &self.bucket_width {
            pairs.push(("bucket_width".to_string(), v.clone()));
        }
        if let Some(v) = self.limit {
            pairs.push(("limit".to_string(), v.to_string()));
        }
        if let Some(v) = &self.page {
            pairs.push(("page".to_string(), v.clone()));
        }
        if let Some(values) = &self.project_ids {
            for v in values {
                pairs.push(("project_ids".to_string(), v.clone()));
            }
        }
        if let Some(values) = &self.api_key_ids {
            for v in values {
                pairs.push(("api_key_ids".to_string(), v.clone()));
            }
        }
        if let Some(values) = &self.user_ids {
            for v in values {
                pairs.push(("user_ids".to_string(), v.clone()));
            }
        }
        if let Some(values) = &self.models {
            for v in values {
                pairs.push(("models".to_string(), v.clone()));
            }
        }
        if let Some(values) = &self.group_by {
            for v in values {
                pairs.push(("group_by".to_string(), v.clone()));
            }
        }
        pairs
    }
}

/// Query params for `GET /organization/usage/moderations`. `group_by` accepts any
/// combination of `project_id`, `user_id`, `api_key_id`, `model`.
#[derive(Builder, Debug, Clone, Default)]
pub struct UsageModerationsRequest {
    pub start_time: i64,
    pub end_time: Option<i64>,
    #[builder(into)]
    pub bucket_width: Option<String>,
    pub limit: Option<u32>,
    #[builder(into)]
    pub page: Option<String>,
    pub project_ids: Option<Vec<String>>,
    pub api_key_ids: Option<Vec<String>>,
    pub user_ids: Option<Vec<String>>,
    pub models: Option<Vec<String>>,
    pub group_by: Option<Vec<String>>,
}

impl UsageModerationsRequest {
    pub(crate) fn to_query_pairs(&self) -> Vec<(String, String)> {
        let mut pairs = Vec::new();
        pairs.push(("start_time".to_string(), self.start_time.to_string()));
        if let Some(v) = self.end_time {
            pairs.push(("end_time".to_string(), v.to_string()));
        }
        if let Some(v) = &self.bucket_width {
            pairs.push(("bucket_width".to_string(), v.clone()));
        }
        if let Some(v) = self.limit {
            pairs.push(("limit".to_string(), v.to_string()));
        }
        if let Some(v) = &self.page {
            pairs.push(("page".to_string(), v.clone()));
        }
        if let Some(values) = &self.project_ids {
            for v in values {
                pairs.push(("project_ids".to_string(), v.clone()));
            }
        }
        if let Some(values) = &self.api_key_ids {
            for v in values {
                pairs.push(("api_key_ids".to_string(), v.clone()));
            }
        }
        if let Some(values) = &self.user_ids {
            for v in values {
                pairs.push(("user_ids".to_string(), v.clone()));
            }
        }
        if let Some(values) = &self.models {
            for v in values {
                pairs.push(("models".to_string(), v.clone()));
            }
        }
        if let Some(values) = &self.group_by {
            for v in values {
                pairs.push(("group_by".to_string(), v.clone()));
            }
        }
        pairs
    }
}

/// Query params for `GET /organization/usage/audio_speeches`. `group_by` accepts any
/// combination of `project_id`, `user_id`, `api_key_id`, `model`.
#[derive(Builder, Debug, Clone, Default)]
pub struct UsageAudioSpeechesRequest {
    pub start_time: i64,
    pub end_time: Option<i64>,
    #[builder(into)]
    pub bucket_width: Option<String>,
    pub limit: Option<u32>,
    #[builder(into)]
    pub page: Option<String>,
    pub project_ids: Option<Vec<String>>,
    pub api_key_ids: Option<Vec<String>>,
    pub user_ids: Option<Vec<String>>,
    pub models: Option<Vec<String>>,
    pub group_by: Option<Vec<String>>,
}

impl UsageAudioSpeechesRequest {
    pub(crate) fn to_query_pairs(&self) -> Vec<(String, String)> {
        let mut pairs = Vec::new();
        pairs.push(("start_time".to_string(), self.start_time.to_string()));
        if let Some(v) = self.end_time {
            pairs.push(("end_time".to_string(), v.to_string()));
        }
        if let Some(v) = &self.bucket_width {
            pairs.push(("bucket_width".to_string(), v.clone()));
        }
        if let Some(v) = self.limit {
            pairs.push(("limit".to_string(), v.to_string()));
        }
        if let Some(v) = &self.page {
            pairs.push(("page".to_string(), v.clone()));
        }
        if let Some(values) = &self.project_ids {
            for v in values {
                pairs.push(("project_ids".to_string(), v.clone()));
            }
        }
        if let Some(values) = &self.api_key_ids {
            for v in values {
                pairs.push(("api_key_ids".to_string(), v.clone()));
            }
        }
        if let Some(values) = &self.user_ids {
            for v in values {
                pairs.push(("user_ids".to_string(), v.clone()));
            }
        }
        if let Some(values) = &self.models {
            for v in values {
                pairs.push(("models".to_string(), v.clone()));
            }
        }
        if let Some(values) = &self.group_by {
            for v in values {
                pairs.push(("group_by".to_string(), v.clone()));
            }
        }
        pairs
    }
}

/// Query params for `GET /organization/usage/audio_transcriptions`. `group_by` accepts
/// any combination of `project_id`, `user_id`, `api_key_id`, `model`.
#[derive(Builder, Debug, Clone, Default)]
pub struct UsageAudioTranscriptionsRequest {
    pub start_time: i64,
    pub end_time: Option<i64>,
    #[builder(into)]
    pub bucket_width: Option<String>,
    pub limit: Option<u32>,
    #[builder(into)]
    pub page: Option<String>,
    pub project_ids: Option<Vec<String>>,
    pub api_key_ids: Option<Vec<String>>,
    pub user_ids: Option<Vec<String>>,
    pub models: Option<Vec<String>>,
    pub group_by: Option<Vec<String>>,
}

impl UsageAudioTranscriptionsRequest {
    pub(crate) fn to_query_pairs(&self) -> Vec<(String, String)> {
        let mut pairs = Vec::new();
        pairs.push(("start_time".to_string(), self.start_time.to_string()));
        if let Some(v) = self.end_time {
            pairs.push(("end_time".to_string(), v.to_string()));
        }
        if let Some(v) = &self.bucket_width {
            pairs.push(("bucket_width".to_string(), v.clone()));
        }
        if let Some(v) = self.limit {
            pairs.push(("limit".to_string(), v.to_string()));
        }
        if let Some(v) = &self.page {
            pairs.push(("page".to_string(), v.clone()));
        }
        if let Some(values) = &self.project_ids {
            for v in values {
                pairs.push(("project_ids".to_string(), v.clone()));
            }
        }
        if let Some(values) = &self.api_key_ids {
            for v in values {
                pairs.push(("api_key_ids".to_string(), v.clone()));
            }
        }
        if let Some(values) = &self.user_ids {
            for v in values {
                pairs.push(("user_ids".to_string(), v.clone()));
            }
        }
        if let Some(values) = &self.models {
            for v in values {
                pairs.push(("models".to_string(), v.clone()));
            }
        }
        if let Some(values) = &self.group_by {
            for v in values {
                pairs.push(("group_by".to_string(), v.clone()));
            }
        }
        pairs
    }
}
```

- [ ] **Step 4: Append the 4 methods**

In `rs_openai/src/apis/organization/usage.rs`, add inside `impl<'a> OrganizationUsage<'a>`, after `completions`:

```rust
    pub async fn embeddings(
        &self,
        req: &usage::UsageEmbeddingsRequest,
    ) -> OpenAIResponse<usage::UsageResponse> {
        self.openai
            .get_with_pairs("/organization/usage/embeddings", &req.to_query_pairs())
            .await
    }

    pub async fn moderations(
        &self,
        req: &usage::UsageModerationsRequest,
    ) -> OpenAIResponse<usage::UsageResponse> {
        self.openai
            .get_with_pairs("/organization/usage/moderations", &req.to_query_pairs())
            .await
    }

    pub async fn audio_speeches(
        &self,
        req: &usage::UsageAudioSpeechesRequest,
    ) -> OpenAIResponse<usage::UsageResponse> {
        self.openai
            .get_with_pairs("/organization/usage/audio_speeches", &req.to_query_pairs())
            .await
    }

    pub async fn audio_transcriptions(
        &self,
        req: &usage::UsageAudioTranscriptionsRequest,
    ) -> OpenAIResponse<usage::UsageResponse> {
        self.openai
            .get_with_pairs(
                "/organization/usage/audio_transcriptions",
                &req.to_query_pairs(),
            )
            .await
    }
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cd rs_openai && NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai apis::organization::usage::tests`
Expected: 7 tests PASS (3 from Task 1 + 4 new)

- [ ] **Step 6: Run fmt, clippy, and the full test suite**

```bash
cd rs_openai
cargo fmt
NO_PROXY=127.0.0.1,localhost cargo clippy -p rs_openai --all-targets -- -D warnings
NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai
```
Expected: clean, clean, 224/224 (220 from Task 1 + 4 new — re-verify the actual current count first if Task 1 isn't merged yet).

- [ ] **Step 7: Commit**

```bash
git add rs_openai/src/interfaces/organization/usage.rs
git add rs_openai/src/apis/organization/usage.rs
git commit -m "feat(organization): add usage embeddings/moderations/audio_speeches/audio_transcriptions"
```

---

### Task 3: `images`, `web_search_calls`

**Files:**
- Modify: `rs_openai/src/interfaces/organization/usage.rs` (append 2 request types)
- Modify: `rs_openai/src/apis/organization/usage.rs` (append 2 methods)
- Test: `rs_openai/src/apis/organization/usage.rs` (extend the test module)

**Interfaces:**
- Consumes: `interfaces::organization::usage::{UsageImagesResult, UsageWebSearchesResult, UsageResponse}` from Task 1.
- Produces: `interfaces::organization::usage::{UsageImagesRequest, UsageWebSearchCallsRequest}`, `OrganizationUsage::{images, web_search_calls}`.

`images` adds `sizes: Option<Vec<String>>` and `sources: Option<Vec<String>>` beyond the shared core (`group_by` additionally accepts `size`, `source`). `web_search_calls` adds `context_levels: Option<Vec<String>>` beyond the shared core (`group_by` additionally accepts `context_level`).

- [ ] **Step 1: Write the failing tests**

Add to the test module (add import: `crate::interfaces::organization::usage::{UsageImagesRequest, UsageWebSearchCallsRequest}`):

```rust
    #[tokio::test]
    async fn images_serializes_sizes_and_sources_and_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/organization/usage/images"))
            .and(query_param("start_time", "1700000000"))
            .and(query_param("sizes", "1024x1024"))
            .and(query_param("sources", "image.generation"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "object": "page",
                "has_more": false,
                "next_page": null,
                "data": [{
                    "object": "bucket",
                    "start_time": 1700000000_i64,
                    "end_time": 1700086400_i64,
                    "results": [{
                        "object": "organization.usage.images.result",
                        "images": 5_i64,
                        "num_model_requests": 5_i64,
                        "size": "1024x1024",
                        "source": "image.generation"
                    }]
                }]
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("test-key").with_base_url(mock_server.uri());
        let req = UsageImagesRequest::builder()
            .start_time(1700000000)
            .sizes(vec!["1024x1024".to_string()])
            .sources(vec!["image.generation".to_string()])
            .build();
        let result = client.organization().usage().images(&req).await.unwrap();

        match &result.data[0].results[0] {
            crate::interfaces::organization::usage::UsageResult::Images(r) => {
                assert_eq!(r.images, 5);
                assert_eq!(r.source.as_deref(), Some("image.generation"));
            }
            other => panic!("expected Images variant, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn web_search_calls_serializes_context_levels_and_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/organization/usage/web_search_calls"))
            .and(query_param("start_time", "1700000000"))
            .and(query_param("context_levels", "high"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "object": "page",
                "has_more": false,
                "next_page": null,
                "data": [{
                    "object": "bucket",
                    "start_time": 1700000000_i64,
                    "end_time": 1700086400_i64,
                    "results": [{
                        "object": "organization.usage.web_searches.result",
                        "num_model_requests": 3_i64,
                        "num_requests": 3_i64,
                        "context_level": "high"
                    }]
                }]
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("test-key").with_base_url(mock_server.uri());
        let req = UsageWebSearchCallsRequest::builder()
            .start_time(1700000000)
            .context_levels(vec!["high".to_string()])
            .build();
        let result = client.organization().usage().web_search_calls(&req).await.unwrap();

        match &result.data[0].results[0] {
            crate::interfaces::organization::usage::UsageResult::WebSearches(r) => {
                assert_eq!(r.context_level.as_deref(), Some("high"));
            }
            other => panic!("expected WebSearches variant, got {other:?}"),
        }
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd rs_openai && NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai apis::organization::usage::tests`
Expected: FAIL to compile.

- [ ] **Step 3: Append the 2 request types**

Append to the end of `rs_openai/src/interfaces/organization/usage.rs`:

```rust

/// Query params for `GET /organization/usage/images`. `group_by` accepts any
/// combination of `project_id`, `user_id`, `api_key_id`, `model`, `size`, `source`.
#[derive(Builder, Debug, Clone, Default)]
pub struct UsageImagesRequest {
    pub start_time: i64,
    pub end_time: Option<i64>,
    #[builder(into)]
    pub bucket_width: Option<String>,
    pub limit: Option<u32>,
    #[builder(into)]
    pub page: Option<String>,
    pub project_ids: Option<Vec<String>>,
    pub api_key_ids: Option<Vec<String>>,
    pub user_ids: Option<Vec<String>>,
    pub models: Option<Vec<String>>,
    pub sizes: Option<Vec<String>>,
    pub sources: Option<Vec<String>>,
    pub group_by: Option<Vec<String>>,
}

impl UsageImagesRequest {
    pub(crate) fn to_query_pairs(&self) -> Vec<(String, String)> {
        let mut pairs = Vec::new();
        pairs.push(("start_time".to_string(), self.start_time.to_string()));
        if let Some(v) = self.end_time {
            pairs.push(("end_time".to_string(), v.to_string()));
        }
        if let Some(v) = &self.bucket_width {
            pairs.push(("bucket_width".to_string(), v.clone()));
        }
        if let Some(v) = self.limit {
            pairs.push(("limit".to_string(), v.to_string()));
        }
        if let Some(v) = &self.page {
            pairs.push(("page".to_string(), v.clone()));
        }
        if let Some(values) = &self.project_ids {
            for v in values {
                pairs.push(("project_ids".to_string(), v.clone()));
            }
        }
        if let Some(values) = &self.api_key_ids {
            for v in values {
                pairs.push(("api_key_ids".to_string(), v.clone()));
            }
        }
        if let Some(values) = &self.user_ids {
            for v in values {
                pairs.push(("user_ids".to_string(), v.clone()));
            }
        }
        if let Some(values) = &self.models {
            for v in values {
                pairs.push(("models".to_string(), v.clone()));
            }
        }
        if let Some(values) = &self.sizes {
            for v in values {
                pairs.push(("sizes".to_string(), v.clone()));
            }
        }
        if let Some(values) = &self.sources {
            for v in values {
                pairs.push(("sources".to_string(), v.clone()));
            }
        }
        if let Some(values) = &self.group_by {
            for v in values {
                pairs.push(("group_by".to_string(), v.clone()));
            }
        }
        pairs
    }
}

/// Query params for `GET /organization/usage/web_search_calls`. `group_by` accepts any
/// combination of `project_id`, `user_id`, `api_key_id`, `model`, `context_level`.
#[derive(Builder, Debug, Clone, Default)]
pub struct UsageWebSearchCallsRequest {
    pub start_time: i64,
    pub end_time: Option<i64>,
    #[builder(into)]
    pub bucket_width: Option<String>,
    pub limit: Option<u32>,
    #[builder(into)]
    pub page: Option<String>,
    pub project_ids: Option<Vec<String>>,
    pub api_key_ids: Option<Vec<String>>,
    pub user_ids: Option<Vec<String>>,
    pub models: Option<Vec<String>>,
    pub context_levels: Option<Vec<String>>,
    pub group_by: Option<Vec<String>>,
}

impl UsageWebSearchCallsRequest {
    pub(crate) fn to_query_pairs(&self) -> Vec<(String, String)> {
        let mut pairs = Vec::new();
        pairs.push(("start_time".to_string(), self.start_time.to_string()));
        if let Some(v) = self.end_time {
            pairs.push(("end_time".to_string(), v.to_string()));
        }
        if let Some(v) = &self.bucket_width {
            pairs.push(("bucket_width".to_string(), v.clone()));
        }
        if let Some(v) = self.limit {
            pairs.push(("limit".to_string(), v.to_string()));
        }
        if let Some(v) = &self.page {
            pairs.push(("page".to_string(), v.clone()));
        }
        if let Some(values) = &self.project_ids {
            for v in values {
                pairs.push(("project_ids".to_string(), v.clone()));
            }
        }
        if let Some(values) = &self.api_key_ids {
            for v in values {
                pairs.push(("api_key_ids".to_string(), v.clone()));
            }
        }
        if let Some(values) = &self.user_ids {
            for v in values {
                pairs.push(("user_ids".to_string(), v.clone()));
            }
        }
        if let Some(values) = &self.models {
            for v in values {
                pairs.push(("models".to_string(), v.clone()));
            }
        }
        if let Some(values) = &self.context_levels {
            for v in values {
                pairs.push(("context_levels".to_string(), v.clone()));
            }
        }
        if let Some(values) = &self.group_by {
            for v in values {
                pairs.push(("group_by".to_string(), v.clone()));
            }
        }
        pairs
    }
}
```

- [ ] **Step 4: Append the 2 methods**

In `rs_openai/src/apis/organization/usage.rs`, add inside `impl<'a> OrganizationUsage<'a>`:

```rust
    pub async fn images(
        &self,
        req: &usage::UsageImagesRequest,
    ) -> OpenAIResponse<usage::UsageResponse> {
        self.openai
            .get_with_pairs("/organization/usage/images", &req.to_query_pairs())
            .await
    }

    pub async fn web_search_calls(
        &self,
        req: &usage::UsageWebSearchCallsRequest,
    ) -> OpenAIResponse<usage::UsageResponse> {
        self.openai
            .get_with_pairs("/organization/usage/web_search_calls", &req.to_query_pairs())
            .await
    }
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cd rs_openai && NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai apis::organization::usage::tests`
Expected: 9 tests PASS (7 from Tasks 1-2 + 2 new)

- [ ] **Step 6: Run fmt, clippy, and the full test suite**

```bash
cd rs_openai
cargo fmt
NO_PROXY=127.0.0.1,localhost cargo clippy -p rs_openai --all-targets -- -D warnings
NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai
```
Expected: clean, clean, 226/226 (224 + 2 new — re-verify actual current count first if Tasks 1-2 aren't merged yet).

- [ ] **Step 7: Commit**

```bash
git add rs_openai/src/interfaces/organization/usage.rs
git add rs_openai/src/apis/organization/usage.rs
git commit -m "feat(organization): add usage images/web_search_calls"
```

---

### Task 4: `vector_stores`, `code_interpreter_sessions`, `file_search_calls`, `costs` + final cross-check

**Files:**
- Modify: `rs_openai/src/interfaces/organization/usage.rs` (append 4 request types)
- Modify: `rs_openai/src/apis/organization/usage.rs` (append 4 methods)
- Modify: `rs_openai/src/apis/organization/mod.rs` (final cross-check — see Step 9)
- Test: `rs_openai/src/apis/organization/usage.rs` (extend the test module)

**Interfaces:**
- Consumes: `interfaces::organization::usage::{UsageVectorStoresResult, UsageCodeInterpreterSessionsResult, UsageFileSearchesResult, UsageCostsResult, UsageCostsAmount, UsageResponse}` from Task 1.
- Produces: `interfaces::organization::usage::{UsageVectorStoresRequest, UsageCodeInterpreterSessionsRequest, UsageFileSearchCallsRequest, UsageCostsRequest}`, `OrganizationUsage::{vector_stores, code_interpreter_sessions, file_search_calls, costs}`. This is the last task in the plan — the final cross-check closes out the whole sub-slice.

`vector_stores` and `code_interpreter_sessions` have the reduced field set: only `start_time`, `end_time`, `bucket_width`, `limit`, `page`, `project_ids`, `group_by` (valid value: `project_id` only — no `api_key_ids`/`models`/`user_ids` at all). `file_search_calls` adds `vector_store_ids: Option<Vec<String>>` and `api_key_ids` but no `models` (`group_by` additionally accepts `user_id`, `api_key_id`, `vector_store_id`). `costs` has its own distinct field set (`api_key_ids`, no `models`/`user_ids`; `group_by` accepts `project_id`, `line_item`, `api_key_id`) AND a distinct route: `/organization/costs`, NOT `/organization/usage/costs`.

- [ ] **Step 1: Write the failing tests**

Add to the test module (add import: `crate::interfaces::organization::usage::{UsageCodeInterpreterSessionsRequest, UsageCostsRequest, UsageFileSearchCallsRequest, UsageVectorStoresRequest}`):

```rust
    #[tokio::test]
    async fn vector_stores_serializes_request_and_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/organization/usage/vector_stores"))
            .and(query_param("start_time", "1700000000"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "object": "page",
                "has_more": false,
                "next_page": null,
                "data": [{
                    "object": "bucket",
                    "start_time": 1700000000_i64,
                    "end_time": 1700086400_i64,
                    "results": [{
                        "object": "organization.usage.vector_stores.result",
                        "usage_bytes": 1024_i64
                    }]
                }]
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("test-key").with_base_url(mock_server.uri());
        let req = UsageVectorStoresRequest::builder().start_time(1700000000).build();
        let result = client.organization().usage().vector_stores(&req).await.unwrap();

        match &result.data[0].results[0] {
            crate::interfaces::organization::usage::UsageResult::VectorStores(r) => {
                assert_eq!(r.usage_bytes, 1024);
            }
            other => panic!("expected VectorStores variant, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn code_interpreter_sessions_serializes_request_and_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/organization/usage/code_interpreter_sessions"))
            .and(query_param("start_time", "1700000000"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "object": "page",
                "has_more": false,
                "next_page": null,
                "data": [{
                    "object": "bucket",
                    "start_time": 1700000000_i64,
                    "end_time": 1700086400_i64,
                    "results": [{
                        "object": "organization.usage.code_interpreter_sessions.result",
                        "num_sessions": 7_i64
                    }]
                }]
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("test-key").with_base_url(mock_server.uri());
        let req = UsageCodeInterpreterSessionsRequest::builder().start_time(1700000000).build();
        let result = client
            .organization()
            .usage()
            .code_interpreter_sessions(&req)
            .await
            .unwrap();

        match &result.data[0].results[0] {
            crate::interfaces::organization::usage::UsageResult::CodeInterpreterSessions(r) => {
                assert_eq!(r.num_sessions, 7);
            }
            other => panic!("expected CodeInterpreterSessions variant, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn file_search_calls_serializes_vector_store_ids_and_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/organization/usage/file_search_calls"))
            .and(query_param("start_time", "1700000000"))
            .and(query_param("vector_store_ids", "vs_1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "object": "page",
                "has_more": false,
                "next_page": null,
                "data": [{
                    "object": "bucket",
                    "start_time": 1700000000_i64,
                    "end_time": 1700086400_i64,
                    "results": [{
                        "object": "organization.usage.file_searches.result",
                        "num_requests": 9_i64,
                        "vector_store_id": "vs_1"
                    }]
                }]
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("test-key").with_base_url(mock_server.uri());
        let req = UsageFileSearchCallsRequest::builder()
            .start_time(1700000000)
            .vector_store_ids(vec!["vs_1".to_string()])
            .build();
        let result = client.organization().usage().file_search_calls(&req).await.unwrap();

        match &result.data[0].results[0] {
            crate::interfaces::organization::usage::UsageResult::FileSearches(r) => {
                assert_eq!(r.vector_store_id.as_deref(), Some("vs_1"));
            }
            other => panic!("expected FileSearches variant, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn costs_uses_distinct_route_and_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/organization/costs"))
            .and(query_param("start_time", "1700000000"))
            .and(query_param("group_by", "line_item"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "object": "page",
                "has_more": false,
                "next_page": null,
                "data": [{
                    "object": "bucket",
                    "start_time": 1700000000_i64,
                    "end_time": 1700086400_i64,
                    "results": [{
                        "object": "organization.costs.result",
                        "amount": {"currency": "usd", "value": 12.5},
                        "line_item": "gpt-4o"
                    }]
                }]
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("test-key").with_base_url(mock_server.uri());
        let req = UsageCostsRequest::builder()
            .start_time(1700000000)
            .group_by(vec!["line_item".to_string()])
            .build();
        let result = client.organization().usage().costs(&req).await.unwrap();

        match &result.data[0].results[0] {
            crate::interfaces::organization::usage::UsageResult::Costs(r) => {
                assert_eq!(r.line_item.as_deref(), Some("gpt-4o"));
                let amount = r.amount.as_ref().unwrap();
                assert_eq!(amount.currency.as_deref(), Some("usd"));
                assert_eq!(amount.value, Some(12.5));
            }
            other => panic!("expected Costs variant, got {other:?}"),
        }
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd rs_openai && NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai apis::organization::usage::tests`
Expected: FAIL to compile.

- [ ] **Step 3: Append the 4 request types**

Append to the end of `rs_openai/src/interfaces/organization/usage.rs`:

```rust

/// Query params for `GET /organization/usage/vector_stores`. `group_by` accepts only
/// `project_id`.
#[derive(Builder, Debug, Clone, Default)]
pub struct UsageVectorStoresRequest {
    pub start_time: i64,
    pub end_time: Option<i64>,
    #[builder(into)]
    pub bucket_width: Option<String>,
    pub limit: Option<u32>,
    #[builder(into)]
    pub page: Option<String>,
    pub project_ids: Option<Vec<String>>,
    pub group_by: Option<Vec<String>>,
}

impl UsageVectorStoresRequest {
    pub(crate) fn to_query_pairs(&self) -> Vec<(String, String)> {
        let mut pairs = Vec::new();
        pairs.push(("start_time".to_string(), self.start_time.to_string()));
        if let Some(v) = self.end_time {
            pairs.push(("end_time".to_string(), v.to_string()));
        }
        if let Some(v) = &self.bucket_width {
            pairs.push(("bucket_width".to_string(), v.clone()));
        }
        if let Some(v) = self.limit {
            pairs.push(("limit".to_string(), v.to_string()));
        }
        if let Some(v) = &self.page {
            pairs.push(("page".to_string(), v.clone()));
        }
        if let Some(values) = &self.project_ids {
            for v in values {
                pairs.push(("project_ids".to_string(), v.clone()));
            }
        }
        if let Some(values) = &self.group_by {
            for v in values {
                pairs.push(("group_by".to_string(), v.clone()));
            }
        }
        pairs
    }
}

/// Query params for `GET /organization/usage/code_interpreter_sessions`. `group_by`
/// accepts only `project_id`.
#[derive(Builder, Debug, Clone, Default)]
pub struct UsageCodeInterpreterSessionsRequest {
    pub start_time: i64,
    pub end_time: Option<i64>,
    #[builder(into)]
    pub bucket_width: Option<String>,
    pub limit: Option<u32>,
    #[builder(into)]
    pub page: Option<String>,
    pub project_ids: Option<Vec<String>>,
    pub group_by: Option<Vec<String>>,
}

impl UsageCodeInterpreterSessionsRequest {
    pub(crate) fn to_query_pairs(&self) -> Vec<(String, String)> {
        let mut pairs = Vec::new();
        pairs.push(("start_time".to_string(), self.start_time.to_string()));
        if let Some(v) = self.end_time {
            pairs.push(("end_time".to_string(), v.to_string()));
        }
        if let Some(v) = &self.bucket_width {
            pairs.push(("bucket_width".to_string(), v.clone()));
        }
        if let Some(v) = self.limit {
            pairs.push(("limit".to_string(), v.to_string()));
        }
        if let Some(v) = &self.page {
            pairs.push(("page".to_string(), v.clone()));
        }
        if let Some(values) = &self.project_ids {
            for v in values {
                pairs.push(("project_ids".to_string(), v.clone()));
            }
        }
        if let Some(values) = &self.group_by {
            for v in values {
                pairs.push(("group_by".to_string(), v.clone()));
            }
        }
        pairs
    }
}

/// Query params for `GET /organization/usage/file_search_calls`. `group_by` accepts any
/// combination of `project_id`, `user_id`, `api_key_id`, `vector_store_id`.
#[derive(Builder, Debug, Clone, Default)]
pub struct UsageFileSearchCallsRequest {
    pub start_time: i64,
    pub end_time: Option<i64>,
    #[builder(into)]
    pub bucket_width: Option<String>,
    pub limit: Option<u32>,
    #[builder(into)]
    pub page: Option<String>,
    pub project_ids: Option<Vec<String>>,
    pub api_key_ids: Option<Vec<String>>,
    pub user_ids: Option<Vec<String>>,
    pub vector_store_ids: Option<Vec<String>>,
    pub group_by: Option<Vec<String>>,
}

impl UsageFileSearchCallsRequest {
    pub(crate) fn to_query_pairs(&self) -> Vec<(String, String)> {
        let mut pairs = Vec::new();
        pairs.push(("start_time".to_string(), self.start_time.to_string()));
        if let Some(v) = self.end_time {
            pairs.push(("end_time".to_string(), v.to_string()));
        }
        if let Some(v) = &self.bucket_width {
            pairs.push(("bucket_width".to_string(), v.clone()));
        }
        if let Some(v) = self.limit {
            pairs.push(("limit".to_string(), v.to_string()));
        }
        if let Some(v) = &self.page {
            pairs.push(("page".to_string(), v.clone()));
        }
        if let Some(values) = &self.project_ids {
            for v in values {
                pairs.push(("project_ids".to_string(), v.clone()));
            }
        }
        if let Some(values) = &self.api_key_ids {
            for v in values {
                pairs.push(("api_key_ids".to_string(), v.clone()));
            }
        }
        if let Some(values) = &self.user_ids {
            for v in values {
                pairs.push(("user_ids".to_string(), v.clone()));
            }
        }
        if let Some(values) = &self.vector_store_ids {
            for v in values {
                pairs.push(("vector_store_ids".to_string(), v.clone()));
            }
        }
        if let Some(values) = &self.group_by {
            for v in values {
                pairs.push(("group_by".to_string(), v.clone()));
            }
        }
        pairs
    }
}

/// Query params for `GET /organization/costs`. `group_by` accepts any combination of
/// `project_id`, `line_item`, `api_key_id`. `bucket_width` is `"1d"`-only on this
/// endpoint (unlike the other 10, which also support `"1m"`/`"1h"`) -- kept a plain
/// `Option<String>` regardless, per this crate's standing convention.
#[derive(Builder, Debug, Clone, Default)]
pub struct UsageCostsRequest {
    pub start_time: i64,
    pub end_time: Option<i64>,
    #[builder(into)]
    pub bucket_width: Option<String>,
    pub limit: Option<u32>,
    #[builder(into)]
    pub page: Option<String>,
    pub project_ids: Option<Vec<String>>,
    pub api_key_ids: Option<Vec<String>>,
    pub group_by: Option<Vec<String>>,
}

impl UsageCostsRequest {
    pub(crate) fn to_query_pairs(&self) -> Vec<(String, String)> {
        let mut pairs = Vec::new();
        pairs.push(("start_time".to_string(), self.start_time.to_string()));
        if let Some(v) = self.end_time {
            pairs.push(("end_time".to_string(), v.to_string()));
        }
        if let Some(v) = &self.bucket_width {
            pairs.push(("bucket_width".to_string(), v.clone()));
        }
        if let Some(v) = self.limit {
            pairs.push(("limit".to_string(), v.to_string()));
        }
        if let Some(v) = &self.page {
            pairs.push(("page".to_string(), v.clone()));
        }
        if let Some(values) = &self.project_ids {
            for v in values {
                pairs.push(("project_ids".to_string(), v.clone()));
            }
        }
        if let Some(values) = &self.api_key_ids {
            for v in values {
                pairs.push(("api_key_ids".to_string(), v.clone()));
            }
        }
        if let Some(values) = &self.group_by {
            for v in values {
                pairs.push(("group_by".to_string(), v.clone()));
            }
        }
        pairs
    }
}
```

Note: `UsageFileSearchCallsRequest` includes `user_ids` per the confirmed source (`file_search_calls` has `user_ids` even though it lacks `models`) — this was correctly listed in the spec's field-delta table's `group_by` column (`user_id` is a valid group-by value) implying the filter field itself also exists; both are included here for consistency with the source.

- [ ] **Step 4: Append the 4 methods**

In `rs_openai/src/apis/organization/usage.rs`, add inside `impl<'a> OrganizationUsage<'a>`:

```rust
    pub async fn vector_stores(
        &self,
        req: &usage::UsageVectorStoresRequest,
    ) -> OpenAIResponse<usage::UsageResponse> {
        self.openai
            .get_with_pairs("/organization/usage/vector_stores", &req.to_query_pairs())
            .await
    }

    pub async fn code_interpreter_sessions(
        &self,
        req: &usage::UsageCodeInterpreterSessionsRequest,
    ) -> OpenAIResponse<usage::UsageResponse> {
        self.openai
            .get_with_pairs(
                "/organization/usage/code_interpreter_sessions",
                &req.to_query_pairs(),
            )
            .await
    }

    pub async fn file_search_calls(
        &self,
        req: &usage::UsageFileSearchCallsRequest,
    ) -> OpenAIResponse<usage::UsageResponse> {
        self.openai
            .get_with_pairs("/organization/usage/file_search_calls", &req.to_query_pairs())
            .await
    }

    pub async fn costs(
        &self,
        req: &usage::UsageCostsRequest,
    ) -> OpenAIResponse<usage::UsageResponse> {
        self.openai
            .get_with_pairs("/organization/costs", &req.to_query_pairs())
            .await
    }
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cd rs_openai && NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai apis::organization::usage::tests`
Expected: 13 tests PASS (9 from Tasks 1-3 + 4 new)

- [ ] **Step 6: Final cross-check — re-read the whole branch's diff for doc-comment staleness**

This is the last task in the plan. Run `git diff master --stat` (or `git log --oneline master..HEAD` then `git diff <merge-base>..HEAD`) and re-read `rs_openai/src/apis/organization/mod.rs` in full:
- Confirm `usage()`'s doc comment now says "Scoped accessor for organization usage reporting." (no "Not yet implemented").
- Confirm `projects()`/`project()`/`roles()`/`role()` doc comments still correctly say "Not yet implemented" — untouched, out of scope for this slice.
- Confirm the module-level `//!` header at the top of the file is updated to include `usage` among the implemented sub-resources. This exact staleness pattern (a shared header not updated when a new sub-resource ships, since no single task's diff touches it) was caught by the final whole-branch review in BOTH prior Organization/admin sub-slices — fix it proactively now rather than waiting for a third recurrence. The header should end up listing `admin_api_keys, audit_logs, users, invites, groups, usage` as implemented and only `projects, roles` as not yet implemented.

If the header needs updating, edit it directly in this step (before running the final fmt/clippy/test pass below), matching the style of the two prior fixes (see `git log -p -- rs_openai/src/apis/organization/mod.rs` for the exact prior wording if useful context).

- [ ] **Step 7: Run fmt, clippy, and the full test suite one final time**

```bash
cd rs_openai
cargo fmt
NO_PROXY=127.0.0.1,localhost cargo clippy -p rs_openai --all-targets -- -D warnings
NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai
```
Expected: clean, clean, all tests passing (baseline 217 + Task 1's 3 + Task 2's 4 + Task 3's 2 + Task 4's 4 = 230).

- [ ] **Step 8: Commit**

```bash
git add rs_openai/src/interfaces/organization/usage.rs
git add rs_openai/src/apis/organization/usage.rs
git add rs_openai/src/apis/organization/mod.rs
git commit -m "feat(organization): add usage vector_stores/code_interpreter_sessions/file_search_calls/costs, completing sub-slice 3 of 4"
```
