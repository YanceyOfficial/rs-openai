# Phase 2a: Module Restructuring & Scaffolding Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Reorganize `rs_openai`'s module layout to mirror OpenAI's category → resource → operation API hierarchy, add ID-scoped accessor chaining for sub-resources, remove dead endpoints, and create file skeletons for every category the crate doesn't implement yet (except Realtime) — with zero new request/response field content beyond what's specified below.

**Architecture:** Categories with real sub-resources become Rust module directories (`apis/<category>/mod.rs` + one file per sub-resource, mirrored under `interfaces/`); categories without sub-resources stay single flat files. Every ID-addressable resource gets a plural collection accessor (`client.<category>()`) and, where OpenAI's API has a single-item route, a singular ID-scoped accessor (`client.<category_singular>(id)`) that sub-resource accessors chain off with no ID repetition. Newly-scaffolded categories get their accessor structs and constructors wired all the way from `client.rs` down, with zero endpoint methods — an honest "not implemented yet" skeleton, not a placeholder pretending otherwise.

**Tech Stack:** Same as Phase 1 (Rust edition 2024, `bon` builders, `reqwest`/`reqwest-middleware`, `wiremock` tests) — this phase adds no new dependencies.

## Global Constraints

- Spec: `docs/superpowers/specs/2026-07-31-phase2-restructuring-design.md`. Research backing every category/gap claim below: `docs/superpowers/research/2026-07-31-openai-api-structure.md` and the three field-diff docs (`2026-07-31-field-diff-chat-embeddings-moderations-models.md`, `2026-07-31-field-diff-images-audio-files-fine-tuning.md`, `2026-07-31-field-diff-assistants-cluster.md`).
- No backward-compatibility constraint this round (explicit project-owner direction) — this is a breaking-change pass, ships as a pre-1.0 minor version bump.
- **Non-goals, do not do in this plan:** adding any new request/response fields to existing types (tool calling, multimodal content, `dimensions`, `model` on images, fine-tuning `method`, etc.); implementing actual endpoint logic or full request/response types for any newly-scaffolded category; Realtime API in any form; restructuring the Assistants cluster's internals (it gets a deprecation notice only, no file moves).
- Every task's "done" check includes `cargo build --workspace --all-targets`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace`, and `cargo fmt --all -- --check`.
- Every file this plan touches or creates was read/verified in full before this plan was written (either directly, or via the committed research docs).
- Client accessor pattern (established in the approved spec, applied throughout): a plural method returns a collection-ops struct; a singular method takes an ID and returns an ID-scoped struct with retrieve/update/delete plus `.sub_resource()` accessors that carry the parent ID forward without repeating it. Both structs hold `openai: &'a OpenAI`; ID-scoped structs additionally hold the ID(s) as owned `String`s (cheap to clone when descending a level).

---

### Task 1: Delete `edits`/`engines`, fix the `CreateBatchRequest` metadata bug

**Files:**
- Delete: `rs_openai/src/apis/edits.rs`, `rs_openai/src/interfaces/edits.rs`, `rs_openai/src/apis/engines.rs`, `rs_openai/src/interfaces/engines.rs`
- Delete: `examples/edits/`, `examples/engines/` (whole directories)
- Modify: `rs_openai/src/apis/mod.rs`, `rs_openai/src/interfaces/mod.rs`, `rs_openai/src/client.rs`, `Cargo.toml` (root)
- Modify: `rs_openai/src/interfaces/batch.rs`

**Interfaces:**
- Consumes: nothing from later tasks.
- Produces: nothing later tasks depend on — this task is fully self-contained.

`edits` and `engines` map to OpenAI endpoints shut down in 2024-01-04 and 2022-12-03 respectively (confirmed in `2026-07-29-rust-modernization-design.md` during Phase 1 and reconfirmed in the Phase 2 structure-tree research) — there is no live endpoint to keep them working against. `CreateBatchRequest.meta` has no `#[serde(rename = "metadata")]`, so it currently serializes to the wire key `"meta"` instead of the real API field `"metadata"` — a confirmed, independent bug found during Phase 2 research (`2026-07-31-field-diff-images-audio-files-fine-tuning.md`), unrelated to the rest of this task but fixed here since nothing else in this plan touches `batch.rs`.

- [ ] **Step 1: Delete the edits and engines files**

```bash
git rm rs_openai/src/apis/edits.rs rs_openai/src/interfaces/edits.rs \
       rs_openai/src/apis/engines.rs rs_openai/src/interfaces/engines.rs
git rm -r examples/edits examples/engines
```

- [ ] **Step 2: Remove their entries from `apis/mod.rs` and `interfaces/mod.rs`**

Both files currently read (identical content):

```rust
pub mod assistants;
pub mod audio;
pub mod batch;
pub mod chat;
pub mod completions;
pub mod edits;
pub mod embeddings;
pub mod engines;
pub mod files;
pub mod fine_tuning;
pub mod images;
pub mod messages;
pub mod models;
pub mod moderations;
pub mod runs;
pub mod threads;
pub mod uploads;
```

Remove the `pub mod edits;` and `pub mod engines;` lines from both `rs_openai/src/apis/mod.rs` and `rs_openai/src/interfaces/mod.rs`, leaving:

```rust
pub mod assistants;
pub mod audio;
pub mod batch;
pub mod chat;
pub mod completions;
pub mod embeddings;
pub mod files;
pub mod fine_tuning;
pub mod images;
pub mod messages;
pub mod models;
pub mod moderations;
pub mod runs;
pub mod threads;
pub mod uploads;
```

- [ ] **Step 3: Remove `edits`/`engines` from `rs_openai/src/client.rs`**

Remove `edits` and `engines` from the `use crate::apis::{...}` import block (currently `assistants, audio, batch, chat, completions, edits, embeddings, engines, files, fine_tuning, images, messages, models, moderations, runs, threads, uploads`, becomes `assistants, audio, batch, chat, completions, embeddings, files, fine_tuning, images, messages, models, moderations, runs, threads, uploads`), and remove these two accessor methods entirely:

```rust
    pub fn edits(&self) -> edits::Edits<'_> {
        edits::Edits::new(self)
    }
```

```rust
    pub fn engines(&self) -> engines::Engines<'_> {
        engines::Engines::new(self)
    }
```

- [ ] **Step 4: Remove `edits`/`engines` from the root `Cargo.toml` workspace members**

Current `members` list:

```toml
members = [
    "rs_openai",
    "examples/audio",
    "examples/chat",
    "examples/chat_stream_with_actix_web",
    "examples/completions",
    "examples/edits",
    "examples/embeddings",
    "examples/engines",
    "examples/files",
    "examples/fine_tuning",
    "examples/images",
    "examples/models",
    "examples/moderations",
]
```

Remove `"examples/edits",` and `"examples/engines",`:

```toml
members = [
    "rs_openai",
    "examples/audio",
    "examples/chat",
    "examples/chat_stream_with_actix_web",
    "examples/completions",
    "examples/embeddings",
    "examples/files",
    "examples/fine_tuning",
    "examples/images",
    "examples/models",
    "examples/moderations",
]
```

- [ ] **Step 5: Fix the `CreateBatchRequest` metadata bug**

In `rs_openai/src/interfaces/batch.rs`, rename the field from `meta` to `metadata` (no rename attribute needed once the Rust name matches the wire name):

```rust
    /// Optional custom metadata for the batch.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
```

This is the only change to `CreateBatchRequest`; `ListBatchRequest`, `RequestCounts`, `BatchResponse`, `ListBatchResponse` are untouched.

- [ ] **Step 6: Fix the one call site that builds a `CreateBatchRequest` with the old field name**

