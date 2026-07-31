# Phase 2a: Module Restructuring & Scaffolding — Design

Status: Approved
Date: 2026-07-31

## Context

Phase 1 (`docs/superpowers/specs/2026-07-29-rust-modernization-design.md`, `docs/superpowers/plans/2026-07-29-rust-modernization-plan.md`) modernized `rs_openai`'s Rust code quality without touching the OpenAI API surface. It shipped 2026-07-31 (merged to `master` at `f22a31d`).

Phase 2 updates the API surface itself. The project owner's direction, given while Phase 1 was still executing:
- Read OpenAI's current API reference (`https://developers.openai.com/api/reference/overview`) closely — it's organized in a category → resource → operation hierarchy (e.g. ChatKit → Sessions/Threads → individual operations).
- Adopt that same hierarchy for `rs_openai` itself, in both internal module organization and the public API surface callers use.
- No backward-compatibility constraint this round — delete unused fields, drop dead endpoints, no transition shims.

Four research passes were completed and committed before this design:
- `docs/superpowers/research/2026-07-31-openai-api-structure.md` — full category/resource/operation map of OpenAI's current API (~21 categories, ~45 resources, ~150 operations), cross-referenced against `rs_openai`'s current 17 modules.
- `docs/superpowers/research/2026-07-31-field-diff-chat-embeddings-moderations-models.md`
- `docs/superpowers/research/2026-07-31-field-diff-images-audio-files-fine-tuning.md`
- `docs/superpowers/research/2026-07-31-field-diff-assistants-cluster.md`

Headline finding: `rs_openai` has zero coverage of roughly half of OpenAI's current API (Responses — the now-recommended primary API — Conversations, Vector Stores, ChatKit, Containers, Evals, Videos, Webhooks, and the entire Organization/admin tier), and most of what it does have is missing fields added over the last two years (tool calling on Chat being the single biggest gap).

This document specs the first slice of Phase 2 work: **restructuring the crate's module layout and public API shape to mirror OpenAI's hierarchy, and scaffolding every missing category so its file structure exists** — without yet filling in the new field-level content or implementing the new categories' actual request/response types. That's deliberate, matching the project owner's own requested sequencing (crawl → structure → restructure paths → scaffold → *then* implement). Filling in scaffolded files and applying the field-level diffs are separate follow-on specs.

## Goals

