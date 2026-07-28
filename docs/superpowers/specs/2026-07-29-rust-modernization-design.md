# Phase 1: Rust Modernization — Design

Status: Approved
Date: 2026-07-29

## Context

`rs_openai` (crates.io: `rs_openai`) is a Rust client for the OpenAI API, started ~2 years
ago as a learning project and last touched in the `feat: add runs apis` commit. It hasn't
kept up with either current Rust idioms or the current OpenAI API surface.

This repo's remediation work is split into two independently-specced phases, per the
project owner's explicit ordering:

1. **Phase 1 (this doc):** bring the existing code up to current Rust best practices
   *without* changing API behavior or endpoint coverage.
2. **Phase 2 (separate, later spec):** update the API surface against
   https://developers.openai.com/api/docs — add the Responses API and Conversations API,
   refresh existing endpoints (tool calling, structured outputs, multimodal content,
   `dimensions` on embeddings, `gpt-image-1`, etc.), deprecate `assistants`/`threads`/
   `runs`/`messages` in favor of Responses + Conversations (the Assistants API sunsets
   2026-08-26 — four weeks after this doc), remove long-dead `edits`/`engines` endpoints,
   and decide on Realtime API support (deferred out of the immediate round per the owner).

Phase 2 is **out of scope** for this document and is not decided beyond what's noted above.

## Current-state findings

Confirmed by reading the code and grepping `rs_openai/src`:

- `client.rs:47` — `Client::new()` is called on **every request**, so there is no
  connection pooling or reuse at all.
- `Cargo.toml` declares `backoff = "0.4.0"` but it is **never referenced** anywhere in
  `src/` — a dead dependency. The README even lists "Enhances backoff" as an open TODO,
  confirming retry/backoff was never actually wired up.
- 17 `.unwrap()`/`.expect()` call sites in production code, including:
  - `client.rs`: `org_id.parse().unwrap()` in `headers()`, `File::create(...).expect(...)`
    and `io::copy(...).expect(...)` in `resolve_file_response`, `.eventsource().unwrap()`
    in `post_stream`.
  - Multipart form building in `apis/images.rs`, `apis/audio.rs`, `apis/files.rs`,
    `apis/uploads.rs`.
- `OpenAI::new` has an inverted signature — `pub fn new(&self) -> Self` — so callers write
  `OpenAI::new(&OpenAI { api_key, org_id: None })`: construct the struct, then call `.new()`
  on it to clone itself. Not a real constructor.
- `shared/macro.rs` (~200 lines) hand-generates `From` impls, including const-generic array
  impls for every array-size combination, to implement "string or array of strings/tokens"
  for `Stop`, `Prompt`, `EmbeddingInput`, `ModerationInput`. This is the concrete case of
  "naive" logic the project owner flagged.
- **Zero tests** exist anywhere in the repo, despite CI running `cargo test --verbose`
  (currently a no-op).
- CI (`rust.yml`) uses `actions/checkout@v3` (current: v4+); `rust-clippy.yml` uses
  `actions-rs/toolchain`, which is archived/unmaintained, and only uploads clippy results
  as a SARIF report rather than failing the build — `cargo fmt` isn't checked at all.
- **The repo has two disconnected Cargo workspaces.** The root `Cargo.toml` workspace only
  contains `rs_openai`; `examples/` is a *separate* workspace with its own `[workspace]`
  block. `cargo build`/`test`/`clippy` from the repo root — what CI runs — never touches
  any of the 12 example crates, so they can silently bit-rot. Confirmed gap: `assistants`,
  `threads`, `runs`, `messages`, `batch`, and `uploads` have no example crate at all despite
  existing in `src/`.
- `edition = "2021"` in both `Cargo.toml` files; no `rust-version` (MSRV) pin anywhere.

## Goals

- Fix the connection-reuse and dead-dependency issues in the HTTP client.
- Remove panics from reachable production code paths; propagate errors instead.
- Give the crate a real constructor and support both `OpenAI-Organization` and
  `OpenAI-Project` headers (the client rework touches this code anyway; doing it once here
  avoids a second pass in Phase 2).
- Replace the derive_builder-based builders with `bon`, crate-wide, so Phase 2's interface
  rewrites don't also have to carry a builder-pattern migration.
- Simplify `shared/macro.rs`'s "string or array" modeling to a `#[serde(untagged)]` enum
  with blanket `From` impls.
- Unify the two workspaces into one, add a real automated test suite (mocked HTTP), and
  make CI actually enforce `fmt`/`clippy`/`test` instead of merely reporting on it.
- Move to edition 2024 with a pinned MSRV.

## Non-goals (explicitly deferred to Phase 2)

- Any change to request/response field shapes, endpoint coverage, or which OpenAI APIs are
  supported.
- Removing `edits`/`engines` (dead OpenAI endpoints) — deferred so Phase 1 stays purely
  about code quality, not API-surface decisions.
- Realtime API, Responses API, Conversations API, Vector Stores API.

## Design

### 1. Toolchain & CI

- Bump `edition` to `"2024"` in both `rs_openai/Cargo.toml` and the (to-be-merged) example
  crates; add `rust-version` to the workspace manifest pinning an explicit MSRV compatible
  with edition 2024 (concrete version confirmed at implementation time, not baked into this
  spec).
- Merge the two workspaces: root `Cargo.toml` becomes the single workspace listing
  `rs_openai` and every crate under `examples/*`, with `[workspace.dependencies]` centralizing
  shared versions (`tokio`, `dotenvy`, `futures`, etc.) so the lib and its examples can't
  drift apart. `examples/Cargo.toml` is deleted as a nested workspace.