Run: `grep -rn "\.meta(" rs_openai/src/apis/batch.rs`
Expected: no matches (the bon builder's field-setter method is named after the struct field, so once the field is renamed, `.meta(...)` becomes `.metadata(...)` automatically — this step is a verification, not a code change, unless the grep finds a test or example that still calls `.meta(...)`, in which case update it to `.metadata(...)`).

- [ ] **Step 7: Verify**

Run: `cargo build --workspace --all-targets`
Expected: clean build, no references to `edits`/`engines`/`meta` remain.

Run: `cargo clippy --workspace --all-targets --all-features -- -D warnings`
Expected: exit 0.

Run: `cargo test --workspace`
Expected: all prior tests still pass (this task removed 2 test files along with `edits.rs`/`engines.rs` — that's expected, not a regression).

Run: `cargo fmt --all -- --check`
Expected: clean (run `cargo fmt --all` first if not).

- [ ] **Step 8: Commit**

```bash
git add -A
git commit -m "$(cat <<'EOF'
feat!: drop edits/engines, fix CreateBatchRequest metadata field

edits (shut down 2024-01-04) and engines (shut down 2022-12-03) map to
no live OpenAI endpoint — removed outright, no compat shim, per the
Phase 2 no-backward-compatibility direction.

CreateBatchRequest.meta lacked #[serde(rename = "metadata")], so it
serialized to the wrong wire key and OpenAI silently dropped any batch
metadata set through this crate. Renamed the field to match.
EOF
)"
```

---

### Task 2: Restructure Fine Tuning + Audio into directories, scaffold their new sub-resources

**Files:**
- Move: `rs_openai/src/apis/fine_tuning.rs` → `rs_openai/src/apis/fine_tuning/mod.rs`
- Create: `rs_openai/src/apis/fine_tuning/checkpoints/mod.rs`, `rs_openai/src/apis/fine_tuning/checkpoints/permissions.rs`
- Move: `rs_openai/src/apis/audio.rs` → `rs_openai/src/apis/audio/mod.rs`
- Create: `rs_openai/src/apis/audio/voice_consents.rs`, `rs_openai/src/apis/audio/voices.rs`

No `client.rs` changes in this task — `checkpoint(id)`/`voice_consents()`/`voice_consent(id)`/`voices()` are reached through the existing `client.fine_tuning()`/`client.audio()` top-level accessors, not as new top-level methods.

**Not moved:** `rs_openai/src/interfaces/fine_tuning.rs` and `rs_openai/src/interfaces/audio.rs` stay exactly where they are, as flat files. Nothing in this task (or anywhere else in this plan) adds an `interfaces/fine_tuning/*` or `interfaces/audio/*` sub-file — the new sub-resources are `apis/`-only scaffolds per this task's own scaffold convention below — so moving these into directories now would be pure churn with no file to justify it. `apis::fine_tuning`'s existing `pub use crate::interfaces::fine_tuning::*;` re-export keeps working unchanged either way, since it doesn't care whether `interfaces::fine_tuning` is a file or a directory.

**Interfaces:**
- Consumes: nothing from Task 1.
- Produces: `client.fine_tuning().checkpoint(id)` → `FineTuningCheckpoint<'a>` with `.permissions()` → `FineTuningCheckpointPermissions<'a>` (scaffold, no methods). `client.audio().voice_consents()` → `AudioVoiceConsents<'a>` (scaffold), `client.audio().voice_consent(id)` → `AudioVoiceConsent<'a>` (scaffold), `client.audio().voices()` → `AudioVoices<'a>` (scaffold). No later task depends on these exact names, but keep them — Task 8's final verification checks the whole crate builds together.

**Scaffold convention used throughout this plan:** a struct with no endpoint methods yet (a "leaf" scaffold) gets `#[allow(dead_code)]` on its definition, since nothing in the crate reads its private fields until real methods are added — this is standard, correct practice for intentionally-unfinished code, not a code smell to fix. A struct that has at least one real accessor method (even if that method just returns another scaffold) does NOT need the allow, since its fields ARE read. Newly-scaffolded resources with no existing implementation get an `apis/` file only — no `interfaces/` file, since there's nothing to define there until real request/response types are added at implementation time.

Fine Tuning Checkpoint Permissions (`/v1/fine_tuning/checkpoints/{checkpoint}/permissions`) is a resource family alongside Jobs, not nested under a job — so it hangs off a new `fine_tuning().checkpoint(id)` accessor, parallel to the existing (unchanged) Jobs methods. Audio Voice Consents and Voices are entirely new sub-resources of the existing Audio category — Voice Consents has full CRUD by ID (confirmed in the field-diff research), Voices only has a confirmed `create` operation so far, so it gets a collection-only scaffold for now (a singular accessor can be added when its exact operation set is confirmed during implementation).

- [ ] **Step 1: Move Fine Tuning's `apis/` file into a directory, unchanged**

```bash
mkdir -p rs_openai/src/apis/fine_tuning
git mv rs_openai/src/apis/fine_tuning.rs rs_openai/src/apis/fine_tuning/mod.rs
```

`rs_openai/src/apis/mod.rs` needs no change for this move — `pub mod fine_tuning;` resolves to `fine_tuning/mod.rs` identically to how it resolved to `fine_tuning.rs` before. `rs_openai/src/interfaces/fine_tuning.rs` is untouched (see "Not moved" above).

- [ ] **Step 2: Add the `checkpoint` accessor and its scaffolded sub-resource**

At the top of `rs_openai/src/apis/fine_tuning/mod.rs`, add the new submodule declaration right after the existing `use` block:

```rust
pub mod checkpoints;
```

At the end of `impl<'a> FineTuning<'a>` (after the existing `cancel` method, before the closing `}`), add:

```rust

    /// Scoped accessor for a specific fine-tuning checkpoint's operations (e.g. sharing
    /// permissions). Not yet implemented — see
    /// docs/superpowers/research/2026-07-31-field-diff-images-audio-files-fine-tuning.md.
    pub fn checkpoint(&self, id: impl Into<String>) -> checkpoints::FineTuningCheckpoint<'a> {
        checkpoints::FineTuningCheckpoint::new(self.openai, id.into())
    }
```

Create `rs_openai/src/apis/fine_tuning/checkpoints/mod.rs`:

```rust
//! A specific fine-tuning checkpoint's operations. Not yet implemented — see
//! docs/superpowers/research/2026-07-31-field-diff-images-audio-files-fine-tuning.md.

pub mod permissions;

use crate::client::OpenAI;

pub struct FineTuningCheckpoint<'a> {
    openai: &'a OpenAI,
    id: String,
}

impl<'a> FineTuningCheckpoint<'a> {
    pub(crate) fn new(openai: &'a OpenAI, id: String) -> Self {
        Self { openai, id }
    }

    /// Scoped accessor for which organizations can access this checkpoint. Not yet
    /// implemented.
    pub fn permissions(&self) -> permissions::FineTuningCheckpointPermissions<'a> {
        permissions::FineTuningCheckpointPermissions::new(self.openai, self.id.clone())
    }
}
```

Create `rs_openai/src/apis/fine_tuning/checkpoints/permissions.rs`:

```rust
//! Grants/revokes other organizations' access to a fine-tuning checkpoint
//! (`POST`/`GET`/`DELETE .../fine_tuning/checkpoints/{checkpoint}/permissions`,
//! admin-key-only). Not yet implemented — see
//! docs/superpowers/research/2026-07-31-field-diff-images-audio-files-fine-tuning.md.

use crate::client::OpenAI;

#[allow(dead_code)]
pub struct FineTuningCheckpointPermissions<'a> {
    openai: &'a OpenAI,
    checkpoint_id: String,
}

impl<'a> FineTuningCheckpointPermissions<'a> {
    pub(crate) fn new(openai: &'a OpenAI, checkpoint_id: String) -> Self {
        Self {
            openai,
            checkpoint_id,
        }
    }
}
```

- [ ] **Step 3: Move Audio's `apis/` file into a directory, unchanged**

```bash
mkdir -p rs_openai/src/apis/audio
git mv rs_openai/src/apis/audio.rs rs_openai/src/apis/audio/mod.rs
```

Same as Step 1 — no `mod.rs` declaration changes needed elsewhere for this move alone. `rs_openai/src/interfaces/audio.rs` is untouched (see "Not moved" at the top of this task).

- [ ] **Step 4: Add the Voice Consents and Voices accessors**

At the top of `rs_openai/src/apis/audio/mod.rs`, add after the existing `use` block:

```rust
pub mod voice_consents;
pub mod voices;
```

At the end of `impl<'a> Audio<'a>` (after the existing `is_json_type` private method — actually add these as public methods before the private helper methods, keeping the existing method order otherwise unchanged), add:

```rust

    /// Scoped accessor for voice-cloning consent records. Not yet implemented — see
    /// docs/superpowers/research/2026-07-31-field-diff-images-audio-files-fine-tuning.md.
    pub fn voice_consents(&self) -> voice_consents::AudioVoiceConsents<'a> {
        voice_consents::AudioVoiceConsents::new(self.openai)
    }

    /// Scoped accessor for a specific voice-cloning consent record. Not yet implemented.
    pub fn voice_consent(&self, id: impl Into<String>) -> voice_consents::AudioVoiceConsent<'a> {
        voice_consents::AudioVoiceConsent::new(self.openai, id.into())
    }

    /// Scoped accessor for custom voice registration. Not yet implemented.
    pub fn voices(&self) -> voices::AudioVoices<'a> {
        voices::AudioVoices::new(self.openai)
    }
```

Create `rs_openai/src/apis/audio/voice_consents.rs`:

```rust
//! Consent records for voice cloning / custom-voice features
//! (`/v1/audio/voice_consents`). Not yet implemented — see
//! docs/superpowers/research/2026-07-31-field-diff-images-audio-files-fine-tuning.md.

use crate::client::OpenAI;

#[allow(dead_code)]
pub struct AudioVoiceConsents<'a> {
    openai: &'a OpenAI,
}

impl<'a> AudioVoiceConsents<'a> {
    pub(crate) fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }
}

#[allow(dead_code)]
pub struct AudioVoiceConsent<'a> {
    openai: &'a OpenAI,
    id: String,
}

impl<'a> AudioVoiceConsent<'a> {
    pub(crate) fn new(openai: &'a OpenAI, id: String) -> Self {
        Self { openai, id }
    }
}
```

Create `rs_openai/src/apis/audio/voices.rs`:

```rust
//! Custom voice registration (`/v1/audio/voices`). Not yet implemented — see
//! docs/superpowers/research/2026-07-31-field-diff-images-audio-files-fine-tuning.md.
//! Only `create` is confirmed so far; add a singular, ID-scoped `AudioVoice` accessor
//! here once the full operation set is confirmed during implementation.

use crate::client::OpenAI;

#[allow(dead_code)]
pub struct AudioVoices<'a> {
    openai: &'a OpenAI,
}

impl<'a> AudioVoices<'a> {
    pub(crate) fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }
}
```

- [ ] **Step 5: Verify**

Run: `cargo build --workspace --all-targets`
Expected: clean build.

Run: `cargo clippy --workspace --all-targets --all-features -- -D warnings`
Expected: exit 0 (confirms the `#[allow(dead_code)]` placements are correct and sufficient).

Run: `cargo test --workspace`
Expected: all existing Fine Tuning and Audio tests still pass unchanged (the move doesn't touch their content).

Run: `cargo fmt --all -- --check`
Expected: clean.

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "$(cat <<'EOF'
feat: restructure fine_tuning/audio into directories, scaffold new sub-resources

Moves fine_tuning.rs and audio.rs into fine_tuning/mod.rs and
audio/mod.rs (content unchanged) to make room for their new
sub-resources per OpenAI's current API: Fine Tuning Checkpoint
Permissions and Audio Voice Consents/Voices. Scaffolds only — no
request/response types or endpoint logic yet, per the Phase 2a design.
EOF
)"
```

---

### Task 3: Deprecation notices on the Assistants cluster

**Files:**
- Modify: `rs_openai/src/apis/assistants.rs`, `rs_openai/src/apis/threads.rs`, `rs_openai/src/apis/messages.rs`, `rs_openai/src/apis/runs.rs`

**Interfaces:**
- Consumes: nothing from earlier tasks.
- Produces: nothing later tasks depend on — this task only edits doc comments, no code shape changes.

Per the design spec: no structural change to this cluster, since it sunsets 2026-08-26 and OpenAI's own migration guidance points at Responses + Conversations as the replacement. Use a **prominent doc comment**, not the `#[deprecated]` attribute — `#[deprecated]` on `Assistants`/`Threads`/`Messages`/`Runs` would make every internal caller (client.rs's accessor methods, and every test in each file's own `#[cfg(test)] mod tests`) trigger the `deprecated` lint, which is a hard error under this crate's `-D warnings` clippy gate. A doc comment is visible in rustdoc and IDE tooltips without that cascade.

- [ ] **Step 1: Add the deprecation notice to `assistants.rs`**

Replace the file's opening doc comment and struct doc comment:

```rust
//! Build assistants that can call models and use tools to perform tasks.
//!
//! [Get started with the Assistants API](https://platform.openai.com/docs/assistants)

use crate::client::OpenAI;
use crate::interfaces::assistants;
pub use crate::interfaces::assistants::*;
use crate::shared::response_wrapper::OpenAIResponse;

/// The Assistants API is deprecated and sunsets **2026-08-26**. New integrations should use
/// the Responses API (`client.responses()`) combined with the Conversations API
/// (`client.conversations()`) instead — see OpenAI's Assistants migration guide. This module
/// is kept in its current flat structure and receives no further feature work.
pub struct Assistants<'a> {
    openai: &'a OpenAI,
}
```

- [ ] **Step 2: Add the deprecation notice to `threads.rs`**

Replace the file's opening doc comment and struct doc comment:

```rust
//! Create threads that assistants can interact with.
//!
//! Related guide: [Assistants](https://platform.openai.com/docs/assistants/overview)

use crate::client::OpenAI;
use crate::interfaces::threads;
pub use crate::interfaces::threads::*;
use crate::shared::response_wrapper::OpenAIResponse;

/// The Assistants API is deprecated and sunsets **2026-08-26**. New integrations should use
/// the Responses API (`client.responses()`) combined with the Conversations API
/// (`client.conversations()`) instead — see OpenAI's Assistants migration guide. This module
/// is kept in its current flat structure and receives no further feature work.
pub struct Threads<'a> {
    openai: &'a OpenAI,
}
```

- [ ] **Step 3: Add the deprecation notice to `messages.rs`**

Replace the file's opening doc comment and struct doc comment:

```rust
//! Create messages that assistants can interact with.
//!
//! Related guide: [Assistants](https://platform.openai.com/docs/assistants/overview)

use crate::client::OpenAI;
use crate::interfaces::messages;
pub use crate::interfaces::messages::*;
use crate::shared::response_wrapper::OpenAIResponse;

/// The Assistants API is deprecated and sunsets **2026-08-26**. New integrations should use
/// the Responses API (`client.responses()`) combined with the Conversations API
/// (`client.conversations()`) instead — see OpenAI's Assistants migration guide. This module
/// is kept in its current flat structure and receives no further feature work.
pub struct Messages<'a> {
    openai: &'a OpenAI,
}
```

- [ ] **Step 4: Add the deprecation notice to `runs.rs`**

Replace the file's opening doc comment and struct doc comment:

```rust
//! Create runs that assistants can interact with.
//!
//! Related guide: [Assistants](https://platform.openai.com/docs/assistants/overview)

use crate::client::OpenAI;
use crate::interfaces::runs;
pub use crate::interfaces::runs::*;
use crate::shared::response_wrapper::OpenAIResponse;

/// The Assistants API is deprecated and sunsets **2026-08-26**. New integrations should use
/// the Responses API (`client.responses()`) combined with the Conversations API
/// (`client.conversations()`) instead — see OpenAI's Assistants migration guide. This module
/// is kept in its current flat structure and receives no further feature work. Also note:
/// this module is missing "Submit Tool Outputs" and "Run Steps" (see
/// `docs/superpowers/research/2026-07-31-field-diff-assistants-cluster.md`) and will not gain
/// them, given the sunset.
pub struct Runs<'a> {
    openai: &'a OpenAI,
}
```

- [ ] **Step 5: Verify**

Run: `cargo build --workspace --all-targets`
Expected: clean build.

Run: `cargo clippy --workspace --all-targets --all-features -- -D warnings`
Expected: exit 0 — confirms the doc-comment-only approach doesn't trigger the `deprecated` lint anywhere.

Run: `cargo test --workspace`
Expected: all existing Assistants/Threads/Messages/Runs tests still pass unchanged.

Run: `cargo fmt --all -- --check`
Expected: clean.

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "$(cat <<'EOF'
docs: mark the Assistants cluster deprecated ahead of its 2026-08-26 sunset

Adds a prominent doc comment to Assistants/Threads/Messages/Runs
pointing callers at Responses + Conversations, OpenAI's own migration
guidance. Uses a doc comment rather than #[deprecated] to avoid
cascading deprecation warnings into every internal caller under this
crate's -D warnings clippy gate. No structural change — not worth
restructuring a cluster that's going away.
EOF
)"
```

---

### Task 4: Scaffold Vector Stores, ChatKit, Containers, Evals + Graders

**Files:**
- Create: `rs_openai/src/apis/vector_stores/mod.rs`, `rs_openai/src/apis/vector_stores/files.rs`, `rs_openai/src/apis/vector_stores/file_batches.rs`
- Create: `rs_openai/src/apis/chatkit/mod.rs`, `rs_openai/src/apis/chatkit/sessions.rs`, `rs_openai/src/apis/chatkit/threads.rs`
- Create: `rs_openai/src/apis/containers/mod.rs`, `rs_openai/src/apis/containers/files.rs`
- Create: `rs_openai/src/apis/evals/mod.rs`, `rs_openai/src/apis/evals/runs.rs`, `rs_openai/src/apis/graders.rs`
- Modify: `rs_openai/src/apis/mod.rs`, `rs_openai/src/client.rs`

**Interfaces:**
- Consumes: nothing from earlier tasks.
- Produces: `client.vector_stores()`/`client.vector_store(id)` (`.files()`, `.file_batches()`); `client.chatkit()` (`.sessions()`, `.session(id)`, `.threads()`, `.thread(id)`, and `.thread(id).items()`); `client.containers()`/`client.container(id)` (`.files()`); `client.evals()`/`client.eval(id)` (`.runs()`, `.run(run_id)`); `client.graders()`. All scaffolds — no endpoint methods. No later task depends on these names, but Task 8's final verification checks the whole crate builds together.

Per the design spec's Vector Stores worked example, these four categories get real module directories because they have genuine sub-resources (Files/File Batches under Vector Stores, Sessions/Threads under ChatKit — the exact 3-tier example cited when this restructuring was requested — Files under Containers, Runs under Evals). Graders is a standalone resource per the structure research (not nested under Evals), with an unconfirmed operation set, so it stays a flat single file with only a collection-shaped placeholder. Every leaf struct (no accessor methods) gets `#[allow(dead_code)]` per the Task 2 convention; structs with at least one accessor method reading their fields don't need it.

- [ ] **Step 1: Create the Vector Stores scaffold**

Create `rs_openai/src/apis/vector_stores/mod.rs`:

```rust
//! Semantic search over uploaded files, used by the Responses API's `file_search` tool. Not
//! yet implemented — see docs/superpowers/research/2026-07-31-openai-api-structure.md.

pub mod file_batches;
pub mod files;

use crate::client::OpenAI;

#[allow(dead_code)]
pub struct VectorStores<'a> {
    openai: &'a OpenAI,
}

impl<'a> VectorStores<'a> {
    pub(crate) fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }
}

pub struct VectorStore<'a> {
    openai: &'a OpenAI,
    id: String,
}

impl<'a> VectorStore<'a> {
    pub(crate) fn new(openai: &'a OpenAI, id: String) -> Self {
        Self { openai, id }
    }

    /// Scoped accessor for files attached to this vector store. Not yet implemented.
    pub fn files(&self) -> files::VectorStoreFiles<'a> {
        files::VectorStoreFiles::new(self.openai, self.id.clone())
    }

    /// Scoped accessor for batch file-attach operations on this vector store. Not yet
    /// implemented.
    pub fn file_batches(&self) -> file_batches::VectorStoreFileBatches<'a> {
        file_batches::VectorStoreFileBatches::new(self.openai, self.id.clone())
    }
}
```

Create `rs_openai/src/apis/vector_stores/files.rs`:

```rust
//! A vector store's attached files (`/v1/vector_stores/{vector_store_id}/files`). Not yet
//! implemented — see docs/superpowers/research/2026-07-31-openai-api-structure.md.

use crate::client::OpenAI;

#[allow(dead_code)]
pub struct VectorStoreFiles<'a> {
    openai: &'a OpenAI,
    vector_store_id: String,
}

impl<'a> VectorStoreFiles<'a> {
    pub(crate) fn new(openai: &'a OpenAI, vector_store_id: String) -> Self {
        Self {
            openai,
            vector_store_id,
        }
    }
}
```

Create `rs_openai/src/apis/vector_stores/file_batches.rs`:

```rust
//! Batch file-attach operations for a vector store
//! (`/v1/vector_stores/{vector_store_id}/file_batches`). Not yet implemented — see
//! docs/superpowers/research/2026-07-31-openai-api-structure.md.

use crate::client::OpenAI;

#[allow(dead_code)]
pub struct VectorStoreFileBatches<'a> {
    openai: &'a OpenAI,
    vector_store_id: String,
}

impl<'a> VectorStoreFileBatches<'a> {
    pub(crate) fn new(openai: &'a OpenAI, vector_store_id: String) -> Self {
        Self {
            openai,
            vector_store_id,
        }
    }
}
```

- [ ] **Step 2: Create the ChatKit scaffold**

Create `rs_openai/src/apis/chatkit/mod.rs`:

```rust
//! ChatKit (Beta): embeddable chat UI sessions and their threads. Not yet implemented — see
//! docs/superpowers/research/2026-07-31-openai-api-structure.md.

pub mod sessions;
pub mod threads;

use crate::client::OpenAI;

pub struct ChatKit<'a> {
    openai: &'a OpenAI,
}

impl<'a> ChatKit<'a> {
    pub(crate) fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }

    /// Scoped accessor for ChatKit sessions. Not yet implemented.
    pub fn sessions(&self) -> sessions::ChatKitSessions<'a> {
        sessions::ChatKitSessions::new(self.openai)
    }

    /// Scoped accessor for a specific ChatKit session. Not yet implemented.
    pub fn session(&self, id: impl Into<String>) -> sessions::ChatKitSession<'a> {
        sessions::ChatKitSession::new(self.openai, id.into())
    }

    /// Scoped accessor for ChatKit threads. Not yet implemented.
    pub fn threads(&self) -> threads::ChatKitThreads<'a> {
        threads::ChatKitThreads::new(self.openai)
    }

    /// Scoped accessor for a specific ChatKit thread. Not yet implemented.
    pub fn thread(&self, id: impl Into<String>) -> threads::ChatKitThread<'a> {
        threads::ChatKitThread::new(self.openai, id.into())
    }
}
```

Create `rs_openai/src/apis/chatkit/sessions.rs`:

```rust
//! ChatKit sessions: `POST /v1/chatkit/sessions` create,
//! `POST /v1/chatkit/sessions/{session_id}/cancel` cancel. Not yet implemented — see
//! docs/superpowers/research/2026-07-31-openai-api-structure.md.

use crate::client::OpenAI;

#[allow(dead_code)]
pub struct ChatKitSessions<'a> {
    openai: &'a OpenAI,
}

impl<'a> ChatKitSessions<'a> {
    pub(crate) fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }
}

#[allow(dead_code)]
pub struct ChatKitSession<'a> {
    openai: &'a OpenAI,
    id: String,
}

impl<'a> ChatKitSession<'a> {
    pub(crate) fn new(openai: &'a OpenAI, id: String) -> Self {
        Self { openai, id }
    }
}
```

Create `rs_openai/src/apis/chatkit/threads.rs`:

```rust
//! ChatKit threads: `GET /v1/chatkit/threads` list, `GET .../{thread_id}` retrieve,
//! `DELETE .../{thread_id}` delete, `GET .../{thread_id}/items` list items. Not yet
//! implemented — see docs/superpowers/research/2026-07-31-openai-api-structure.md.

use crate::client::OpenAI;

#[allow(dead_code)]
pub struct ChatKitThreads<'a> {
    openai: &'a OpenAI,
}

impl<'a> ChatKitThreads<'a> {
    pub(crate) fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }
}

pub struct ChatKitThread<'a> {
    openai: &'a OpenAI,
    id: String,
}

impl<'a> ChatKitThread<'a> {
    pub(crate) fn new(openai: &'a OpenAI, id: String) -> Self {
        Self { openai, id }
    }

    /// Scoped accessor for this thread's items. Not yet implemented.
    pub fn items(&self) -> ChatKitThreadItems<'a> {
        ChatKitThreadItems::new(self.openai, self.id.clone())
    }
}

#[allow(dead_code)]
pub struct ChatKitThreadItems<'a> {
    openai: &'a OpenAI,
    thread_id: String,
}

impl<'a> ChatKitThreadItems<'a> {
    pub(crate) fn new(openai: &'a OpenAI, thread_id: String) -> Self {
        Self { openai, thread_id }
    }
}
```

- [ ] **Step 3: Create the Containers scaffold**

Create `rs_openai/src/apis/containers/mod.rs`:

```rust
//! Sandboxed containers backing the Code Interpreter tool. Not yet implemented — see
//! docs/superpowers/research/2026-07-31-openai-api-structure.md.

pub mod files;

use crate::client::OpenAI;

#[allow(dead_code)]
pub struct Containers<'a> {
    openai: &'a OpenAI,
}

impl<'a> Containers<'a> {
    pub(crate) fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
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

    /// Scoped accessor for files inside this container. Not yet implemented.
    pub fn files(&self) -> files::ContainerFiles<'a> {
        files::ContainerFiles::new(self.openai, self.id.clone())
    }
}
```

Create `rs_openai/src/apis/containers/files.rs`:

```rust
//! A container's files (`/v1/containers/{container_id}/files`), including content retrieve.
//! Not yet implemented — see docs/superpowers/research/2026-07-31-openai-api-structure.md.

use crate::client::OpenAI;

#[allow(dead_code)]
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
}
```

- [ ] **Step 4: Create the Evals + Graders scaffold**

Create `rs_openai/src/apis/evals/mod.rs`:

```rust
//! Evaluation runs for grading model outputs against a defined criteria set. Not yet
//! implemented — see docs/superpowers/research/2026-07-31-openai-api-structure.md.

pub mod runs;

use crate::client::OpenAI;

#[allow(dead_code)]
pub struct Evals<'a> {
    openai: &'a OpenAI,
}

impl<'a> Evals<'a> {
    pub(crate) fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }
}

pub struct Eval<'a> {
    openai: &'a OpenAI,
    id: String,
}

impl<'a> Eval<'a> {
    pub(crate) fn new(openai: &'a OpenAI, id: String) -> Self {
        Self { openai, id }
    }

    /// Scoped accessor for this eval's runs. Not yet implemented.
    pub fn runs(&self) -> runs::EvalRuns<'a> {
        runs::EvalRuns::new(self.openai, self.id.clone())
    }

    /// Scoped accessor for a specific run of this eval. Not yet implemented.
    pub fn run(&self, run_id: impl Into<String>) -> runs::EvalRun<'a> {
        runs::EvalRun::new(self.openai, self.id.clone(), run_id.into())
    }
}
```

Create `rs_openai/src/apis/evals/runs.rs`:

```rust
//! An eval's runs, including `POST .../runs/{run_id}/cancel`. Not yet implemented — see
//! docs/superpowers/research/2026-07-31-openai-api-structure.md.

use crate::client::OpenAI;

#[allow(dead_code)]
pub struct EvalRuns<'a> {
    openai: &'a OpenAI,
    eval_id: String,
}

impl<'a> EvalRuns<'a> {
    pub(crate) fn new(openai: &'a OpenAI, eval_id: String) -> Self {
        Self { openai, eval_id }
    }
}

#[allow(dead_code)]
pub struct EvalRun<'a> {
    openai: &'a OpenAI,
    eval_id: String,
    run_id: String,
}

impl<'a> EvalRun<'a> {
    pub(crate) fn new(openai: &'a OpenAI, eval_id: String, run_id: String) -> Self {
        Self {
            openai,
            eval_id,
            run_id,
        }
    }
}
```

Create `rs_openai/src/apis/graders.rs`:

```rust
//! Standalone grading-logic definitions used by Evals. Operation set not yet confirmed — see
//! docs/superpowers/research/2026-07-31-openai-api-structure.md. Not yet implemented.

use crate::client::OpenAI;

#[allow(dead_code)]
pub struct Graders<'a> {
    openai: &'a OpenAI,
}

impl<'a> Graders<'a> {
    pub(crate) fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }
}
```

- [ ] **Step 5: Wire the new modules into `apis/mod.rs`**

Replace the full contents of `rs_openai/src/apis/mod.rs` with:

```rust
pub mod assistants;
pub mod audio;
pub mod batch;
pub mod chat;
pub mod chatkit;
pub mod completions;
pub mod containers;
pub mod embeddings;
pub mod evals;
pub mod files;
pub mod fine_tuning;
pub mod graders;
pub mod images;
pub mod messages;
pub mod models;
pub mod moderations;
pub mod runs;
pub mod threads;
pub mod uploads;
pub mod vector_stores;
```

- [ ] **Step 6: Wire the new modules into `client.rs`**

In the `use crate::apis::{...}` block at the top of `rs_openai/src/client.rs`, add `chatkit`, `containers`, `evals`, `graders`, and `vector_stores` to the import list (exact wrapping doesn't matter — `cargo fmt` fixes it in Step 7):

```rust
use crate::apis::{
    assistants, audio, batch, chat, chatkit, completions, containers, embeddings, evals, files,
    fine_tuning, graders, images, messages, models, moderations, runs, threads, uploads,
    vector_stores,
};
```

Then, in `impl OpenAI`, insert these new accessor methods at the alphabetical positions shown (each snippet below shows the existing method it goes next to, for anchoring):

Between `chat()` and `completions()`:

```rust

    pub fn chatkit(&self) -> chatkit::ChatKit<'_> {
        chatkit::ChatKit::new(self)
    }
```

Between `completions()` and `embeddings()`:

```rust

    pub fn containers(&self) -> containers::Containers<'_> {
        containers::Containers::new(self)
    }

    pub fn container(&self, id: impl Into<String>) -> containers::Container<'_> {
        containers::Container::new(self, id.into())
    }
```

Between `embeddings()` and `files()`:

```rust

    pub fn evals(&self) -> evals::Evals<'_> {
        evals::Evals::new(self)
    }

    pub fn eval(&self, id: impl Into<String>) -> evals::Eval<'_> {
        evals::Eval::new(self, id.into())
    }
```

Between `fine_tuning()` and `images()`:

```rust

    pub fn graders(&self) -> graders::Graders<'_> {
        graders::Graders::new(self)
    }
```

After `upload()`, before the closing `}` of `impl OpenAI`:

```rust

    pub fn vector_stores(&self) -> vector_stores::VectorStores<'_> {
        vector_stores::VectorStores::new(self)
    }

    pub fn vector_store(&self, id: impl Into<String>) -> vector_stores::VectorStore<'_> {
        vector_stores::VectorStore::new(self, id.into())
    }
```

- [ ] **Step 7: Verify**

Run: `cargo fmt --all` (fixes the import-list wrapping from Step 6), then `cargo fmt --all -- --check`
Expected: clean.

Run: `cargo build --workspace --all-targets`
Expected: clean build.

Run: `cargo clippy --workspace --all-targets --all-features -- -D warnings`
Expected: exit 0 — confirms every `#[allow(dead_code)]` placement is correct and every non-leaf struct's fields are genuinely read.

Run: `cargo test --workspace`
Expected: all existing tests still pass unchanged (this task adds no tests — scaffolds have no behavior to test yet).

- [ ] **Step 8: Commit**

```bash
git add -A
git commit -m "$(cat <<'EOF'
feat: scaffold Vector Stores, ChatKit, Containers, Evals, Graders

Adds module directories and accessor-struct scaffolding for four
categories rs_openai has zero coverage of today, per the OpenAI API
structure research. ChatKit is the exact 3-tier category → resource
→ operation example that motivated this restructuring
(chatkit -> sessions/threads -> individual operations). No
request/response types or endpoint logic yet — that's later,
separately-scoped work.
EOF
)"
```

---

### Task 5: Scaffold Responses, Conversations, Videos, Webhooks

**Files:**
- Create: `rs_openai/src/apis/responses.rs`, `rs_openai/src/apis/conversations.rs`, `rs_openai/src/apis/videos.rs`, `rs_openai/src/apis/webhooks.rs`
- Modify: `rs_openai/src/apis/mod.rs`, `rs_openai/src/client.rs`

**Interfaces:**
- Consumes: nothing from earlier tasks.
- Produces: `client.responses()`/`client.response(id)`; `client.conversations()`/`client.conversation(id)`; `client.videos()`/`client.video(id)`; `client.webhooks()`. All scaffolds — no endpoint methods.

None of these four categories have confirmed sub-resources worth their own directory (Responses' `input_items` and Conversations' `items` are single list endpoints, not multi-operation sub-resources; Videos is a plain collection/singular pair; Webhooks isn't a CRUD resource at all — no ID, no collection, just a future signature-verification helper). Per the design's "flat categories stay single files" rule, all four are single files. Every struct here is a leaf scaffold (no accessor methods anywhere) so every one gets `#[allow(dead_code)]`.

Note: Responses and Conversations have no plain "list all" endpoint in OpenAI's API (per the structure research, only create/retrieve/delete/etc.) — their plural accessor exists for symmetry with every other category and to host `create` once implemented, not because a `list` operation exists.

- [ ] **Step 1: Create the Responses scaffold**

Create `rs_openai/src/apis/responses.rs`:

```rust
//! Model responses — OpenAI's now-recommended primary API, superseding Chat Completions and
//! (eventually) Assistants. Not yet implemented — see
//! docs/superpowers/research/2026-07-31-openai-api-structure.md.

use crate::client::OpenAI;

#[allow(dead_code)]
pub struct Responses<'a> {
    openai: &'a OpenAI,
}

impl<'a> Responses<'a> {
    pub(crate) fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }
}

#[allow(dead_code)]
pub struct Response<'a> {
    openai: &'a OpenAI,
    id: String,
}

impl<'a> Response<'a> {
    pub(crate) fn new(openai: &'a OpenAI, id: String) -> Self {
        Self { openai, id }
    }
}
```

- [ ] **Step 2: Create the Conversations scaffold**

Create `rs_openai/src/apis/conversations.rs`:

```rust
//! Stateful conversation context for the Responses API — replaces the Assistants API's
//! "thread" concept. Not yet implemented — see
//! docs/superpowers/research/2026-07-31-openai-api-structure.md.

use crate::client::OpenAI;

#[allow(dead_code)]
pub struct Conversations<'a> {
    openai: &'a OpenAI,
}

impl<'a> Conversations<'a> {
    pub(crate) fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }
}

#[allow(dead_code)]
pub struct Conversation<'a> {
    openai: &'a OpenAI,
    id: String,
}

impl<'a> Conversation<'a> {
    pub(crate) fn new(openai: &'a OpenAI, id: String) -> Self {
        Self { openai, id }
    }
}
```

- [ ] **Step 3: Create the Videos scaffold**

Create `rs_openai/src/apis/videos.rs`:

```rust
//! Sora video generation. Not yet implemented — see
//! docs/superpowers/research/2026-07-31-openai-api-structure.md.

use crate::client::OpenAI;

#[allow(dead_code)]
pub struct Videos<'a> {
    openai: &'a OpenAI,
}

impl<'a> Videos<'a> {
    pub(crate) fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }
}

#[allow(dead_code)]
pub struct Video<'a> {
    openai: &'a OpenAI,
    id: String,
}

impl<'a> Video<'a> {
    pub(crate) fn new(openai: &'a OpenAI, id: String) -> Self {
        Self { openai, id }
    }
}
```

- [ ] **Step 4: Create the Webhooks scaffold**

Create `rs_openai/src/apis/webhooks.rs`:

```rust
//! Signature verification and typed event payloads for OpenAI's outbound webhooks (e.g.
//! batch completion, eval run completion). Not a CRUD resource — needs an "unwrap"-style
//! helper (verify signature + deserialize payload) rather than HTTP-calling methods. Not yet
//! implemented — see docs/superpowers/research/2026-07-31-openai-api-structure.md.

use crate::client::OpenAI;

#[allow(dead_code)]
pub struct Webhooks<'a> {
    openai: &'a OpenAI,
}

impl<'a> Webhooks<'a> {
    pub(crate) fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }
}
```

- [ ] **Step 5: Wire the new modules into `apis/mod.rs`**

Replace the full contents of `rs_openai/src/apis/mod.rs` with:

```rust
pub mod assistants;
pub mod audio;
pub mod batch;
pub mod chat;
pub mod chatkit;
pub mod completions;
pub mod containers;
pub mod conversations;
pub mod embeddings;
pub mod evals;
pub mod files;
pub mod fine_tuning;
pub mod graders;
pub mod images;
pub mod messages;
pub mod models;
pub mod moderations;
pub mod responses;
pub mod runs;
pub mod threads;
pub mod uploads;
pub mod vector_stores;
pub mod videos;
pub mod webhooks;
```

- [ ] **Step 6: Wire the new modules into `client.rs`**

In the `use crate::apis::{...}` block, add `conversations`, `responses`, `videos`, and `webhooks` to the import list (exact wrapping doesn't matter — `cargo fmt` fixes it in Step 7).

In `impl OpenAI`, insert these new accessor methods at the alphabetical positions shown:

Between `container(...)` (end of the Task 4 Containers pair) and `embeddings()`:

```rust

    pub fn conversations(&self) -> conversations::Conversations<'_> {
        conversations::Conversations::new(self)
    }

    pub fn conversation(&self, id: impl Into<String>) -> conversations::Conversation<'_> {
        conversations::Conversation::new(self, id.into())
    }
```

Between `moderations()` and `runs()`:

```rust

    pub fn responses(&self) -> responses::Responses<'_> {
        responses::Responses::new(self)
    }

    pub fn response(&self, id: impl Into<String>) -> responses::Response<'_> {
        responses::Response::new(self, id.into())
    }
```

After `vector_store(...)` (end of the Task 4 Vector Stores pair), before the closing `}` of `impl OpenAI`:

```rust

    pub fn videos(&self) -> videos::Videos<'_> {
        videos::Videos::new(self)
    }

    pub fn video(&self, id: impl Into<String>) -> videos::Video<'_> {
        videos::Video::new(self, id.into())
    }

    pub fn webhooks(&self) -> webhooks::Webhooks<'_> {
        webhooks::Webhooks::new(self)
    }
```

- [ ] **Step 7: Verify**

Run: `cargo fmt --all`, then `cargo fmt --all -- --check`
Expected: clean.

Run: `cargo build --workspace --all-targets`
Expected: clean build.

Run: `cargo clippy --workspace --all-targets --all-features -- -D warnings`
Expected: exit 0.

Run: `cargo test --workspace`
Expected: all existing tests still pass unchanged.

- [ ] **Step 8: Commit**

```bash
git add -A
git commit -m "$(cat <<'EOF'
feat: scaffold Responses, Conversations, Videos, Webhooks

Adds flat-file accessor-struct scaffolding for four categories
rs_openai has zero coverage of today: Responses (OpenAI's now-
recommended primary API, superseding Chat Completions) and
Conversations (its stateful-context companion, replacing Assistants'
thread concept), plus Videos and Webhooks. No request/response types
or endpoint logic yet — separately-scoped follow-on work.
EOF
)"
```

---

### Task 6: Scaffold the Organization/admin tier (core resources)

**Files:**
- Create: `rs_openai/src/apis/organization/mod.rs`, `rs_openai/src/apis/organization/audit_logs.rs`, `rs_openai/src/apis/organization/admin_api_keys.rs`, `rs_openai/src/apis/organization/usage.rs`, `rs_openai/src/apis/organization/groups.rs`, `rs_openai/src/apis/organization/invites.rs`, `rs_openai/src/apis/organization/roles.rs`, `rs_openai/src/apis/organization/users.rs`
- Modify: `rs_openai/src/apis/mod.rs`, `rs_openai/src/client.rs`

**Interfaces:**
- Consumes: nothing from earlier tasks.
- Produces: `client.organization()` → `organization::Organization<'_>` with `.audit_logs()`, `.admin_api_keys()`/`.admin_api_key(id)`, `.usage()`, `.groups()`/`.group(id)` (+ `.group(id).users()`), `.invites()`/`.invite(id)`, `.roles()`/`.role(id)`, `.users()`/`.user(id)` (+ `.user(id).roles()`). **Task 7 depends on this task's `Organization` struct and `pub(crate) fn new` existing in `rs_openai/src/apis/organization/mod.rs`** — it appends a `pub mod projects;` declaration and a `.projects()`/`.project(id)` method pair to the same `impl<'a> Organization<'a>` block, so this task's `mod.rs` must compile and be committed before Task 7 starts.

This is the large admin surface the design spec calls out as needing "its own top-level namespace, separate from the 'call the model' surface" (`client.organization()`, not spread across top-level `client.*()` accessors). Per the structure research, exact path prefixes are marked **[verify exact prefixes]** — not yet confirmed, which is fine for a scaffold (no HTTP calls exist yet to get the path wrong). Projects is deliberately excluded from this task — it's large enough and independent enough to be its own task (Task 7), and this task's `Organization` struct is designed to receive it as a pure addition afterward, with no reshaping of what this task builds.

Groups and Users each have one real sub-resource (a group's member users; a user's assigned roles) so they use the singular-struct-with-a-method pattern from earlier tasks; Audit Logs, Admin API Keys, Usage, Invites, and Roles have no further sub-resources and stay flat leaf structs.

- [ ] **Step 1: Create the `Organization` top-level struct**

Create `rs_openai/src/apis/organization/mod.rs`:

```rust
//! Organization-level administration: audit logs, admin API keys, usage, groups, invites,
//! projects, roles, and users. A distinct "admin tier", separate from the "call the model"
//! surface the rest of this crate exposes. Not yet implemented — see
//! docs/superpowers/research/2026-07-31-openai-api-structure.md.

pub mod admin_api_keys;
pub mod audit_logs;
pub mod groups;
pub mod invites;
pub mod roles;
pub mod usage;
pub mod users;

use crate::client::OpenAI;

pub struct Organization<'a> {
    openai: &'a OpenAI,
}

impl<'a> Organization<'a> {
    pub(crate) fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }

    /// Scoped accessor for organization audit logs. Not yet implemented.
    pub fn audit_logs(&self) -> audit_logs::OrganizationAuditLogs<'a> {
        audit_logs::OrganizationAuditLogs::new(self.openai)
    }

    /// Scoped accessor for organization admin API keys. Not yet implemented.
    pub fn admin_api_keys(&self) -> admin_api_keys::OrganizationAdminApiKeys<'a> {
        admin_api_keys::OrganizationAdminApiKeys::new(self.openai)
    }

    /// Scoped accessor for a specific admin API key. Not yet implemented.
    pub fn admin_api_key(
        &self,
        id: impl Into<String>,
    ) -> admin_api_keys::OrganizationAdminApiKey<'a> {
        admin_api_keys::OrganizationAdminApiKey::new(self.openai, id.into())
    }

    /// Scoped accessor for organization usage reporting. Not yet implemented.
    pub fn usage(&self) -> usage::OrganizationUsage<'a> {
        usage::OrganizationUsage::new(self.openai)
    }

    /// Scoped accessor for organization groups. Not yet implemented.
    pub fn groups(&self) -> groups::OrganizationGroups<'a> {
        groups::OrganizationGroups::new(self.openai)
    }

    /// Scoped accessor for a specific organization group. Not yet implemented.
    pub fn group(&self, id: impl Into<String>) -> groups::OrganizationGroup<'a> {
        groups::OrganizationGroup::new(self.openai, id.into())
    }

    /// Scoped accessor for organization invites. Not yet implemented.
    pub fn invites(&self) -> invites::OrganizationInvites<'a> {
        invites::OrganizationInvites::new(self.openai)
    }

    /// Scoped accessor for a specific organization invite. Not yet implemented.
    pub fn invite(&self, id: impl Into<String>) -> invites::OrganizationInvite<'a> {
        invites::OrganizationInvite::new(self.openai, id.into())
    }

    /// Scoped accessor for organization roles. Not yet implemented.
    pub fn roles(&self) -> roles::OrganizationRoles<'a> {
        roles::OrganizationRoles::new(self.openai)
    }

    /// Scoped accessor for a specific organization role. Not yet implemented.
    pub fn role(&self, id: impl Into<String>) -> roles::OrganizationRole<'a> {
        roles::OrganizationRole::new(self.openai, id.into())
    }

    /// Scoped accessor for organization users. Not yet implemented.
    pub fn users(&self) -> users::OrganizationUsers<'a> {
        users::OrganizationUsers::new(self.openai)
    }

    /// Scoped accessor for a specific organization user. Not yet implemented.
    pub fn user(&self, id: impl Into<String>) -> users::OrganizationUser<'a> {
        users::OrganizationUser::new(self.openai, id.into())
    }
}
```

- [ ] **Step 2: Create the flat leaf resources — Audit Logs, Admin API Keys, Usage**

Create `rs_openai/src/apis/organization/audit_logs.rs`:

```rust
//! Organization audit logs (`/v1/organization/audit_logs`, list-only). Not yet implemented —
//! see docs/superpowers/research/2026-07-31-openai-api-structure.md.

use crate::client::OpenAI;

#[allow(dead_code)]
pub struct OrganizationAuditLogs<'a> {
    openai: &'a OpenAI,
}

impl<'a> OrganizationAuditLogs<'a> {
    pub(crate) fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }
}
```

Create `rs_openai/src/apis/organization/admin_api_keys.rs`:

```rust
//! Organization admin API keys (`/v1/organization/admin_api_keys`). Not yet implemented —
//! see docs/superpowers/research/2026-07-31-openai-api-structure.md.

use crate::client::OpenAI;

#[allow(dead_code)]
pub struct OrganizationAdminApiKeys<'a> {
    openai: &'a OpenAI,
}

impl<'a> OrganizationAdminApiKeys<'a> {
    pub(crate) fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }
}

#[allow(dead_code)]
pub struct OrganizationAdminApiKey<'a> {
    openai: &'a OpenAI,
    id: String,
}

impl<'a> OrganizationAdminApiKey<'a> {
    pub(crate) fn new(openai: &'a OpenAI, id: String) -> Self {
        Self { openai, id }
    }
}
```

Create `rs_openai/src/apis/organization/usage.rs`:

```rust
//! Organization usage reporting (`/v1/organization/usage`). Not yet implemented — see
//! docs/superpowers/research/2026-07-31-openai-api-structure.md.

use crate::client::OpenAI;

#[allow(dead_code)]
pub struct OrganizationUsage<'a> {
    openai: &'a OpenAI,
}

impl<'a> OrganizationUsage<'a> {
    pub(crate) fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }
}
```

- [ ] **Step 3: Create Groups (with member Users sub-resource)**

Create `rs_openai/src/apis/organization/groups.rs`:

```rust
//! Organization groups (`/v1/organization/groups`) and their member users. Not yet
//! implemented — see docs/superpowers/research/2026-07-31-openai-api-structure.md.

use crate::client::OpenAI;

#[allow(dead_code)]
pub struct OrganizationGroups<'a> {
    openai: &'a OpenAI,
}

impl<'a> OrganizationGroups<'a> {
    pub(crate) fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }
}

pub struct OrganizationGroup<'a> {
    openai: &'a OpenAI,
    id: String,
}

impl<'a> OrganizationGroup<'a> {
    pub(crate) fn new(openai: &'a OpenAI, id: String) -> Self {
        Self { openai, id }
    }

    /// Scoped accessor for this group's member users. Not yet implemented.
    pub fn users(&self) -> OrganizationGroupUsers<'a> {
        OrganizationGroupUsers::new(self.openai, self.id.clone())
    }
}

#[allow(dead_code)]
pub struct OrganizationGroupUsers<'a> {
    openai: &'a OpenAI,
    group_id: String,
}

impl<'a> OrganizationGroupUsers<'a> {
    pub(crate) fn new(openai: &'a OpenAI, group_id: String) -> Self {
        Self { openai, group_id }
    }
}
```

- [ ] **Step 4: Create Invites and Roles**

Create `rs_openai/src/apis/organization/invites.rs`:

```rust
//! Organization invites (`/v1/organization/invites`). Not yet implemented — see
//! docs/superpowers/research/2026-07-31-openai-api-structure.md.

use crate::client::OpenAI;

#[allow(dead_code)]
pub struct OrganizationInvites<'a> {
    openai: &'a OpenAI,
}

impl<'a> OrganizationInvites<'a> {
    pub(crate) fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }
}

#[allow(dead_code)]
pub struct OrganizationInvite<'a> {
    openai: &'a OpenAI,
    id: String,
}

impl<'a> OrganizationInvite<'a> {
    pub(crate) fn new(openai: &'a OpenAI, id: String) -> Self {
        Self { openai, id }
    }
}
```

Create `rs_openai/src/apis/organization/roles.rs`:

```rust
//! Organization roles (`/v1/organization/roles`). Not yet implemented — see
//! docs/superpowers/research/2026-07-31-openai-api-structure.md.

use crate::client::OpenAI;

#[allow(dead_code)]
pub struct OrganizationRoles<'a> {
    openai: &'a OpenAI,
}

impl<'a> OrganizationRoles<'a> {
    pub(crate) fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }
}

#[allow(dead_code)]
pub struct OrganizationRole<'a> {
    openai: &'a OpenAI,
    id: String,
}

impl<'a> OrganizationRole<'a> {
    pub(crate) fn new(openai: &'a OpenAI, id: String) -> Self {
        Self { openai, id }
    }
}
```

- [ ] **Step 5: Create Users (with assigned Roles sub-resource)**

Create `rs_openai/src/apis/organization/users.rs`:

```rust
//! Organization users (`/v1/organization/users`) and their assigned roles. Not yet
//! implemented — see docs/superpowers/research/2026-07-31-openai-api-structure.md.

use crate::client::OpenAI;

#[allow(dead_code)]
pub struct OrganizationUsers<'a> {
    openai: &'a OpenAI,
}

impl<'a> OrganizationUsers<'a> {
    pub(crate) fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }
}

pub struct OrganizationUser<'a> {
    openai: &'a OpenAI,
    id: String,
}

impl<'a> OrganizationUser<'a> {
    pub(crate) fn new(openai: &'a OpenAI, id: String) -> Self {
        Self { openai, id }
    }

    /// Scoped accessor for this user's assigned roles. Not yet implemented.
    pub fn roles(&self) -> OrganizationUserRoles<'a> {
        OrganizationUserRoles::new(self.openai, self.id.clone())
    }
}

#[allow(dead_code)]
pub struct OrganizationUserRoles<'a> {
    openai: &'a OpenAI,
    user_id: String,
}

impl<'a> OrganizationUserRoles<'a> {
    pub(crate) fn new(openai: &'a OpenAI, user_id: String) -> Self {
        Self { openai, user_id }
    }
}
```

- [ ] **Step 6: Wire `organization` into `apis/mod.rs`**

Replace the full contents of `rs_openai/src/apis/mod.rs` with:

```rust
pub mod assistants;
pub mod audio;
pub mod batch;
pub mod chat;
pub mod chatkit;
pub mod completions;
pub mod containers;
pub mod conversations;
pub mod embeddings;
pub mod evals;
pub mod files;
pub mod fine_tuning;
pub mod graders;
pub mod images;
pub mod messages;
pub mod models;
pub mod moderations;
pub mod organization;
pub mod responses;
pub mod runs;
pub mod threads;
pub mod uploads;
pub mod vector_stores;
pub mod videos;
pub mod webhooks;
```

- [ ] **Step 7: Wire `organization` into `client.rs`**

Add `organization` to the `use crate::apis::{...}` import list.

Between `moderations()` and `responses()`, insert:

```rust

    pub fn organization(&self) -> organization::Organization<'_> {
        organization::Organization::new(self)
    }
```

- [ ] **Step 8: Verify**

Run: `cargo fmt --all`, then `cargo fmt --all -- --check`
Expected: clean.

Run: `cargo build --workspace --all-targets`
Expected: clean build.

Run: `cargo clippy --workspace --all-targets --all-features -- -D warnings`
Expected: exit 0.

Run: `cargo test --workspace`
Expected: all existing tests still pass unchanged.

- [ ] **Step 9: Commit**

```bash
git add -A
git commit -m "$(cat <<'EOF'
feat: scaffold the Organization/admin tier (core resources)

Adds client.organization() as its own top-level namespace, separate
from the "call the model" surface, per the design spec. Covers audit
logs, admin API keys, usage, groups (+ member users), invites, roles,
and users (+ assigned roles). Projects is deliberately excluded here
— it's large and independent enough to be its own follow-up task.
Scaffolds only — no request/response types or endpoint logic yet.
EOF
)"
```

---

### Task 7: Scaffold the Organization/admin tier (Projects sub-tree)

**Files:**
- Create: `rs_openai/src/apis/organization/projects/mod.rs`, `rs_openai/src/apis/organization/projects/api_keys.rs`, `rs_openai/src/apis/organization/projects/groups.rs`, `rs_openai/src/apis/organization/projects/rate_limits.rs`, `rs_openai/src/apis/organization/projects/service_accounts.rs`, `rs_openai/src/apis/organization/projects/users.rs`
- Modify: `rs_openai/src/apis/organization/mod.rs`

**Interfaces:**
- Consumes: Task 6's `rs_openai/src/apis/organization/mod.rs` — specifically its `Organization<'a>` struct and `impl<'a> Organization<'a>` block, which this task appends to (do not restructure what Task 6 built, only add to it).
- Produces: `client.organization().projects()` → `projects::OrganizationProjects<'_>`; `client.organization().project(id)` → `projects::OrganizationProject<'_>` with `.api_keys()`/`.api_key(id)`, `.groups()`, `.rate_limits()`, `.service_accounts()`/`.service_account(id)`, `.users()`/`.user(id)`. No later task depends on these — Task 8's final verification checks the whole crate builds together.

Per the structure research, Projects sub-resources have varying confirmed operation sets: API Keys, Service Accounts, and Users all have a confirmed `retrieve` operation (so they get both a plural collection struct and a singular ID-scoped struct); Groups only has `list`/`delete` (no retrieve — plural only); Rate Limits is `read`/`update` on the project itself, more a settings object than a collection (single struct, no plural/singular split).

The structure research also flagged a **parallel, slightly differently-scoped "Projects" family** in OpenAI's reference index (Groups Roles, Roles, Users Roles) that looks like it might be a project-scoped-API-key view of the same underlying resources — not scaffolded here since it's genuinely ambiguous; a doc comment flags it for resolution against live docs before implementation.

- [ ] **Step 1: Create the `OrganizationProjects`/`OrganizationProject` structs**

Create `rs_openai/src/apis/organization/projects/mod.rs`:

```rust
//! A project-scoped view of organization resources: API keys, groups, rate limits, service
//! accounts, and users, all scoped to one project. Not yet implemented — see
//! docs/superpowers/research/2026-07-31-openai-api-structure.md.
//!
//! Note: the structure research also found a parallel, slightly differently-scoped
//! "Projects" family (Groups Roles, Roles, Users Roles) in OpenAI's reference index — likely
//! a project-scoped-API-key view of the same underlying resources. Not scaffolded here;
//! resolve the ambiguity against live docs before implementing.

pub mod api_keys;
pub mod groups;
pub mod rate_limits;
pub mod service_accounts;
pub mod users;

use crate::client::OpenAI;

#[allow(dead_code)]
pub struct OrganizationProjects<'a> {
    openai: &'a OpenAI,
}

impl<'a> OrganizationProjects<'a> {
    pub(crate) fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }
}

pub struct OrganizationProject<'a> {
    openai: &'a OpenAI,
    id: String,
}

impl<'a> OrganizationProject<'a> {
    pub(crate) fn new(openai: &'a OpenAI, id: String) -> Self {
        Self { openai, id }
    }

    /// Scoped accessor for this project's API keys. Not yet implemented.
    pub fn api_keys(&self) -> api_keys::OrganizationProjectApiKeys<'a> {
        api_keys::OrganizationProjectApiKeys::new(self.openai, self.id.clone())
    }

    /// Scoped accessor for a specific project API key. Not yet implemented.
    pub fn api_key(&self, id: impl Into<String>) -> api_keys::OrganizationProjectApiKey<'a> {
        api_keys::OrganizationProjectApiKey::new(self.openai, self.id.clone(), id.into())
    }

    /// Scoped accessor for this project's groups. Not yet implemented.
    pub fn groups(&self) -> groups::OrganizationProjectGroups<'a> {
        groups::OrganizationProjectGroups::new(self.openai, self.id.clone())
    }

    /// Scoped accessor for this project's rate limits. Not yet implemented.
    pub fn rate_limits(&self) -> rate_limits::OrganizationProjectRateLimits<'a> {
        rate_limits::OrganizationProjectRateLimits::new(self.openai, self.id.clone())
    }

    /// Scoped accessor for this project's service accounts. Not yet implemented.
    pub fn service_accounts(&self) -> service_accounts::OrganizationProjectServiceAccounts<'a> {
        service_accounts::OrganizationProjectServiceAccounts::new(self.openai, self.id.clone())
    }

    /// Scoped accessor for a specific project service account. Not yet implemented.
    pub fn service_account(
        &self,
        id: impl Into<String>,
    ) -> service_accounts::OrganizationProjectServiceAccount<'a> {
        service_accounts::OrganizationProjectServiceAccount::new(
            self.openai,
            self.id.clone(),
            id.into(),
        )
    }

    /// Scoped accessor for this project's users. Not yet implemented.
    pub fn users(&self) -> users::OrganizationProjectUsers<'a> {
        users::OrganizationProjectUsers::new(self.openai, self.id.clone())
    }

    /// Scoped accessor for a specific project user. Not yet implemented.
    pub fn user(&self, id: impl Into<String>) -> users::OrganizationProjectUser<'a> {
        users::OrganizationProjectUser::new(self.openai, self.id.clone(), id.into())
    }
}
```

- [ ] **Step 2: Create the Project API Keys and Service Accounts sub-resources**

Create `rs_openai/src/apis/organization/projects/api_keys.rs`:

```rust
//! A project's API keys (`/v1/organization/projects/{project_id}/api_keys`): list, retrieve,
//! delete. Not yet implemented — see
//! docs/superpowers/research/2026-07-31-openai-api-structure.md.

