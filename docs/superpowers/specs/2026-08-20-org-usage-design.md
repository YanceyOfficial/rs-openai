# Organization/Admin: Usage (sub-slice 3 of 4) — Design

Status: Approved
Date: 2026-08-20

## Context

Third of 4 sub-slices for the Organization/admin scaffold (see `docs/superpowers/specs/2026-08-19-org-admin-api-keys-audit-logs-design.md` and `docs/superpowers/specs/2026-08-20-org-users-invites-groups-design.md` for the prior two, already shipped and merged).

**Research method, same as sub-slices 1-2**: `gh api repos/openai/openai-python/contents/src/openai/types/admin/organization/usage_*.py` and `src/openai/resources/admin/organization/usage.py` — confirmed authoritative, OpenAPI-spec-generated.

**Structural finding, unique to this sub-slice**: all 11 Usage endpoints (`completions`, `embeddings`, `moderations`, `images`, `audio_speeches`, `audio_transcriptions`, `vector_stores`, `code_interpreter_sessions`, `file_search_calls`, `web_search_calls`, `costs`) share ONE IDENTICAL response envelope (`{data: [{start_time, end_time, object:"bucket", results: [UsageResult]}], has_more, next_page, object:"page"}`) and ONE shared, `object`-discriminated 11-variant `UsageResult` union — confirmed by reading multiple `usage_*_response.py` files and finding byte-identical `DataResult`/`Data` definitions differing only in the outer response type's name. Only the request PARAMS genuinely differ per endpoint (different allowed `group_by` values, a few endpoint-specific filters). This is designed once, shared 11 ways — the opposite of every prior slice's per-resource type duplication, and a deliberate simplification opportunity.

**A crate-blocking finding, resolved via `AskUserQuestion`**: every one of the 11 endpoints' request params leans on array-valued query fields — `group_by: Vec<String>` (which fields to break results down by) plus assorted `Vec<String>` ID/value filters (`project_ids`, `api_key_ids`, `user_ids`, `models`, `sizes`, `sources`, `context_levels`, `vector_store_ids`). `group_by` in particular is not an optional edge feature here — without it, a Usage call returns one aggregate bucket per time period with no breakdown, which defeats the primary purpose of most real calls to this API. This collides directly with the `Vec<String>`-query-serialization limitation confirmed (and worked around by deferral) in sub-slice 2: `serde_urlencoded` (this crate's query backend, reached via `reqwest`'s `.query()`) cannot serialize a struct field typed `Vec<String>` at all.

**The fix, chosen over deferral or a new dependency**: OpenAI's own Python SDK source (`src/openai/_qs.py`) confirms its query builder uses `array_format = "repeat"` — i.e., a `Vec<String>` field serializes as repeated bare keys (`group_by=model&group_by=project_id`), not bracket or comma notation. Separately confirmed empirically in a scratch project: `serde_urlencoded::to_string` on a **`Vec<(String, String)>` of flat key-value pairs** (as opposed to a struct with a `Vec<String>` *field*) serializes correctly and produces exactly this repeated-key format — because a sequence of scalar pairs is precisely the shape `serde_urlencoded` is designed for; the earlier failure was specific to asking it to flatten a `Vec` nested *inside* a struct field, not to sequences of pairs themselves. Each `Usage<Name>Request` type gets a `to_query_pairs(&self) -> Vec<(String, String)>` method that manually builds the full flat pair list (scalars stringified once each, array fields flattened to one pair per element) — replacing the generic `client.get<T, F>(route, query: &F)` call (which serializes a whole struct via `.query(&F)`) with a new `client.get_with_pairs<T>(route, pairs: &[(String, String)])` that calls `.query(&pairs)` instead. **This is the first `client.rs` change across all of Organization/admin's sub-slices** (sub-slices 1-2 both explicitly needed none) — justified because it's a genuinely new query shape (partial-array, not full-struct) that no existing method models, not a workaround for something the existing methods could already do.

## Goals

