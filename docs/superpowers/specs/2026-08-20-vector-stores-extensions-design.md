# Vector Stores: File Batches, Files update/content, typed search filters — Design

Status: Approved
Date: 2026-08-20

## Context

Closes Vector Stores' three deferred pieces from its original core slice: File Batches (deferred entirely), Files' `update`/`content` (deferred), and `search`'s `filters` param's recursive grammar (kept as raw JSON). All confirmed via the `gh api` SDK-source technique.

## Goals

### File Batches (`apis::vector_stores::file_batches`, existing scaffold gets real methods)

- `VectorStoreFileBatches::create()` (`POST .../file_batches`), `VectorStoreFileBatch::retrieve()` (`GET .../file_batches/{batch_id}`), `::cancel()` (`POST .../file_batches/{batch_id}/cancel`), `::list_files()` (`GET .../file_batches/{batch_id}/files`). No `update`/`delete` — confirmed, a batch is an append-only ingestion operation.
- `CreateVectorStoreFileBatchRequest`: `file_ids: Option<Vec<String>>`, `files: Option<Vec<VectorStoreFileBatchFileEntry>>` (mutually exclusive per source — not enforced client-side, matches this crate's convention of not re-implementing server-side "one of" validation), `attributes: Option<HashMap<String, serde_json::Value>>` (reuses the existing type), `chunking_strategy: Option<ChunkingStrategy>` (reuses the existing type).
- `VectorStoreFileBatchFileEntry`: `file_id: String`, `attributes: Option<HashMap<String, serde_json::Value>>`, `chunking_strategy: Option<ChunkingStrategy>`.
- `VectorStoreFileBatchCounts`: `cancelled: i64`, `completed: i64`, `failed: i64`, `in_progress: i64`, `total: i64`.
- `VectorStoreFileBatch`: `id: String`, `created_at: i64`, `file_counts: VectorStoreFileBatchCounts`, `object: String`, `status: String` (`"in_progress"`/`"completed"`/`"cancelled"`/`"failed"`), `vector_store_id: String`.
- `ListVectorStoreFileBatchFilesRequest`: `after: Option<String>`, `before: Option<String>`, `filter: Option<String>`, `limit: Option<u32>`, `order: Option<SortOrder>`.
- List-files reuses the existing `ListVectorStoreFilesResponse` (same item shape, `VectorStoreFile`).

A new `VectorStoreFileBatch<'a>` ID-scoped accessor is added beyond the flat scaffold (which only modeled the collection level) — matching the established precedent (`VectorStoreFile<'a>`, `ContainerFile<'a>`, etc.).

### Files `update`/`content` (appended to the existing `VectorStoreFile<'a>`/`VectorStoreFiles<'a>`)

- `UpdateVectorStoreFileRequest`: `attributes: Option<HashMap<String, serde_json::Value>>` (confirmed `Required[Optional[...]]` in source — the field itself must be present in the request, but its value can be `null` to clear attributes; modeled as a plain, always-serialized `Option` field, NOT `#[serde(skip_serializing_if)]`, so `null` is sent explicitly when clearing).
- `content()`: `GET /vector_stores/{vector_store_id}/files/{file_id}/content` → `VectorStoreFileContentResponse { data: Vec<VectorStoreFileContentItem>, ... }` -- confirmed via source: `FileContentResponse` here is `{text: Option<String>, type: Option<String>}` and the endpoint actually returns a **list** of these (paginated content chunks, confirmed by checking the resource method's return type is a list page, not one object) — modeled as `VectorStoreFileContentResponse { data: Vec<VectorStoreFileContentItem>, has_more: bool, next_page: Option<String>, object: String }`.

### Typed `filters` (Vector Stores' `search`)

- `VectorStoreSearchFilter` — a recursive, `type`-tagged enum: `Eq { key, value }` | `Ne { key, value }` | `Gt { key, value }` | `Gte { key, value }` | `Lt { key, value }` | `Lte { key, value }` | `In { key, value }` | `Nin { key, value }` (all 8 confirmed comparison operators, `value: serde_json::Value` since the source confirms a heterogeneous `str | float | bool | List[str | float]`) plus `And { filters: Vec<VectorStoreSearchFilter> }` | `Or { filters: Vec<VectorStoreSearchFilter> }` (confirmed 2 compound operators). No `Box` needed — the recursion is already behind `Vec`, which is heap-allocated.
- `VectorStoreSearchRequest.filters` changes from `Option<serde_json::Value>` to `Option<VectorStoreSearchFilter>`, a deliberate breaking change (correctness-driven, matching this crate's established practice — the raw-JSON escape hatch is no longer needed now that the grammar is confirmed simple enough to fully type).

## Design notes

- All new types appended to the existing flat `interfaces/vector_stores.rs` (matching established colocation precedent) and `interfaces/vector_stores/` split files (`file_batches.rs`, `files.rs`) as appropriate — check the actual current file split before writing, since Vector Stores may already be a directory (unlike ChatKit/Evals/Conversations, which stayed flat).
- No `client.rs` changes.

## Testing

`wiremock` tests: File Batches (create/retrieve/cancel/list_files — 4), Files update/content (2), search filters (a nested `And`-of-`Eq`+`Gte` round-trip test proving the recursive structure serializes and deserializes correctly, replacing the prior raw-JSON test).

## Versioning

One deliberate breaking change (`filters`' type), otherwise purely additive. No `client.rs` changes. Closes all three of Vector Stores' remaining deferred pieces.