use crate::client::OpenAI;

#[allow(dead_code)]
pub struct OrganizationProjectApiKeys<'a> {
    openai: &'a OpenAI,
    project_id: String,
}

impl<'a> OrganizationProjectApiKeys<'a> {
    pub(crate) fn new(openai: &'a OpenAI, project_id: String) -> Self {
        Self { openai, project_id }
    }
}

#[allow(dead_code)]
pub struct OrganizationProjectApiKey<'a> {
    openai: &'a OpenAI,
    project_id: String,
    id: String,
}

impl<'a> OrganizationProjectApiKey<'a> {
    pub(crate) fn new(openai: &'a OpenAI, project_id: String, id: String) -> Self {
        Self {
            openai,
            project_id,
            id,
        }
    }
}
```

Create `rs_openai/src/apis/organization/projects/service_accounts.rs`:

```rust
//! A project's service accounts
//! (`/v1/organization/projects/{project_id}/service_accounts`): create, list, retrieve,
//! delete. Not yet implemented — see
//! docs/superpowers/research/2026-07-31-openai-api-structure.md.

use crate::client::OpenAI;

#[allow(dead_code)]
pub struct OrganizationProjectServiceAccounts<'a> {
    openai: &'a OpenAI,
    project_id: String,
}

impl<'a> OrganizationProjectServiceAccounts<'a> {
    pub(crate) fn new(openai: &'a OpenAI, project_id: String) -> Self {
        Self { openai, project_id }
    }
}