### Shared types (defined once, used by all 11 endpoints)

```rust
// interfaces/organization/usage.rs

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
```

11 `UsageXxxResult` structs (the `object` field itself lives on the enum variant's `#[serde(rename)]`, so each inner struct omits `object` — an internally-tagged enum in serde requires the tag field to NOT also appear as a named field on the variant's payload struct):

- `UsageCompletionsResult`: `input_tokens: i64`, `num_model_requests: i64`, `output_tokens: i64`, `api_key_id: Option<String>`, `batch: Option<bool>`, `input_audio_tokens: Option<i64>`, `input_cache_write_tokens: Option<i64>`, `input_cached_audio_tokens: Option<i64>`, `input_cached_image_tokens: Option<i64>`, `input_cached_text_tokens: Option<i64>`, `input_cached_tokens: Option<i64>`, `input_image_tokens: Option<i64>`, `input_text_tokens: Option<i64>`, `input_uncached_tokens: Option<i64>`, `model: Option<String>`, `output_audio_tokens: Option<i64>`, `output_image_tokens: Option<i64>`, `output_text_tokens: Option<i64>`, `project_id: Option<String>`, `service_tier: Option<String>`, `user_id: Option<String>`.
- `UsageEmbeddingsResult`: `input_tokens: i64`, `num_model_requests: i64`, `api_key_id: Option<String>`, `model: Option<String>`, `project_id: Option<String>`, `user_id: Option<String>`.
- `UsageModerationsResult`: identical shape to `UsageEmbeddingsResult` (`input_tokens`, `num_model_requests`, `api_key_id`, `model`, `project_id`, `user_id`) — confirmed independently in the source as a separate Python class with the same fields, kept as a separate Rust type (not a type alias) matching this crate's established convention of one type per confirmed API object even when two objects happen to share a shape today (they could diverge independently later; `AuditLogActorUser` was the one deliberate exception, justified there because the source ITSELF referenced one shared shape across two paths — that's not the case here, these are two independently-defined result classes that merely look alike).
- `UsageImagesResult`: `images: i64`, `num_model_requests: i64`, `api_key_id: Option<String>`, `model: Option<String>`, `project_id: Option<String>`, `size: Option<String>`, `source: Option<String>`, `user_id: Option<String>`.
- `UsageAudioSpeechesResult`: `characters: i64`, `num_model_requests: i64`, `api_key_id: Option<String>`, `model: Option<String>`, `project_id: Option<String>`, `user_id: Option<String>`.
- `UsageAudioTranscriptionsResult`: `num_model_requests: i64`, `seconds: i64`, `api_key_id: Option<String>`, `model: Option<String>`, `project_id: Option<String>`, `user_id: Option<String>`.
- `UsageVectorStoresResult`: `usage_bytes: i64`, `project_id: Option<String>`.
- `UsageCodeInterpreterSessionsResult`: `num_sessions: i64`, `project_id: Option<String>`.
- `UsageFileSearchesResult`: `num_requests: i64`, `api_key_id: Option<String>`, `project_id: Option<String>`, `user_id: Option<String>`, `vector_store_id: Option<String>`.
- `UsageWebSearchesResult`: `num_model_requests: i64`, `num_requests: i64`, `api_key_id: Option<String>`, `context_level: Option<String>`, `model: Option<String>`, `project_id: Option<String>`, `user_id: Option<String>`.
- `UsageCostsResult`: `amount: Option<UsageCostsAmount>`, `api_key_id: Option<String>`, `line_item: Option<String>`, `project_id: Option<String>`, `quantity: Option<f64>`. `UsageCostsAmount`: `currency: Option<String>`, `value: Option<f64>`.

All numeric usage-count fields (`input_tokens`, `images`, `characters`, `seconds`, `usage_bytes`, `num_sessions`, `num_requests`, etc.) confirmed non-optional `int` in the source — kept as required `i64` (not `Option`), matching how every prior slice keeps a confirmed-required field required rather than defensively widening it.