- Reorganize `apis`/`interfaces` so that any OpenAI resource with real sub-resources (Vector Stores → Files/File Batches, ChatKit → Sessions/Threads, Fine Tuning → Checkpoints → Permissions, etc.) becomes a Rust module directory mirroring that nesting. Resources with no sub-resources stay flat single files — no forced directories for their own sake.
- Give every ID-addressable resource both a plural collection accessor (`client.vector_stores()` — create/list, no ID) and a singular ID-scoped accessor (`client.vector_store(id)` — retrieve/update/delete plus sub-resource accessors chained off it with no ID repetition), per the pattern already confirmed with the project owner.
- Give the Organization/admin tier its own top-level namespace (`client.organization()...`), separate from the "call the model" surface.
- Create file skeletons — module wiring plus accessor structs with constructors, no request/response types or endpoint methods yet — for every category currently missing entirely: Responses, Conversations, Vector Stores, ChatKit, Containers, Evals, Videos, Webhooks, Organization (audit logs, admin API keys, groups, invites, projects + their sub-resources, roles, users). Realtime stays out of scope (already deferred, needs its own connection-oriented design separate from this HTTP client).
- Delete `edits` and `engines` outright (dead on OpenAI's side since 2022–2024; no reason to keep or scaffold them).
- Leave the Assistants cluster (`assistants`/`threads`/`messages`/`runs`) in its current flat structure, unrestructured, clearly marked deprecated — it sunsets 2026-08-26, so investing in reorganizing it isn't worth it.
- Fix the one confirmed, independent bug already found during research while touching the relevant file anyway: `CreateBatchRequest.meta` needs `#[serde(rename = "metadata")]` (currently serializes to the wrong wire key, silently dropping batch metadata).

## Non-goals (explicit, deferred to later Phase 2 specs)

- Adding any new fields to existing request/response types (tool calling, multimodal chat content, `dimensions` on embeddings, `model` on images, fine-tuning `method`, etc. — all already catalogued in the field-diff research docs).
- Implementing the actual request/response types and endpoint logic for any newly-scaffolded category (Responses, Conversations, Vector Stores, ChatKit, Containers, Evals, Videos, Webhooks, Organization).
- Realtime API (any form).
- Anything about the Assistants cluster beyond marking it deprecated in place.

## Design

### Module layout pattern

For a category with sub-resources, e.g. Vector Stores:

```
rs_openai/src/apis/vector_stores/
  mod.rs          # VectorStores<'a> (collection ops), VectorStore<'a> (ID-scoped ops + sub-resource accessors)
  files.rs        # VectorStoreFiles<'a>
  file_batches.rs # VectorStoreFileBatches<'a>

rs_openai/src/interfaces/vector_stores/
  mod.rs          # CreateVectorStoreRequest, VectorStoreResponse, etc.
  files.rs        # request/response types for the Files sub-resource
  file_batches.rs # request/response types for the File Batches sub-resource
```

`apis::vector_stores` re-exports `interfaces::vector_stores::*` the same way every Phase-1-migrated module already does (`pub use crate::interfaces::vector_stores::*;`), and each sub-module does the same one level down for its own interface sub-module. Flat categories (Embeddings, Moderations, Models, Chat, legacy Completions, Files, Uploads, Batches — none of which have sub-resources per the structure-tree research) are untouched by this pattern; they stay exactly as Phase 1 left them, single `apis/<name>.rs` + `interfaces/<name>.rs` files.

### Accessor pattern

```rust
impl OpenAI {
    pub fn vector_stores(&self) -> VectorStores<'_> {
        VectorStores { openai: self }
    }
    pub fn vector_store(&self, id: impl Into<String>) -> VectorStore<'_> {
        VectorStore { openai: self, id: id.into() }
    }
}

pub struct VectorStores<'a> { openai: &'a OpenAI }
impl<'a> VectorStores<'a> {
    pub async fn create(&self, req: &CreateVectorStoreRequest) -> OpenAIResponse<VectorStoreResponse> { /* ... */ }
    pub async fn list(&self, req: &ListVectorStoresRequest) -> OpenAIResponse<ListVectorStoresResponse> { /* ... */ }
}

pub struct VectorStore<'a> { openai: &'a OpenAI, id: String }
impl<'a> VectorStore<'a> {
    pub async fn retrieve(&self) -> OpenAIResponse<VectorStoreResponse> { /* ... */ }
    pub async fn update(&self, req: &UpdateVectorStoreRequest) -> OpenAIResponse<VectorStoreResponse> { /* ... */ }
    pub async fn delete(&self) -> OpenAIResponse<DeleteVectorStoreResponse> { /* ... */ }
    pub async fn search(&self, req: &SearchVectorStoreRequest) -> OpenAIResponse<SearchVectorStoreResponse> { /* ... */ }

    pub fn files(&self) -> VectorStoreFiles<'a> {
        VectorStoreFiles { openai: self.openai, vector_store_id: self.id.clone() }
    }
    pub fn file_batches(&self) -> VectorStoreFileBatches<'a> {
        VectorStoreFileBatches { openai: self.openai, vector_store_id: self.id.clone() }
    }
}
```

This scales to N levels the same way (see Assistants-shaped example: `client.thread(id).run(run_id).steps().list()` — not applied to the actual, soon-dead Assistants cluster, but the same mechanism a genuinely 3-tier-deep resource like ChatKit Threads → Items, or ChatKit itself, will use). Cloning a short ID string when descending a level is cheap and acceptable.

Collection-only resources with no ID-addressable single item (e.g. Moderations — nothing is fetchable by ID) keep only the plural form; no singular accessor is added where OpenAI's API doesn't have one.

### Scaffolding definition

"Scaffolded" means: the module file exists, is wired into its parent `mod.rs` and ultimately into `client.rs`, declares its accessor struct(s) with a `new`/constructor, and compiles — but has no endpoint methods and no request/response types yet. This is an honest skeleton, not a placeholder pretending to be finished: a category like `Responses` will exist as `apis::responses::Responses<'a>`/`apis::responses::Response<'a>` (or similar, exact shape decided at implementation time per that category's actual sub-resource structure) with a doc comment stating it's not yet implemented, and zero methods — calling code simply can't reach it yet, which is the correct and only honest state for genuinely unimplemented functionality.

### Organization/admin tier

`client.organization()` → `OrganizationApi<'a>` (or similar name, finalized at implementation) with its own nested accessors: `audit_logs()`, `admin_api_keys()`, `usage()`, `groups()` (+ `group(id).users()`), `invites()`, `projects()` (+ `project(id).api_keys()`/`.groups()`/`.rate_limits()`/`.service_accounts()`/`.users()`), `roles()`, `users()` (+ `user(id).roles()`). This whole tier is scaffolded only, per Non-goals — no implementation yet.

### Assistants cluster handling

No structural change. Add a module-level or type-level deprecation notice (exact form — `#[deprecated]` attribute vs. prominent doc comment — decided at implementation time, since `#[deprecated]` on a whole module tends to cascade unhelpfully into every internal usage) referencing the 2026-08-26 sunset and pointing at Responses + Conversations as the replacement, per the recommendation already established in Phase 1 planning and reaffirmed in the Phase 2 research.

### `edits`/`engines` removal

Delete `apis/edits.rs`, `apis/engines.rs`, `interfaces/edits.rs`, `interfaces/engines.rs`, their entries in `apis/mod.rs`/`interfaces/mod.rs`, their `OpenAI` accessor methods, and the `examples/edits`/`examples/engines` example crates (including their workspace-member entries). No replacement, no deprecation shim — they don't exist in OpenAI's API anymore.

### Batch `metadata` bug fix

While restructuring/touching `interfaces/batch.rs` in this pass, add `#[serde(rename = "metadata")]` to `CreateBatchRequest.meta` (or rename the Rust field itself to `metadata` with no rename attribute needed — implementation-time call). This is a pure, already-confirmed bug fix independent of the rest of Phase 2's new-field work, and there's no reason to leave it broken until the later field-upgrade pass when this file is already being touched.

## Versioning

This is another pre-1.0 breaking-change round (module paths change, several `OpenAI` accessor method signatures change shape from today's flat style to plural/singular pairs where applicable, `edits`/`engines` disappear). Ships as a minor version bump per Cargo's pre-1.0 convention, exact number decided at implementation time.