#[allow(dead_code)]
pub struct OrganizationProjectServiceAccount<'a> {
    openai: &'a OpenAI,
    project_id: String,
    id: String,
}

impl<'a> OrganizationProjectServiceAccount<'a> {
    pub(crate) fn new(openai: &'a OpenAI, project_id: String, id: String) -> Self {
        Self {
            openai,
            project_id,
            id,
        }
    }
}
```

- [ ] **Step 3: Create the Project Groups, Rate Limits, and Users sub-resources**

Create `rs_openai/src/apis/organization/projects/groups.rs`:

```rust
//! A project's groups (`/v1/organization/projects/{project_id}/groups`): list, delete. Not
//! yet implemented — see docs/superpowers/research/2026-07-31-openai-api-structure.md.

use crate::client::OpenAI;

#[allow(dead_code)]
pub struct OrganizationProjectGroups<'a> {
    openai: &'a OpenAI,
    project_id: String,
}

impl<'a> OrganizationProjectGroups<'a> {
    pub(crate) fn new(openai: &'a OpenAI, project_id: String) -> Self {
        Self { openai, project_id }
    }
}
```

Create `rs_openai/src/apis/organization/projects/rate_limits.rs`:

```rust
//! A project's rate limits (`/v1/organization/projects/{project_id}/rate_limits`): read,
//! update. Not yet implemented — see
//! docs/superpowers/research/2026-07-31-openai-api-structure.md.