- Add `[workspace.lints.rust]` / `[workspace.lints.clippy]` in the root manifest instead of
  per-crate lint attributes; each member crate opts in with `[lints] workspace = true`.
- CI (`rust.yml`): replace `actions/checkout@v3` with `v4+`, add `dtolnay/rust-toolchain`
  (replacing the archived `actions-rs/toolchain` in `rust-clippy.yml` too),
  add `Swatinem/rust-cache`. Add explicit `cargo fmt --all -- --check` and
  `cargo clippy --workspace --all-targets --all-features -- -D warnings` steps that fail
  the build on violation (today clippy issues are only uploaded as a non-blocking SARIF
  report). `cargo test --workspace` so the merged workspace's tests actually run.

### 2. HTTP client & constructor rework (`client.rs`)

- `OpenAI` gains a real constructor: `OpenAI::new(api_key: impl Into<String>) -> Self`,
  with `org_id`/`project_id` set via builder-style methods (e.g. `.with_org_id(...)`,
  `.with_project_id(...)`) rather than public struct-literal construction. This replaces
  the current `OpenAI::new(&OpenAI { .. })` self-clone pattern.
- `OpenAI` holds one `reqwest::Client`, built once in the constructor and reused for every
  request (fixes the current per-request `Client::new()` at `client.rs:47`).
- Add support for the `OpenAI-Project` header alongside the existing
  `OpenAI-Organization` header, since both are read in the same `headers()` method being
  rewritten here.
- Replace the unused `backoff` dependency with `reqwest-middleware` + `reqwest-retry`,
  wired into the shared client so retries/backoff on transient failures actually exist
  (closing the README's "Enhances backoff" TODO).
- Remove the panicking paths: `org_id`/`project_id` header construction, file writes in
  `resolve_file_response`, and `EventSource` creation in `post_stream` all return
  `Result`/propagate `OpenAIError` instead of `.unwrap()`/`.expect()`.

### 3. Error handling (`shared/response_wrapper.rs`)

- Bump `thiserror` 1.x → 2.x.
- Mark `OpenAIError` and `ApiErrorResponse`/`ApiErrorDetail` `#[non_exhaustive]`.
- Add error variants as needed to carry the newly-propagated failures from section 2
  (e.g. header-value construction, file I/O, event-source setup) instead of panicking.

### 4. Builder pattern migration: `derive_builder` → `bon`

- Migrate all 15 files currently using `#[derive(Builder)]` to `bon`'s `#[builder]`.
- Required fields become compile-time enforced (no more runtime "builder didn't set
  `model`" `OpenAIError`s for programmer errors); optional fields keep today's `Option<T>`
  + setter-takes-`impl Into<T>` ergonomics.
- Public API shape changes from `XRequestBuilder::default()...build()?` to
  `X::builder()...build()` (infallible once required fields are enforced at compile time) —
  a breaking change for consumers, acceptable pre-1.0 and bundled with this phase's version
  bump.
- Remove the `derive_builder` dependency entirely once migrated.

### 5. Type modeling cleanup (`shared/macro.rs`)

- Replace the hand-rolled `impl_from!` / `impl_from_for_array_of_tokens!` /
  `impl_from_for_array_of_token_arrays!` macros with:
  ```rust
  #[derive(Debug, Clone, Serialize, Deserialize)]
  #[serde(untagged)]
  pub enum Stop {
      String(String),
      Array(Vec<String>),
  }
  impl<S: Into<String>> From<S> for Stop { ... }
  impl<S: Into<String>> From<Vec<S>> for Stop { ... }
  ```
  applied to `Stop`, `Prompt`, `EmbeddingInput`, `ModerationInput`. Const-generic
  fixed-size-array impls are dropped — callers pass `Vec`/slices, which `impl<S: Into<String>>
  From<&[S]>` covers without needing one impl per array length.
- Net effect: same call-site ergonomics, ~200 lines of macro machinery replaced by a
  handful of generic impls.

### 6. Testing

- Add `wiremock` for HTTP-level mocking.
- Per API module (`chat`, `completions`, `embeddings`, `images`, `audio`, `files`,
  `fine_tuning`, `moderations`, `models`, `batch`, `uploads`, and the soon-to-be-deprecated
  `assistants`/`threads`/`runs`/`messages`): tests covering request serialization against a
  mock server, success-response deserialization, and API-error-response mapping to
  `OpenAIError::ApiError`.
- Unit tests for the new `bon` builders (required-field enforcement, `into` conversions)
  and the new untagged `Stop`/`Prompt`/`EmbeddingInput`/`ModerationInput` serialization
  (string form and array form both round-trip).
- These run under the now-enforced `cargo test --workspace` CI step from section 1.

## Versioning

Phase 1 bundles several breaking changes (constructor shape, builder API shape, possibly
some public error-enum changes). The crate is pre-1.0 (`0.5.0`), so these ship as a minor
version bump (`0.6.0`) per Cargo's pre-1.0 semver convention, not a new major version.

## Open items resolved during brainstorming

- Assistants API handling (Phase 2 decision, recorded here for continuity): deprecate in
  place, replace with Responses + Conversations API as the new primary surface.
- Realtime API: deferred out of this round entirely (neither phase); revisit later as its
  own spec given it needs a persistent-connection architecture this client doesn't have.
- Testing depth: full mocked-HTTP test suite wired into CI, not a minimal fmt/clippy-only
  pass.