### Request types (one per endpoint — genuinely different field sets, kept as separate flat structs per this crate's established no-inheritance convention)

Shared core fields present on every endpoint except `vector_stores`/`code_interpreter_sessions` (smaller) and `costs` (distinct `bucket_width` range): `start_time: i64` (required), `end_time: Option<i64>`, `bucket_width: Option<String>` (confirmed 3-value `"1m"`/`"1h"`/`"1d"` on 10 of 11 endpoints, but kept a plain `String` rather than an enum — matches this crate's established "don't enum-close a field without a concrete reason to construct/match on it" convention, same reasoning as `Invite.status`), `limit: Option<u32>`, `page: Option<String>`, `project_ids: Option<Vec<String>>`, `group_by: Option<Vec<String>>` (kept plain `Vec<String>`, not a closed enum, since the valid value SET genuinely differs per endpoint — see below — and 11 near-duplicate enums would be pure boilerplate for a filter this crate doesn't validate client-side anyway; valid values documented per-field in doc comments).

Per-endpoint field deltas (confirmed via source, all optional):

| Endpoint | Extra fields beyond the shared core | `group_by` confirmed valid values |
|---|---|---|
| `completions` | `api_key_ids`, `user_ids`, `batch: Option<bool>`, `models` | `project_id`, `user_id`, `api_key_id`, `model`, `batch`, `service_tier` |
| `embeddings` | `api_key_ids`, `user_ids`, `models` | `project_id`, `user_id`, `api_key_id`, `model` |
| `moderations` | `api_key_ids`, `user_ids`, `models` | `project_id`, `user_id`, `api_key_id`, `model` |
| `images` | `api_key_ids`, `user_ids`, `models`, `sizes`, `sources` | `project_id`, `user_id`, `api_key_id`, `model`, `size`, `source` |
| `audio_speeches` | `api_key_ids`, `user_ids`, `models` | `project_id`, `user_id`, `api_key_id`, `model` |
| `audio_transcriptions` | `api_key_ids`, `user_ids`, `models` | `project_id`, `user_id`, `api_key_id`, `model` |
| `vector_stores` | (none — no `api_key_ids`/`models`/`user_ids` at all) | `project_id` only |
| `code_interpreter_sessions` | (none — same reduced set as `vector_stores`) | `project_id` only |
| `file_search_calls` | `api_key_ids`, `user_ids`, `vector_store_ids` (no `models`) | `project_id`, `user_id`, `api_key_id`, `vector_store_id` |
| `web_search_calls` | `api_key_ids`, `user_ids`, `context_levels`, `models` | `project_id`, `user_id`, `api_key_id`, `model`, `context_level` |
| `costs` | `api_key_ids` (no `models`/`user_ids`); `bucket_width` confirmed `"1d"`-only (kept plain `String` regardless, doc-noted) | `project_id`, `line_item`, `api_key_id` |

Every request type name: `UsageCompletionsRequest`, `UsageEmbeddingsRequest`, `UsageModerationsRequest`, `UsageImagesRequest`, `UsageAudioSpeechesRequest`, `UsageAudioTranscriptionsRequest`, `UsageVectorStoresRequest`, `UsageCodeInterpreterSessionsRequest`, `UsageFileSearchCallsRequest`, `UsageWebSearchCallsRequest`, `UsageCostsRequest`.

### `to_query_pairs` pattern (one per request type, hand-written — not a shared trait, to keep each type's field list visible and greppable, matching this crate's preference for explicit code over generic abstraction in request-building)

```rust
impl UsageCompletionsRequest {
    pub(crate) fn to_query_pairs(&self) -> Vec<(String, String)> {
        let mut pairs = Vec::new();
        pairs.push(("start_time".to_string(), self.start_time.to_string()));
        if let Some(v) = self.end_time { pairs.push(("end_time".to_string(), v.to_string())); }
        if let Some(v) = &self.bucket_width { pairs.push(("bucket_width".to_string(), v.clone())); }
        if let Some(v) = self.limit { pairs.push(("limit".to_string(), v.to_string())); }
        if let Some(v) = &self.page { pairs.push(("page".to_string(), v.clone())); }
        if let Some(v) = self.batch { pairs.push(("batch".to_string(), v.to_string())); }
        if let Some(values) = &self.project_ids {
            for v in values { pairs.push(("project_ids".to_string(), v.clone())); }
        }
        if let Some(values) = &self.api_key_ids {
            for v in values { pairs.push(("api_key_ids".to_string(), v.clone())); }
        }
        if let Some(values) = &self.models {
            for v in values { pairs.push(("models".to_string(), v.clone())); }
        }
        if let Some(values) = &self.group_by {
            for v in values { pairs.push(("group_by".to_string(), v.clone())); }
        }
        pairs
    }
}
```

Each field gets its own explicit `if let`/`for` block, one per field, in this style — no shared loop or trait abstraction, since each field's query-key name differs and 11 short methods don't justify the extra machinery a generic version would need.

### `client.rs` addition

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

Mirrors the existing `get<T, F>` exactly, replacing the generic `query: &F` parameter with a pre-built pairs slice. `openai_request`/`resolve_response` are the same private helpers `get`/`post`/`delete` already use — purely additive, zero risk to any existing method (confirmed by reading the current `get` implementation directly, same verification standard as every prior cross-cutting `client.rs` touch in this project).

### API surface

`OrganizationUsage<'a>` (already scaffolded, `#[allow(dead_code)]` to be removed) gains 11 methods, one per endpoint, each following the same shape:

```rust
pub async fn completions(&self, req: &usage::UsageCompletionsRequest) -> OpenAIResponse<usage::UsageResponse> {
    self.openai.get_with_pairs("/organization/usage/completions", &req.to_query_pairs()).await
}
```

Route paths confirmed via source: `/organization/usage/{completions, embeddings, moderations, images, audio_speeches, audio_transcriptions, vector_stores, code_interpreter_sessions, file_search_calls, web_search_calls}`, `/organization/costs` (confirmed distinct — NOT `/organization/usage/costs`; the Python resource method is named `costs()` on the `Usage` resource class for developer convenience, but its route is a top-level `/organization/costs`, not nested under `/organization/usage/`).

## Non-goals (explicit, deferred to later slices)

- **The rest of Organization/admin** (Projects + its 5 nested sub-resources) — sub-slice 4, not this one.
- **Any further generalization of the array-query-pairs fix** beyond Usage — the `get_with_pairs` client method and `to_query_pairs` pattern are introduced here because Usage is the first (and, per research this session, only known) resource needing them; not retrofitted onto Audit Logs' or Users' already-deferred array filters, which stay deferred exactly as recorded in sub-slices 1-2's memos. A future slice could revisit those now that the pattern exists, but that's a deliberate separate decision, not automatic.

## Testing

`wiremock` tests per endpoint (11 endpoints × ~1-2 tests each): a representative test asserting `to_query_pairs()` produces the correct repeated-key query string (e.g. `group_by=model&group_by=project_id`) via `wiremock`'s `query_param` matcher (which matches one key against one value — for a repeated key, register the matcher twice, once per expected value, since both must independently match against the same multi-valued query key) and deserializes a `UsageResponse` containing the correct `UsageResult` variant for that endpoint. `costs`' distinctive route (`/organization/costs`, not `/organization/usage/costs`) gets its own explicit path-assertion test. Since `UsageResult` is a `#[serde(tag = "object")]` internally-tagged enum, at least one test proves the tag-based dispatch correctly deserializes two different result variants from two different endpoints' mock responses into the correct enum variant — not just "deserializes successfully", but the correct *variant*.

## Versioning

Purely additive at the crate level, except the ONE new `client.rs` method (`get_with_pairs`) — confirmed non-breaking (new method, no existing method signature changed). Pre-1.0 minor bump alongside the rest of the accumulated Phase 2b work, per established practice.