use crate::client::OpenAI;

#[allow(dead_code)]
pub struct OrganizationProjectRateLimits<'a> {
    openai: &'a OpenAI,
    project_id: String,
}

impl<'a> OrganizationProjectRateLimits<'a> {
    pub(crate) fn new(openai: &'a OpenAI, project_id: String) -> Self {
        Self { openai, project_id }
    }
}
```

Create `rs_openai/src/apis/organization/projects/users.rs`:

```rust
//! A project's users (`/v1/organization/projects/{project_id}/users`): create, list,
//! retrieve, update, delete. Not yet implemented — see
//! docs/superpowers/research/2026-07-31-openai-api-structure.md.

use crate::client::OpenAI;

#[allow(dead_code)]
pub struct OrganizationProjectUsers<'a> {
    openai: &'a OpenAI,
    project_id: String,
}

impl<'a> OrganizationProjectUsers<'a> {
    pub(crate) fn new(openai: &'a OpenAI, project_id: String) -> Self {
        Self { openai, project_id }
    }
}

#[allow(dead_code)]
pub struct OrganizationProjectUser<'a> {
    openai: &'a OpenAI,
    project_id: String,
    id: String,
}

impl<'a> OrganizationProjectUser<'a> {
    pub(crate) fn new(openai: &'a OpenAI, project_id: String, id: String) -> Self {
        Self {
            openai,
            project_id,
            id,
        }
    }
}
```

- [ ] **Step 4: Wire `projects` into `organization/mod.rs`**

In `rs_openai/src/apis/organization/mod.rs`, add `pub mod projects;` to the module declarations, between `pub mod invites;` and `pub mod roles;`:

```rust
pub mod admin_api_keys;
pub mod audit_logs;
pub mod groups;
pub mod invites;
pub mod projects;
pub mod roles;
pub mod usage;
pub mod users;
```

Then, in `impl<'a> Organization<'a>`, insert the following between the `invite(...)` method and the `roles()` method:

```rust

    /// Scoped accessor for organization projects. Not yet implemented.
    pub fn projects(&self) -> projects::OrganizationProjects<'a> {
        projects::OrganizationProjects::new(self.openai)
    }

    /// Scoped accessor for a specific organization project. Not yet implemented.
    pub fn project(&self, id: impl Into<String>) -> projects::OrganizationProject<'a> {
        projects::OrganizationProject::new(self.openai, id.into())
    }
```

No `client.rs` changes are needed for this task — `projects()`/`project(id)` are reached through `client.organization()`, not as new top-level accessors.

- [ ] **Step 5: Verify**

Run: `cargo fmt --all`, then `cargo fmt --all -- --check`
Expected: clean.

Run: `cargo build --workspace --all-targets`
Expected: clean build.

Run: `cargo clippy --workspace --all-targets --all-features -- -D warnings`
Expected: exit 0.

Run: `cargo test --workspace`
Expected: all existing tests still pass unchanged.

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "$(cat <<'EOF'
feat: scaffold the Organization/admin tier (Projects sub-tree)

Adds client.organization().projects()/.project(id) with API keys,
groups, rate limits, service accounts, and users scoped to a single
project, completing the Organization/admin tier's scaffolding.
Flags a genuinely ambiguous parallel "Projects" resource family found
during research (Groups Roles, Roles, Users Roles) for resolution
against live docs before implementation, rather than guessing its
shape now. Scaffolds only — no request/response types or endpoint
logic yet.
EOF
)"
```

---

### Task 8: Final structure cross-check, whole-workspace verification, version bump

**Files:**
- Modify: `rs_openai/Cargo.toml`

**Interfaces:**
- Consumes: everything from Tasks 1–7 — this task makes no structural changes, it verifies the cumulative result of every prior task composes correctly and closes out the plan.
- Produces: nothing further depends on this task; it's the last one.

Each earlier task verified itself in isolation. This task verifies the *whole* branch together — in particular, Tasks 4, 5, and 6 all inserted new `pub fn` accessor methods and `use crate::apis::{...}` import entries into the same `client.rs` file at different anchor points; a clean build here is the real proof those insertions composed without a duplicate name or a missed import. It also closes the loop against the research that started Phase 2a: every category the structure doc found should now have *some* footprint in the crate (implemented, scaffolded, or explicitly deprecated-in-place) — the only deliberate exception is Realtime.

- [ ] **Step 1: Cross-check every category against the structure research**

Run: `cat rs_openai/src/apis/mod.rs`
Expected output — exactly these 25 lines, confirming every category from `docs/superpowers/research/2026-07-31-openai-api-structure.md`'s coverage table has landed except Realtime (deliberately deferred) and Edits/Engines (deliberately deleted in Task 1):

```
pub mod assistants;
pub mod audio;
pub mod batch;
pub mod chat;
pub mod chatkit;
pub mod completions;
pub mod containers;
pub mod conversations;
pub mod embeddings;
pub mod evals;
pub mod files;
pub mod fine_tuning;
pub mod graders;
pub mod images;
pub mod messages;
pub mod models;
pub mod moderations;
pub mod organization;
pub mod responses;
pub mod runs;
pub mod threads;
pub mod uploads;
pub mod vector_stores;
pub mod videos;
pub mod webhooks;
```

If any line is missing, the corresponding task above didn't get its `apis/mod.rs` edit applied — go back and re-check that task before continuing.

Run: `grep -rn "TODO\|FIXME\|not yet implemented" rs_openai/src/apis/ --include=*.rs -l`
Expected: every file in this list is a scaffold file created in Tasks 4–7 (its "not yet implemented" is the intentional scaffold doc comment) — none are in `assistants.rs`, `threads.rs`, `messages.rs`, `runs.rs` (Task 3 used "no further feature work" phrasing, not "not yet implemented", so they won't match), and none are in the pre-existing implemented modules (`audio`, `batch`, `chat`, `completions`, `embeddings`, `files`, `fine_tuning`, `images`, `models`, `moderations`, `uploads`) outside of their new Task 2 scaffold sub-files (`fine_tuning/checkpoints/`, `audio/voice_consents.rs`, `audio/voices.rs`).

- [ ] **Step 2: Run the full verification suite on the assembled branch**

Run: `cargo fmt --all -- --check`
Expected: clean.

Run: `cargo build --workspace --all-targets`
Expected: clean build — this compiles every example crate too (`examples/audio`, `examples/fine_tuning`, etc.), confirming Task 2's directory moves didn't change `fine_tuning`'s or `audio`'s public API and existing examples still compile untouched.

Run: `cargo clippy --workspace --all-targets --all-features -- -D warnings`
Expected: exit 0 — confirms every `#[allow(dead_code)]` placement across every task's scaffolds is both necessary and sufficient, with no unused-allow warnings either (clippy's `unused_attributes`/`dead_code` lints would flag an `#[allow(dead_code)]` on a struct whose fields actually are read).

Run: `cargo test --workspace`
Expected: same pass count as before Task 1 started (this whole plan adds no new tests — it's pure restructuring/scaffolding — so the number should match Task 1's baseline exactly, just organized differently).

- [ ] **Step 3: Bump the crate version**

In `rs_openai/Cargo.toml`, change:

```toml
version = "0.6.0"
```

to:

```toml
version = "0.7.0"
```

Per the design spec's Versioning section: this is a breaking-change round (module paths change, `edits`/`engines` disappear, several new top-level accessors appear) shipped as a pre-1.0 minor bump.

- [ ] **Step 4: Final verification and commit**

Run: `cargo build --workspace --all-targets` once more (confirms the version bump alone didn't break anything — it shouldn't, but this is the last step before committing).
Expected: clean build.

```bash
git add -A
git commit -m "$(cat <<'EOF'
chore: Phase 2a wrap-up — structure cross-check and version bump to 0.7.0

Confirms every category from the OpenAI API structure research
(docs/superpowers/research/2026-07-31-openai-api-structure.md) now
has a footprint in the crate — implemented, scaffolded, or
deprecated-in-place — except Realtime, which remains explicitly
deferred pending its own connection-oriented design. Bumps the crate
to 0.7.0 (pre-1.0 minor) for this round's breaking changes: module
restructuring, edits/engines removal, and the new plural/singular
accessor pattern.

This closes out Phase 2a (module restructuring + scaffolding).
Filling in the newly-scaffolded categories' request/response types
and applying the field-level diffs already catalogued for existing
modules (docs/superpowers/research/2026-07-31-field-diff-*.md) are
separate, later-scoped Phase 2 work.
EOF
)"
```

---
