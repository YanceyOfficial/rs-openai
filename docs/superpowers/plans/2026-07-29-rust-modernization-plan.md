# Phase 1: Rust Modernization Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Bring the `rs_openai` crate's tooling, HTTP client, error handling, builder pattern, and test coverage up to current Rust best practices, with zero changes to which OpenAI endpoints or fields are supported.

**Architecture:** One Cargo workspace containing the `rs_openai` library and all 12 `examples/*` crates (currently two disconnected workspaces). The HTTP client builds its `reqwest` client once and reuses it, with a `reqwest-middleware`/`reqwest-retry` layer added for transient-failure retries on non-streaming requests; SSE streaming keeps a plain `reqwest::Client` because `reqwest-eventsource` only extends `reqwest::RequestBuilder`. Every `derive_builder`-based request builder becomes a `bon`-based builder. Tests are `wiremock`-backed and live next to the code they test, wired into a CI pipeline that actually gates on `fmt`/`clippy`/`test` instead of only reporting on them.

**Tech Stack:** Rust (edition 2024), `bon` (builders), `reqwest` + `reqwest-middleware` + `reqwest-retry` (HTTP + retry), `thiserror` 2.x (errors), `wiremock` (HTTP-mocked tests), GitHub Actions (`dtolnay/rust-toolchain`, `Swatinem/rust-cache`).

## Global Constraints

- Spec: `docs/superpowers/specs/2026-07-29-rust-modernization-design.md`. Everything here traces to that spec's Goals section; nothing here touches API fields/endpoints, removes `edits`/`engines`, or adds Responses/Conversations/Vector Stores/Realtime — those are Phase 2.
- Edition `2024`, MSRV `1.85` (edition 2024's floor), set once in `[workspace.package]` and inherited via `edition.workspace = true` / `rust-version.workspace = true`.
- No behavior change to any request/response JSON shape. Every task's "done" check includes `cargo build --workspace` and, once tests exist, `cargo test --workspace`.
- New dependencies are added with `cargo add`, not hand-typed version numbers, so the resolved version is whatever is actually current when the task runs. Versions named in this plan (e.g. `bon` 3.9.3, `reqwest-middleware` 0.5.2, `reqwest-retry` 0.9.1, `wiremock` 0.6.5) are what resolved when this plan was written and are for reference only.
- Every file this plan touches was read in full before this plan was written; the "before" code shown in each task is the actual current file content, not a paraphrase.

---

### Task 1: Merge the two workspaces, restore the crate's re-export surface, fix bit-rotted examples

**Files:**
- Modify: `Cargo.toml` (root)
- Modify: `rs_openai/Cargo.toml`
- Modify: `rs_openai/src/apis/assistants.rs`, `audio.rs`, `batch.rs`, `chat.rs`, `completions.rs`, `edits.rs`, `embeddings.rs`, `engines.rs`, `files.rs`, `fine_tuning.rs`, `images.rs`, `messages.rs`, `models.rs`, `moderations.rs`, `runs.rs`, `threads.rs`, `uploads.rs` (add a missing re-export; `files.rs` also gets a method rename)
- Modify: `examples/audio/Cargo.toml`, `examples/chat/Cargo.toml`, `examples/chat_stream_with_actix_web/Cargo.toml`, `examples/completions/Cargo.toml`, `examples/edits/Cargo.toml`, `examples/embeddings/Cargo.toml`, `examples/engines/Cargo.toml`, `examples/files/Cargo.toml`, `examples/fine_tuning/Cargo.toml`, `examples/images/Cargo.toml`, `examples/models/Cargo.toml`, `examples/moderations/Cargo.toml`
- Delete: `examples/Cargo.toml`
- Modify: `examples/images/src/main.rs`, `examples/audio/src/main.rs`, `examples/files/src/main.rs`, `examples/fine_tuning/src/main.rs` (fix bit rot uncovered by actually building these crates for the first time)

**Interfaces:**
- Produces: a single Cargo workspace rooted at `Cargo.toml` whose `members` list includes `rs_openai` and every `examples/*` crate; `[workspace.dependencies]` entries (`tokio`, `futures`, `dotenvy`, `serde`, `serde_json`, `reqwest`) that every later task's Cargo.toml edits reference via `{ workspace = true }`; every `apis::<name>` module now also re-exports `interfaces::<name>`'s public items, so `rs_openai::<name>::CreateXRequest` (the flat path the README documents) resolves.

This repo currently has two *disconnected* Cargo workspaces: the root only lists `rs_openai`, and `examples/Cargo.toml` is its own separate workspace. `cargo build`/`test`/`clippy` from the repo root — what CI runs — never touches any example crate, and nothing in this repo's history has ever actually compiled the `examples/` workspace as a whole either (its own members list is a separate, disconnected graph nobody runs in CI). Building it directly (`cd examples && cargo build --workspace`) to find out what Task 1 actually needs to fix surfaced three distinct, real bugs, not just the one FileMeta rename this plan originally assumed:

1. **Missing re-export (breaks 9 of 12 examples).** A past commit ("`3f97f86` feat: separate apis and interfaces") split each API module into `apis::<name>` (client methods) and `interfaces::<name>` (request/response types), but never added a re-export back from `apis::<name>` to `interfaces::<name>`. The crate's own README usage example — `use rs_openai::chat::{ChatCompletionMessageRequestBuilder, CreateChatRequestBuilder, Role}` — does not compile against the current code: `rs_openai::chat` (via `lib.rs`'s `pub use apis::*;`) only contains the `Chat` struct, not the request/builder types. Confirmed by building the real crate: `chat`, `completions`, `edits`, `embeddings`, `moderations` fail with `unresolved import` errors for exactly this reason, and `images`/`audio`/`files` would too once their other bugs (below) are fixed. Verified fix: adding `pub use crate::interfaces::chat::*;` to `apis/chat.rs` (right after its existing private `use crate::interfaces::chat;`) makes the `chat` example build clean. The same one-line addition applies to all 16 `apis/*.rs` files for consistency, even the ones no current example happens to exercise (`assistants`, `batch`, `messages`, `runs`, `threads`, `uploads`) — restoring the documented public API shape everywhere, not just where an example happened to catch it missing.
2. **A real (not example) library bug: `Files::add_upload` should be `Files::delete`.** `rs_openai/src/apis/files.rs` has a method doc-commented `/// Delete a file.` and implemented as `self.openai.delete(&format!("/files/{file_id}"), &()).await` — correct behavior, wrong name (`add_upload`, apparently copy-pasted from the uploads module and never renamed). `examples/files/src/main.rs` calls `client.files().delete("YOUR_FILE_ID")`, which doesn't exist today; the method needs renaming, not the example.
3. **`examples/fine_tuning/src/main.rs` calls three methods with stale signatures.** `FineTuning::list`, `list_events`, and `list_checkpoints` all currently require a `&fine_tuning::ListFineTuningRequest` argument that the example doesn't pass (confirmed via `cargo build`: three `E0061` "this method takes N arguments but N-1 were supplied" errors). The example predates that parameter being added and was never updated.

Separately, and independently of the above: `examples/images/src/main.rs`, `examples/audio/src/main.rs`, and `examples/files/src/main.rs` all import `rs_openai::shared::types::FileMeta`, a type renamed to `File` in commit `c2166a2 feat: rename FileMeta to File`. `examples/audio/src/main.rs` additionally imports `AudioModel` and a bare `ResponseFormat`, neither of which exist (current types are `SttModel`/`TtsModel` and `SttResponseFormat`/`TtsResponseFormat`). Building the full `examples/` workspace with every fix in this task applied (verified locally, then reverted, before writing this plan) leaves zero compile errors.

- [ ] **Step 1: Add the missing re-export to all 16 `apis/*.rs` files**

In each file, add one `pub use` line directly after the existing private `use crate::interfaces::<name>;` line. For `rs_openai/src/apis/chat.rs`:

```rust
use crate::client::OpenAI;
use crate::interfaces::chat;
pub use crate::interfaces::chat::*;
use crate::shared::response_wrapper::{OpenAIError, OpenAIResponse};
use crate::shared::utils::is_stream;
use futures::Stream;
use std::pin::Pin;
```

Apply the same pattern (add `pub use crate::interfaces::<name>::*;` immediately below `use crate::interfaces::<name>;`) to the other 15: `assistants.rs`, `audio.rs`, `batch.rs`, `completions.rs`, `edits.rs`, `embeddings.rs`, `engines.rs`, `files.rs`, `fine_tuning.rs`, `images.rs`, `messages.rs`, `models.rs`, `moderations.rs`, `runs.rs`, `threads.rs`, `uploads.rs`. Each file's `use crate::interfaces::<name>;` line has a unique, unambiguous module name, so this is a mechanical one-line insertion per file with no other content changed.

Run: `grep -c "pub use crate::interfaces" rs_openai/src/apis/*.rs`
Expected: `1` for all 16 files (`mod.rs` isn't in this list and has none).

- [ ] **Step 2: Rename `Files::add_upload` to `Files::delete`**

In `rs_openai/src/apis/files.rs`:

```rust
    /// Delete a file.
    ///
    /// # Path parameters
    ///
    /// - `file_id` - The ID of the file to use for this request
    pub async fn delete(&self, file_id: &str) -> OpenAIResponse<files::DeleteFileResponse> {
        self.openai.delete(&format!("/files/{file_id}"), &()).await
    }
```

(Only the method name on the `pub async fn` line changes — same doc comment, same body.)

- [ ] **Step 3: Fix the four bit-rotted examples so they compile against current `rs_openai`**

In `examples/images/src/main.rs`, change the import and both struct-literal usages:

```rust
use dotenvy::dotenv;
use rs_openai::{
    images::{
        CreateImageEditRequestBuilder, CreateImageRequestBuilder,
        CreateImageVariationRequestBuilder, ImageSize, ResponseFormat,
    },
    shared::types::File,
    OpenAI,
};
use std::io::prelude::*;
use std::{env::var, fs::File as StdFile};
```

(`File` now collides with `std::fs::File`; alias the std one as `StdFile` and update its two uses — `let mut file = StdFile::open(path).unwrap();` in `read_image` — the rest of the function is unchanged.) Then in `create_edit`/`create_variations`, replace `FileMeta { .. }` with `File { .. }` (three occurrences: `.image(File { buffer: origin_buffer, filename: "worldcup.png".into() })`, `.mask(File { buffer: transparent_buffer, filename: "worldcup-transparent.png".into() })`, `.image(File { buffer, filename: "worldcup.png".into() })`).

In `examples/audio/src/main.rs`, change the import and all four builder chains:

```rust
use dotenvy::dotenv;
use rs_openai::{
    audio::{
        CreateTranscriptionRequestBuilder, CreateTranslationRequestBuilder, Language,
        SttModel, SttResponseFormat,
    },
    shared::types::File,
    OpenAI,
};
use std::io::prelude::*;
use std::{env::var, fs::File as StdFile};
```

Replace every `.model(AudioModel::Whisper1)` with `.model(SttModel::Whisper1)`, every `FileMeta { .. }` with `File { .. }`, and every `ResponseFormat::Json`/`ResponseFormat::Vtt`/`ResponseFormat::Srt` with `SttResponseFormat::Json`/`SttResponseFormat::Vtt`/`SttResponseFormat::Srt`. Update `let mut file = StdFile::open(...)` in `main`.

In `examples/files/src/main.rs`:

```rust
use dotenvy::dotenv;
use rs_openai::{files::UploadFileRequestBuilder, shared::types::File, OpenAI};
use std::io::prelude::*;
use std::{env::var, fs::File as StdFile};
```

Replace `let mut file = File::open(...)` with `StdFile::open(...)`, and `FileMeta { filename: ..., buffer }` with `File { filename: ..., buffer }`.

In `examples/fine_tuning/src/main.rs`, the `list`, `list_events`, and `list_checkpoints` calls are each missing their now-required `&ListFineTuningRequest` argument. Add the import and pass an empty request (every field on `ListFineTuningRequest` is `Option`, so an unset builder is valid):

```rust
use dotenvy::dotenv;
use futures::StreamExt;
use rs_openai::{
    fine_tuning::{CreateFineTuningRequestBuilder, ListFineTuningRequestBuilder},
    OpenAI,
};
use std::env::var;
use std::io::{stdout, Write};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    let api_key = var("OPENAI_API_KEY").unwrap();

    let client = OpenAI::new(api_key);

    // create
    let req = CreateFineTuningRequestBuilder::default()
        .training_file("YOUR_FINE_TUNE_FILE")
        .model("davinci")
        .build()?;

    let res = client.fine_tuning().create(&req).await?;
    println!("{:?}", res);

    let list_req = ListFineTuningRequestBuilder::default().build()?;

    // list
    let res = client.fine_tuning().list(&list_req).await?;
    println!("{:?}", res);

    // list events
    let res = client
        .fine_tuning()
        .list_events("", &list_req)
        .await?;
    println!("{:?}", res);

    // list checkpoints
    let res = client
        .fine_tuning()
        .list_checkpoints("", &list_req)
        .await?;
    println!("{:?}", res);

    // retrieve
    let res = client.fine_tuning().retrieve("").await?;
    println!("{:?}", res);

    // cancel
    let res = client.fine_tuning().cancel("").await?;
    println!("{:?}", res);

    Ok(())
}
```

- [ ] **Step 4: Rewrite the root `Cargo.toml` as the merged workspace**

```toml
[workspace]
resolver = "2"
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

[workspace.package]
edition = "2024"
rust-version = "1.85"

[workspace.dependencies]
dotenvy = "0.15.7"
futures = "0.3"
reqwest = { version = "0.12", features = ["json", "stream", "multipart"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["full"] }

[workspace.lints.rust]
unsafe_code = "forbid"

[workspace.lints.clippy]
all = { level = "warn", priority = -1 }
```

- [ ] **Step 5: Delete the nested workspace manifest**

```bash
git rm examples/Cargo.toml
```

- [ ] **Step 6: Update `rs_openai/Cargo.toml` to opt into the workspace and drop dead/misplaced dependencies**

Two findings while merging: `dotenvy` is listed as a runtime dependency of the *library* even though nothing in `rs_openai/src` calls it (it's only referenced inside an ` ```ignore ` doc-comment block in `lib.rs`, which isn't compiled) — the examples that actually use `dotenvy` already depend on it themselves. And `tracing = "0.1.37"` is declared but `grep -rn "tracing::" rs_openai/src` returns nothing — it's dead, same as `backoff` (which Task 3 removes). Both come out here:

```toml
[package]
name = "rs_openai"
version = "0.6.0"
edition.workspace = true
rust-version.workspace = true
authors = ["Yancey Leo <developer@yanceyleo.com>"]
description = "The OpenAI Rust library provides convenient access to the OpenAI API from Rust applications."
repository = "https://github.com/YanceyOfficial/rs-openai"
homepage = "https://yanceyleo.com"
license = "MIT"
keywords = ["openai"]
readme = "../README.md"

[lints]
workspace = true

[dependencies]
backoff = "0.4.0"
derive_builder = "0.20.2"
futures = { workspace = true }
reqwest = { workspace = true }
reqwest-eventsource = "0.6.0"
serde = { workspace = true }
serde_json = { workspace = true }
strum = { version = "0.26.3", features = ["derive"] }
thiserror = "1.0.40"
tokio = { workspace = true }
tokio-stream = "0.1.12"
rand = "0.8.5"
```

(`backoff` and `derive_builder` are still present here — Task 3 removes `backoff` when it adds the retry middleware, and Task 14 removes `derive_builder` once every interface file is migrated to `bon`. Removing them now would leave the crate non-compiling until those later tasks land.)

- [ ] **Step 7: Update every example's `Cargo.toml` to use workspace dependencies**

Apply this shape to all 11 non-actix examples (`audio`, `chat`, `completions`, `edits`, `embeddings`, `engines`, `files`, `fine_tuning`, `images`, `models`, `moderations`) — using `images` as the concrete example, since it needs `futures` too and the others' dependency lists are subsets of this:

```toml
[package]
name = "images"
version = "0.1.0"
edition.workspace = true
publish = false

[dependencies]
dotenvy = { workspace = true }
rs_openai = { path = "../../rs_openai" }
tokio = { workspace = true }
```

`chat` and `completions` and `fine_tuning` also depend on `futures` (used for `StreamExt`) — add `futures = { workspace = true }` to those three only. `chat_stream_with_actix_web`'s `Cargo.toml` keeps its actix-specific dependencies as-is but switches its shared ones to workspace form:

```toml
[package]
name = "chat_stream_with_actix_web"
version = "0.1.0"
edition.workspace = true

[dependencies]
actix-web = "4"
env_logger = "0.10.0"
log = "0.4"
serde = { workspace = true }
serde_json = { workspace = true }
actix-cors = "0.6.2"
futures = { workspace = true }
dotenvy = { workspace = true }
actix-web-lab = "0.19.1"
rs_openai = { path = "../../rs_openai" }
```

- [ ] **Step 8: Verify the merged workspace builds**

Run: `cargo build --workspace --all-targets`
Expected: builds clean — no unresolved-import errors, no `FileMeta`/`AudioModel` errors, no `E0061` argument-count errors. This is the first point in this repo's history where every example is actually compiled by anything.

- [ ] **Step 9: Commit**

```bash
git add Cargo.toml rs_openai/Cargo.toml rs_openai/src/apis/ examples/
git commit -m "fix: restore apis::* re-export of interfaces::*, fix Files::delete, merge workspaces

Three real bugs, all invisible until now because examples/ was a
separate workspace nobody's CI ever built:
- apis::<name> modules never re-exported interfaces::<name>'s public
  items after the 3f97f86 apis/interfaces split, so the flat import
  path the README documents (rs_openai::chat::{CreateChatRequestBuilder,
  Role, ...}) didn't compile. Fixed for all 16 api modules.
- Files::add_upload should have been named Files::delete (correct
  behavior, copy-pasted name).
- examples/fine_tuning called list/list_events/list_checkpoints without
  their required ListFineTuningRequest argument.

Also merges the root and examples/ workspaces into one and adopts
edition 2024, and separately fixes three examples (images, audio,
files) that referenced FileMeta/AudioModel/ResponseFormat identifiers
renamed in a prior commit."
```

---

### Task 2: CI overhaul — enforce fmt/clippy/test instead of only reporting

**Files:**
- Modify: `.github/workflows/rust.yml`
- Modify: `.github/workflows/rust-clippy.yml`
- Create: `rustfmt.toml`

**Interfaces:**
- Consumes: the merged workspace from Task 1 (`cargo build --workspace` etc. now cover every crate).
- Produces: a CI pipeline where `cargo fmt --check`, `cargo clippy -- -D warnings`, and `cargo test` failures actually fail the PR check, not just get uploaded as an ignorable report.

Today, `rust.yml` only runs `cargo build --verbose` and `cargo test --verbose` (against just `rs_openai`, since the workspaces were split) — no `fmt`, no `clippy` gating. `rust-clippy.yml` runs clippy but pipes it through `clippy-sarif` with `continue-on-error: true`, uploading to the GitHub Security tab without ever failing the build; it also uses `actions-rs/toolchain`, an action archived by its maintainer years ago.

- [ ] **Step 1: Rewrite `rust.yml` to gate on fmt, clippy, build, and test across the whole workspace**

```yaml
name: Rust

on:
  push:
    branches: [ "master" ]
  pull_request:
    branches: [ "master" ]

env:
  CARGO_TERM_COLOR: always

jobs:
  check:
    runs-on: ubuntu-latest

    steps:
      - uses: actions/checkout@v4

      - name: Install Rust toolchain
        uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt, clippy

      - uses: Swatinem/rust-cache@v2

      - name: Check formatting
        run: cargo fmt --all -- --check

      - name: Clippy
        run: cargo clippy --workspace --all-targets --all-features -- -D warnings

      - name: Build
        run: cargo build --workspace --all-targets --verbose

      - name: Run tests
        run: cargo test --workspace --verbose
```

- [ ] **Step 2: Add a `rustfmt.toml` so `cargo fmt --check` has a pinned, versioned config instead of relying on whatever formatting each contributor's local rustfmt defaults to**

```toml
edition = "2024"
```

- [ ] **Step 3: Fix the toolchain action and checkout version in `rust-clippy.yml`, keep its SARIF-upload behavior**

This workflow's job is different from `rust.yml`'s: it uploads clippy findings to the GitHub Security tab on a weekly schedule, which is a legitimate use for `continue-on-error: true` (you don't want a scheduled scan to show up as a failed check on unrelated PRs). Only the stale action references change:

```yaml
# This workflow uses actions that are not certified by GitHub.
# They are provided by a third-party and are governed by
# separate terms of service, privacy policy, and support
# documentation.
# rust-clippy is a tool that runs a bunch of lints to catch common
# mistakes in your Rust code and help improve your Rust code.
# More details at https://github.com/rust-lang/rust-clippy
# and https://rust-lang.github.io/rust-clippy/

name: rust-clippy analyze

on:
  push:
    branches: [ "master" ]
  pull_request:
    # The branches below must be a subset of the branches above
    branches: [ "master" ]
  schedule:
    - cron: '24 15 * * 3'

jobs:
  rust-clippy-analyze:
    name: Run rust-clippy analyzing
    runs-on: ubuntu-latest
    permissions:
      contents: read
      security-events: write
      actions: read # only required for a private repository by github/codeql-action/upload-sarif to get the Action run status
    steps:
      - name: Checkout code
        uses: actions/checkout@v4

      - name: Install Rust toolchain
        uses: dtolnay/rust-toolchain@stable
        with:
          components: clippy

      - uses: Swatinem/rust-cache@v2

      - name: Install required cargo
        run: cargo install clippy-sarif sarif-fmt

      - name: Run rust-clippy
        run:
          cargo clippy
          --workspace
          --all-features
          --message-format=json | clippy-sarif | tee rust-clippy-results.sarif | sarif-fmt
        continue-on-error: true

      - name: Upload analysis results to GitHub
        uses: github/codeql-action/upload-sarif@v3
        with:
          sarif_file: rust-clippy-results.sarif
          wait-for-processing: true
```

- [ ] **Step 4: Verify locally before pushing**

Run: `cargo fmt --all -- --check`
Expected: passes, or run `cargo fmt --all` once to normalize the existing codebase and commit that separately first if it doesn't.

Run: `cargo clippy --workspace --all-targets --all-features -- -D warnings`
Expected: this will likely fail right now against the pre-Task-3/4 codebase (the `.unwrap()`s, unused `backoff` import concerns, etc.) — that's expected and is exactly what later tasks fix. Confirm the command itself runs (no action/toolchain errors); a failing lint output is fine at this point in the plan, a crashing command is not.

- [ ] **Step 5: Commit**

```bash
git add .github/workflows/rust.yml .github/workflows/rust-clippy.yml rustfmt.toml
git commit -m "ci: enforce fmt/clippy/test in CI, replace archived actions-rs/toolchain

actions-rs/toolchain has been unmaintained/archived for years; switch to
dtolnay/rust-toolchain and add Swatinem/rust-cache. rust.yml now fails
the build on fmt/clippy violations instead of only running build+test."
```

---

### Task 3: HTTP client rework, error handling, retry middleware

**Files:**
- Modify: `rs_openai/Cargo.toml`
- Modify: `rs_openai/src/client.rs`
- Modify: `rs_openai/src/shared/response_wrapper.rs`
- Modify: `rs_openai/src/apis/chat.rs`, `rs_openai/src/apis/completions.rs` (small ripple from `post_stream`'s new return type)
- Modify: all 12 example crates' `src/main.rs` (constructor call site)

**Interfaces:**
- Consumes: workspace/CI from Tasks 1–2.
- Produces: `OpenAI::new(api_key: impl Into<String>) -> Self`, `OpenAI::with_org_id`/`with_project_id`/`with_base_url(self, impl Into<String>) -> Self` (builder-style, chainable), and `OpenAIError` gaining `Middleware`, `InvalidHeaderValue`, `Io` variants. `pub(crate) async fn post_stream<T, F>(...) -> OpenAIResponse<Pin<Box<dyn Stream<Item = OpenAIResponse<T>> + Send>>>` (was previously infallible, now returns `OpenAIResponse<Stream>`) — Tasks 7 and 8 (`completions`, `chat`) build on this exact signature when they touch `create_with_stream`. Also produces six new `OpenAI` accessor methods — `.assistants()`, `.batch()`, `.messages()`, `.runs()`, `.threads()`, `.upload()` — that Tasks 10–12 rely on to reach those modules the intended `client.xxx().method(...)` way.

Four verified problems this task fixes: (1) `client.rs:47` calls `Client::new()` on every single request — no connection pooling at all. (2) `Cargo.toml` declares `backoff = "0.4.0"` but `grep -rn "backoff" rs_openai/src` returns nothing — it's never used; the README even lists "Enhances backoff" as an open TODO. (3) `OpenAI::new(&self) -> Self` is a self-clone, not a constructor — callers write `OpenAI::new(&OpenAI { api_key, org_id: None })`, constructing the struct just to clone it. (4) `OpenAI` only exposes 11 of its 16 API modules as accessor methods — `grep -n "pub fn.*(&self)" rs_openai/src/client.rs` confirms `audio`, `chat`, `completions`, `edits`, `embeddings`, `engines`, `files`, `fine_tuning`, `images`, `models`, `moderations` exist, but `assistants`, `batch`, `messages`, `runs`, `threads`, and `uploads` do not — those six modules are currently only reachable by importing their struct directly and calling `Xxx::new(&client)` by hand, bypassing the fluent API every other module uses. No example crate exercises any of the six, which is why this has never been caught. Separately, a scratch test run against the current code confirmed that omitting a required builder field (e.g. `model` on `CreateChatRequest`) does **not** error — `CreateChatRequestBuilder::default().build()` succeeds with `model == ""`, because `#[builder(setter(into, strip_option), default)]` falls back to `Default::default()` for any unset field, including ones with no `Option` wrapper. That's motivation carried forward into Task 5+ (bon enforces required fields at compile time, so this can't happen), not something this task fixes directly.

`reqwest-eventsource`'s `RequestBuilderExt` trait is only implemented for `reqwest::RequestBuilder`, not for the middleware-wrapped `reqwest_middleware::RequestBuilder`. So `OpenAI` holds two clients built once in the constructor: `http: ClientWithMiddleware` (retried, used for every normal request) and `raw: reqwest::Client` (used only by `post_stream`, which needs `.eventsource()`).

- [ ] **Step 1: Add/remove dependencies**

```bash
cargo remove backoff
cargo add reqwest-middleware
cargo add reqwest-retry
cargo add thiserror@2 --dry-run # confirm target version, then:
cargo add thiserror@2
cargo add --dev wiremock
```

`rs_openai/Cargo.toml`'s `[dependencies]`/`[dev-dependencies]` end up as:

```toml
[dependencies]
derive_builder = "0.20.2"
futures = { workspace = true }
reqwest = { workspace = true }
reqwest-eventsource = "0.6.0"
reqwest-middleware = "0.5.2"
reqwest-retry = "0.9.1"
serde = { workspace = true }
serde_json = { workspace = true }
strum = { version = "0.26.3", features = ["derive"] }
thiserror = "2"
tokio = { workspace = true }
tokio-stream = "0.1.12"
rand = "0.8.5"

[dev-dependencies]
wiremock = "0.6.5"
```

(`derive_builder` stays until Task 14; `bon` isn't added here — Task 4 adds it when the first interface file migrates.)

- [ ] **Step 2: Rewrite `rs_openai/src/shared/response_wrapper.rs`**

```rust
//! Errors originating from API calls, parsing responses, and reading-or-writing to the file system.
use serde::Deserialize;

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum OpenAIError {
    /// Underlying error from reqwest library after an API call was made
    #[error("http error: {0}")]
    Reqwest(#[from] reqwest::Error),
    /// Error from the retry/middleware layer wrapping the HTTP client
    #[error("http middleware error: {0}")]
    Middleware(#[from] reqwest_middleware::Error),
    /// OpenAI returns error object with details of API call failure
    #[error("{:?}", .0.error)]
    ApiError(ApiErrorResponse),
    /// Error when a response cannot be deserialized into a Rust type
    #[error("failed to deserialize api response: {0}")]
    JSONDeserialize(serde_json::Error),
    /// Error when trying to stream completions SSE
    #[error("stream failed: {0}")]
    StreamError(String),
    /// Error from client side validation
    /// or when builder fails to build request before making API call
    #[error("invalid args: {0}")]
    InvalidArgument(String),
    /// Error constructing an HTTP header value (e.g. an org/project ID with invalid bytes)
    #[error("invalid header value: {0}")]
    InvalidHeaderValue(String),
    /// Error reading or writing a file (e.g. saving a text-to-speech response)
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Deserialize)]
#[non_exhaustive]
pub struct ApiErrorDetail {
    pub message: String,
    pub r#type: String,
    pub param: Option<serde_json::Value>,
    pub code: Option<serde_json::Value>,
}

/// OpenAI API returns error object on failure
#[derive(Debug, Deserialize)]
#[non_exhaustive]
pub struct ApiErrorResponse {
    pub error: ApiErrorDetail,
}

pub type OpenAIResponse<T> = Result<T, OpenAIError>;
```

- [ ] **Step 3: Rewrite `rs_openai/src/client.rs`**

```rust
use crate::apis::{
    assistants, audio, batch, chat, completions, edits, embeddings, engines, files, fine_tuning,
    images, messages, models, moderations, runs, threads, uploads,
};
use crate::shared::response_wrapper::{ApiErrorResponse, OpenAIError, OpenAIResponse};
use futures::{stream::StreamExt, Stream};
use reqwest::{
    header::{HeaderMap, HeaderValue},
    multipart::Form,
    Method,
};
use reqwest_eventsource::{Event, EventSource, RequestBuilderExt};
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use reqwest_retry::{policies::ExponentialBackoff, RetryTransientMiddleware};
use serde::{de::DeserializeOwned, Serialize};
use std::fs::File;
use std::io;
use std::{fmt::Debug, pin::Pin};

/// Default v1 API base url
pub const API_BASE: &str = "https://api.openai.com/v1";

/// Name for organization header
pub const ORGANIZATION_HEADER: &str = "OpenAI-Organization";

/// Name for project header
pub const PROJECT_HEADER: &str = "OpenAI-Project";

pub struct OpenAI {
    api_key: String,
    org_id: Option<String>,
    project_id: Option<String>,
    base_url: String,
    /// Retried, connection-pooled client used for every non-streaming request.
    http: ClientWithMiddleware,
    /// Plain client used only for SSE streaming: `reqwest-eventsource` only
    /// extends `reqwest::RequestBuilder`, not the middleware-wrapped one.
    raw: reqwest::Client,
}

impl OpenAI {
    pub fn new(api_key: impl Into<String>) -> Self {
        let raw = reqwest::Client::new();
        let retry_policy = ExponentialBackoff::builder().build_with_max_retries(3);
        let http = ClientBuilder::new(raw.clone())
            .with(RetryTransientMiddleware::new_with_policy(retry_policy))
            .build();

        Self {
            api_key: api_key.into(),
            org_id: None,
            project_id: None,
            base_url: API_BASE.to_string(),
            http,
            raw,
        }
    }

    /// Sets the `OpenAI-Organization` header sent with every request.
    pub fn with_org_id(mut self, org_id: impl Into<String>) -> Self {
        self.org_id = Some(org_id.into());
        self
    }

    /// Sets the `OpenAI-Project` header sent with every request.
    pub fn with_project_id(mut self, project_id: impl Into<String>) -> Self {
        self.project_id = Some(project_id.into());
        self
    }

    /// Overrides the API base URL. Used by tests to point at a mock server;
    /// also usable for Azure OpenAI-compatible or proxy endpoints.
    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }

    fn headers(&self) -> Result<HeaderMap, OpenAIError> {
        let mut headers = HeaderMap::new();

        if let Some(org_id) = &self.org_id {
            headers.insert(
                ORGANIZATION_HEADER,
                HeaderValue::from_str(org_id)
                    .map_err(|e| OpenAIError::InvalidHeaderValue(e.to_string()))?,
            );
        }

        if let Some(project_id) = &self.project_id {
            headers.insert(
                PROJECT_HEADER,
                HeaderValue::from_str(project_id)
                    .map_err(|e| OpenAIError::InvalidHeaderValue(e.to_string()))?,
            );
        }

        Ok(headers)
    }

    /// Builds a request on the retried, connection-pooled client.
    fn openai_request(
        &self,
        method: Method,
        route: &str,
    ) -> Result<reqwest_middleware::RequestBuilder, OpenAIError> {
        Ok(self
            .http
            .request(method, format!("{}{route}", self.base_url))
            .headers(self.headers()?)
            .bearer_auth(&self.api_key))
    }

    /// Builds a request on the plain client. Used only by `post_stream`.
    fn raw_request(
        &self,
        method: Method,
        route: &str,
    ) -> Result<reqwest::RequestBuilder, OpenAIError> {
        Ok(self
            .raw
            .request(method, format!("{}{route}", self.base_url))
            .headers(self.headers()?)
            .bearer_auth(&self.api_key))
    }

    async fn resolve_response<T>(request: reqwest_middleware::RequestBuilder) -> OpenAIResponse<T>
    where
        T: DeserializeOwned + Debug,
    {
        let response = request.send().await?;
        let status = response.status();
        let bytes = response.bytes().await?;

        if !status.is_success() {
            let api_error: ApiErrorResponse =
                serde_json::from_slice(bytes.as_ref()).map_err(OpenAIError::JSONDeserialize)?;

            return Err(OpenAIError::ApiError(api_error));
        }

        let data: T =
            serde_json::from_slice(bytes.as_ref()).map_err(OpenAIError::JSONDeserialize)?;

        Ok(data)
    }

    async fn resolve_text_response(
        request: reqwest_middleware::RequestBuilder,
    ) -> OpenAIResponse<String> {
        let response = request.send().await?;
        let status = response.status();
        let text = response.text().await?;

        if !status.is_success() {
            let api_error: ApiErrorResponse =
                serde_json::from_slice(text.as_ref()).map_err(OpenAIError::JSONDeserialize)?;

            return Err(OpenAIError::ApiError(api_error));
        }

        Ok(text)
    }

    async fn resolve_file_response(
        request: reqwest_middleware::RequestBuilder,
        filename: &str,
    ) -> OpenAIResponse<()> {
        let response = request.send().await?;
        let status = response.status();
        let text = response.text().await?;

        if !status.is_success() {
            let api_error: ApiErrorResponse =
                serde_json::from_slice(text.as_ref()).map_err(OpenAIError::JSONDeserialize)?;

            return Err(OpenAIError::ApiError(api_error));
        }

        let mut file = File::create(filename)?;
        io::copy(&mut text.as_bytes(), &mut file)?;

        Ok(())
    }

    pub(crate) async fn get<T, F>(&self, route: &str, query: &F) -> OpenAIResponse<T>
    where
        T: DeserializeOwned + Debug,
        F: Serialize,
    {
        let request = self.openai_request(Method::GET, route)?.query(query);
        Self::resolve_response(request).await
    }

    pub(crate) async fn post<T, F>(&self, route: &str, json: &F) -> OpenAIResponse<T>
    where
        T: DeserializeOwned + Debug,
        F: Serialize,
    {
        let request = self.openai_request(Method::POST, route)?.json(json);
        Self::resolve_response(request).await
    }

    pub(crate) async fn post_form<T>(&self, route: &str, form_data: Form) -> OpenAIResponse<T>
    where
        T: DeserializeOwned + Debug,
    {
        let request = self
            .openai_request(Method::POST, route)?
            .multipart(form_data);
        Self::resolve_response(request).await
    }

    pub(crate) async fn post_form_with_text_response(
        &self,
        route: &str,
        form_data: Form,
    ) -> OpenAIResponse<String> {
        let request = self
            .openai_request(Method::POST, route)?
            .multipart(form_data);
        Self::resolve_text_response(request).await
    }

    pub(crate) async fn post_with_file_response<T>(
        &self,
        route: &str,
        json: &T,
        filename: &str,
    ) -> OpenAIResponse<()>
    where
        T: Serialize,
    {
        let request = self.openai_request(Method::POST, route)?.json(json);
        Self::resolve_file_response(request, filename).await
    }

    pub(crate) async fn post_stream<T, F>(
        &self,
        route: &str,
        json: &F,
    ) -> OpenAIResponse<Pin<Box<dyn Stream<Item = OpenAIResponse<T>> + Send>>>
    where
        T: DeserializeOwned + Debug + Send + 'static,
        F: Serialize,
    {
        let request = self.raw_request(Method::POST, route)?.json(json);
        let event_source = request
            .eventsource()
            .map_err(|e| OpenAIError::StreamError(e.to_string()))?;
        Ok(OpenAI::stream_sse(event_source).await)
    }

    pub(crate) async fn delete<T, F>(&self, route: &str, json: &F) -> OpenAIResponse<T>
    where
        T: DeserializeOwned + Debug,
        F: Serialize,
    {
        let request = self.openai_request(Method::DELETE, route)?.json(json);
        Self::resolve_response(request).await
    }

    async fn stream_sse<T>(
        mut event_source: EventSource,
    ) -> Pin<Box<dyn Stream<Item = OpenAIResponse<T>> + Send>>
    where
        T: DeserializeOwned + Debug + Send + 'static,
    {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<OpenAIResponse<T>>();

        tokio::spawn(async move {
            while let Some(evt) = event_source.next().await {
                match evt {
                    Err(e) => {
                        if tx
                            .send(Err(OpenAIError::StreamError(e.to_string())))
                            .is_err()
                        {
                            break;
                        }
                    }
                    Ok(evt) => match evt {
                        Event::Message(message) => {
                            if message.data == "[DONE]" {
                                break;
                            }

                            let response = match serde_json::from_str::<T>(&message.data) {
                                Err(e) => Err(OpenAIError::JSONDeserialize(e)),
                                Ok(output) => Ok(output),
                            };

                            if tx.send(response).is_err() {
                                break;
                            }
                        }
                        Event::Open => continue,
                    },
                }
            }

            event_source.close();
        });

        Box::pin(tokio_stream::wrappers::UnboundedReceiverStream::new(rx))
    }

    pub fn assistants(&self) -> assistants::Assistants {
        assistants::Assistants::new(self)
    }

    pub fn audio(&self) -> audio::Audio {
        audio::Audio::new(self)
    }

    pub fn batch(&self) -> batch::Batch {
        batch::Batch::new(self)
    }

    pub fn chat(&self) -> chat::Chat {
        chat::Chat::new(self)
    }

    pub fn completions(&self) -> completions::Completions {
        completions::Completions::new(self)
    }

    pub fn edits(&self) -> edits::Edits {
        edits::Edits::new(self)
    }

    pub fn embeddings(&self) -> embeddings::Embeddings {
        embeddings::Embeddings::new(self)
    }

    pub fn engines(&self) -> engines::Engines {
        engines::Engines::new(self)
    }

    pub fn files(&self) -> files::Files {
        files::Files::new(self)
    }

    pub fn fine_tuning(&self) -> fine_tuning::FineTuning {
        fine_tuning::FineTuning::new(self)
    }

    pub fn images(&self) -> images::Images {
        images::Images::new(self)
    }

    pub fn messages(&self) -> messages::Messages {
        messages::Messages::new(self)
    }

    pub fn models(&self) -> models::Models {
        models::Models::new(self)
    }

    pub fn moderations(&self) -> moderations::Moderations {
        moderations::Moderations::new(self)
    }

    pub fn runs(&self) -> runs::Runs {
        runs::Runs::new(self)
    }

    pub fn threads(&self) -> threads::Threads {
        threads::Threads::new(self)
    }

    pub fn upload(&self) -> uploads::Upload {
        uploads::Upload::new(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[test]
    fn new_sets_api_key_and_no_org_or_project() {
        let client = OpenAI::new("sk-test");
        assert_eq!(client.api_key, "sk-test");
        assert!(client.org_id.is_none());
        assert!(client.project_id.is_none());
    }

    #[test]
    fn with_org_and_project_populate_headers() {
        let client = OpenAI::new("sk-test")
            .with_org_id("org-123")
            .with_project_id("proj-456");
        let headers = client.headers().unwrap();
        assert_eq!(headers.get(ORGANIZATION_HEADER).unwrap(), "org-123");
        assert_eq!(headers.get(PROJECT_HEADER).unwrap(), "proj-456");
    }

    #[tokio::test]
    async fn retries_transient_server_error_then_succeeds() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/models"))
            .respond_with(ResponseTemplate::new(503))
            .up_to_n_times(1)
            .mount(&mock_server)
            .await;

        Mock::given(method("GET"))
            .and(path("/models"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({ "object": "list", "data": [] })),
            )
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let res: OpenAIResponse<serde_json::Value> = client.get("/models", &()).await;

        assert!(res.is_ok(), "expected the retry to succeed: {res:?}");
    }

    #[tokio::test]
    async fn maps_non_success_status_to_api_error() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/models"))
            .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({
                "error": {
                    "message": "Incorrect API key provided",
                    "type": "invalid_request_error",
                    "param": null,
                    "code": null
                }
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let res: OpenAIResponse<serde_json::Value> = client.get("/models", &()).await;

        match res {
            Err(OpenAIError::ApiError(e)) => {
                assert_eq!(e.error.message, "Incorrect API key provided");
            }
            other => panic!("expected ApiError, got {other:?}"),
        }
    }
}
```

- [ ] **Step 4: Fix the two `post_stream` callers for its new fallible return type**

In `rs_openai/src/apis/chat.rs`, `create_with_stream`'s last line changes from `Ok(self.openai.post_stream("/chat/completions", req).await)` to:

```rust
    pub async fn create_with_stream(
        &self,
        req: &chat::CreateChatRequest,
    ) -> Result<
        Pin<Box<dyn Stream<Item = OpenAIResponse<chat::ChatStreamResponse>> + Send>>,
        OpenAIError,
    > {
        if !is_stream(req.stream) {
            return Err(OpenAIError::InvalidArgument(
                "When stream is false, use Chat::create".into(),
            ));
        }

        self.openai.post_stream("/chat/completions", req).await
    }
```

In `rs_openai/src/apis/completions.rs`, the same change to `create_with_stream` — both files' `create_with_stream` now return exactly what `post_stream` returns (no `Ok(...)` wrapper, no `?`, since the types already match):

```rust
    pub async fn create_with_stream(
        &self,
        req: &completions::CreateCompletionRequest,
    ) -> Result<
        Pin<Box<dyn Stream<Item = OpenAIResponse<completions::CompletionStreamResponse>> + Send>>,
        OpenAIError,
    > {
        if !is_stream(req.stream) {
            return Err(OpenAIError::InvalidArgument(
                "When stream is false, use Completions::create".into(),
            ));
        }

        self.openai.post_stream("/completions", req).await
    }
```

- [ ] **Step 5: Update the constructor call in all 12 example crates**

Every example currently has this exact four-line pattern (verified via `grep -rn "OpenAI::new" examples/`):

```rust
    let client = OpenAI::new(&OpenAI {
        api_key,
        org_id: None,
    });
```

Replace it with a single line in each of: `examples/embeddings/src/main.rs:10`, `examples/chat/src/main.rs:15`, `examples/images/src/main.rs:17`, `examples/fine_tuning/src/main.rs:12`, `examples/models/src/main.rs:10`, `examples/audio/src/main.rs:17`, `examples/completions/src/main.rs:12`, `examples/engines/src/main.rs:10`, `examples/files/src/main.rs:11`, `examples/edits/src/main.rs:10`, `examples/moderations/src/main.rs:13`, `examples/chat_stream_with_actix_web/src/main.rs:29`:

```rust
    let client = OpenAI::new(api_key);
```

- [ ] **Step 6: Run the new tests and the full workspace build**

Run: `cargo test --package rs_openai client::tests`
Expected: all 4 tests pass (`new_sets_api_key_and_no_org_or_project`, `with_org_and_project_populate_headers`, `retries_transient_server_error_then_succeeds`, `maps_non_success_status_to_api_error`).

Run: `cargo build --workspace --all-targets`
Expected: builds clean — every example now compiles against the new one-argument `OpenAI::new`.

- [ ] **Step 7: Commit**

```bash
git add rs_openai/Cargo.toml rs_openai/src/client.rs rs_openai/src/shared/response_wrapper.rs \
        rs_openai/src/apis/chat.rs rs_openai/src/apis/completions.rs examples/
git commit -m "feat!: rework OpenAI client construction, add retry middleware

- OpenAI::new now takes the API key directly; org/project IDs and the
  base URL are set via chainable with_* methods instead of public
  struct-literal construction.
- The reqwest::Client is built once and reused (was previously
  constructed on every request).
- Added reqwest-middleware + reqwest-retry for transient-failure
  retries; removed the unused backoff dependency.
- Replaced .unwrap()/.expect() in header construction, file writes,
  and event-source setup with propagated OpenAIError variants.
- Added the six missing accessor methods (assistants, batch, messages,
  runs, threads, upload) — those modules were previously unreachable
  via the client.xxx() pattern every other module uses."
```

---

### Task 4: Test coverage for `models` and `engines` (no builders, no bon needed)

**Files:**
- Modify: `rs_openai/src/apis/models.rs`
- Modify: `rs_openai/src/apis/engines.rs`

**Interfaces:**
- Consumes: `OpenAI::new(...).with_base_url(...)` from Task 3.
- Produces: the canonical wiremock test pattern (`mock_client()` helper starting a `MockServer` and pointing an `OpenAI` at it) that Tasks 5–12 reuse verbatim in their own files.

Neither `models` nor `engines` has a `derive(Builder)` request type (their methods take only path parameters), so there's no bon migration here — just test coverage, since both currently have zero tests. `engines`'s two methods are `#[deprecated]`; calling them from tests needs `#[allow(deprecated)]` or `-D warnings` in CI (Task 2) fails the build on the deprecation warning.

- [ ] **Step 1: Write the failing tests for `models`**

Append to `rs_openai/src/apis/models.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::OpenAI;
    use crate::shared::response_wrapper::OpenAIError;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    async fn mock_client() -> (MockServer, OpenAI) {
        let mock_server = MockServer::start().await;
        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        (mock_server, client)
    }

    #[tokio::test]
    async fn list_deserializes_response() {
        let (mock_server, client) = mock_client().await;

        Mock::given(method("GET"))
            .and(path("/models"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "object": "list",
                "data": [
                    {"id": "gpt-4o-mini", "object": "model", "created": 1700000000_u32, "owned_by": "openai"}
                ]
            })))
            .mount(&mock_server)
            .await;

        let res = client.models().list().await.unwrap();
        assert_eq!(res.data.len(), 1);
        assert_eq!(res.data[0].id, "gpt-4o-mini");
    }

    #[tokio::test]
    async fn retrieve_deserializes_response() {
        let (mock_server, client) = mock_client().await;

        Mock::given(method("GET"))
            .and(path("/models/gpt-4o-mini"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "gpt-4o-mini", "object": "model", "created": 1700000000_u32, "owned_by": "openai"
            })))
            .mount(&mock_server)
            .await;

        let res = client.models().retrieve("gpt-4o-mini").await.unwrap();
        assert_eq!(res.id, "gpt-4o-mini");
    }

    #[tokio::test]
    async fn delete_maps_api_error_response() {
        let (mock_server, client) = mock_client().await;

        Mock::given(method("DELETE"))
            .and(path("/models/does-not-exist"))
            .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
                "error": {
                    "message": "The model 'does-not-exist' does not exist",
                    "type": "invalid_request_error",
                    "param": null,
                    "code": "model_not_found"
                }
            })))
            .mount(&mock_server)
            .await;

        let err = client.models().delete("does-not-exist").await.unwrap_err();
        match err {
            OpenAIError::ApiError(e) => {
                assert_eq!(e.error.code, Some(serde_json::json!("model_not_found")));
            }
            other => panic!("expected ApiError, got {other:?}"),
        }
    }
}
```

- [ ] **Step 2: Run it to see it pass (this module has no source change, only tests)**

Run: `cargo test --package rs_openai apis::models::tests`
Expected: 3 tests pass.

- [ ] **Step 3: Write and run the `engines` tests**

Append to `rs_openai/src/apis/engines.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::OpenAI;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    async fn mock_client() -> (MockServer, OpenAI) {
        let mock_server = MockServer::start().await;
        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        (mock_server, client)
    }

    #[allow(deprecated)]
    #[tokio::test]
    async fn list_deserializes_response() {
        let (mock_server, client) = mock_client().await;

        Mock::given(method("GET"))
            .and(path("/engines"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "object": "list",
                "data": [
                    {"id": "davinci", "object": "engine", "owner": "openai", "ready": true}
                ]
            })))
            .mount(&mock_server)
            .await;

        let res = client.engines().list().await.unwrap();
        assert_eq!(res.data.len(), 1);
        assert_eq!(res.data[0].id, "davinci");
    }

    #[allow(deprecated)]
    #[tokio::test]
    async fn retrieve_deserializes_response() {
        let (mock_server, client) = mock_client().await;

        Mock::given(method("GET"))
            .and(path("/engines/davinci"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "davinci", "object": "engine", "owner": "openai", "ready": true
            })))
            .mount(&mock_server)
            .await;

        let res = client.engines().retrieve("davinci").await.unwrap();
        assert_eq!(res.id, "davinci");
    }
}
```

Run: `cargo test --package rs_openai apis::engines::tests`
Expected: 2 tests pass, no deprecation warnings escape (the `#[allow(deprecated)]` suppresses them for these two test functions only — production callers of `engines()` still get the deprecation warning, which is the intended behavior).

- [ ] **Step 4: Commit**

```bash
git add rs_openai/src/apis/models.rs rs_openai/src/apis/engines.rs
git commit -m "test: add wiremock coverage for models and engines apis"
```

---

### Task 5: `edits` and `moderations` — migrate to `bon`, add tests

**Files:**
- Modify: `rs_openai/src/interfaces/edits.rs`, `rs_openai/src/interfaces/moderations.rs`
- Modify: `rs_openai/src/apis/edits.rs`, `rs_openai/src/apis/moderations.rs` (append tests)
- Modify: `examples/edits/src/main.rs`, `examples/moderations/src/main.rs`

**Interfaces:**
- Consumes: `OpenAI::new(...).with_base_url(...)` (Task 3), the re-export from `apis::edits`/`apis::moderations` (Task 1).
- Produces: the canonical bon-migration pattern (`#[derive(bon::Builder, ...)]` replacing `#[derive(derive_builder::Builder, ...)]`, `Type::builder()...build()` replacing `TypeBuilder::default()...build()?`) that Tasks 6–12 repeat for their own structs.

- [ ] **Step 1: Add `bon`**

```bash
cargo add bon --dry-run # confirm resolved version, then:
cargo add bon
```

- [ ] **Step 2: Migrate `CreateEditRequest` to `bon`**

Replace `rs_openai/src/interfaces/edits.rs` in full:

```rust
use bon::Builder;
use serde::{Deserialize, Serialize};

#[derive(Builder, Clone, Debug, Serialize)]
pub struct CreateEditRequest {
    /// ID of the model to use. You can use the `text-davinci-edit-001` or `code-davinci-edit-001` model with this endpoint.
    #[builder(into)]
    pub model: String,

    /// The input text to use as a starting point for the edit.
    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input: Option<String>,

    /// The instruction that tells the model how to edit the prompt.
    #[builder(into)]
    pub instruction: String,

    /// How many edits to generate for the input and instruction.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub n: Option<u8>, // default: 1

    /// What sampling temperature to use, between 0 and 2.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>, // min: 0, max: 2, default: 1

    /// An alternative to sampling with temperature (nucleus sampling).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>, //  default: 1
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct EditResponse {
    pub object: String,
    pub created: u32,
    pub choices: Vec<Choice>,
    pub usage: Usage,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct Choice {
    pub text: String,
    pub index: u32,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}
```

(The `OpenAIError` import and every `#[builder(...)]` container attribute from `derive_builder` are gone — `bon` needs none of them. `Default` is dropped from the derive list too: nothing in the codebase calls `CreateEditRequest::default()` directly, only ever `CreateEditRequestBuilder::default()`, which no longer exists after this migration.)

- [ ] **Step 3: Update `examples/edits/src/main.rs`**

```rust
use dotenvy::dotenv;
use rs_openai::{edits::CreateEditRequest, OpenAI};
use std::env::var;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    let api_key = var("OPENAI_API_KEY").unwrap();

    let client = OpenAI::new(api_key);

    let req = CreateEditRequest::builder()
        .model("text-davinci-edit-001")
        .input("Today is Monkey.")
        .instruction("Fix the grammar mistakes.")
        .build();

    let res = client.edits().create(&req).await?;
    println!("{:?}", res);

    Ok(())
}
```

- [ ] **Step 4: Write the `edits` tests**

Append to `rs_openai/src/apis/edits.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::OpenAI;
    use crate::interfaces::edits::CreateEditRequest;
    use wiremock::matchers::{body_json, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn create_serializes_request_and_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/edits"))
            .and(body_json(serde_json::json!({
                "model": "text-davinci-edit-001",
                "input": "Today is Monkey.",
                "instruction": "Fix the grammar mistakes."
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "object": "edit",
                "created": 1700000000_u32,
                "choices": [{"text": "Today is Monday.", "index": 0}],
                "usage": {"prompt_tokens": 10, "completion_tokens": 5, "total_tokens": 15}
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = CreateEditRequest::builder()
            .model("text-davinci-edit-001")
            .input("Today is Monkey.")
            .instruction("Fix the grammar mistakes.")
            .build();

        let res = client.edits().create(&req).await.unwrap();
        assert_eq!(res.choices[0].text, "Today is Monday.");
    }

    #[tokio::test]
    async fn create_maps_api_error() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/edits"))
            .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
                "error": {
                    "message": "instruction is required",
                    "type": "invalid_request_error",
                    "param": "instruction",
                    "code": null
                }
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = CreateEditRequest::builder()
            .model("text-davinci-edit-001")
            .instruction("")
            .build();

        let err = client.edits().create(&req).await.unwrap_err();
        match err {
            crate::shared::response_wrapper::OpenAIError::ApiError(e) => {
                assert_eq!(e.error.param, Some(serde_json::json!("instruction")));
            }
            other => panic!("expected ApiError, got {other:?}"),
        }
    }
}
```

- [ ] **Step 5: Run the `edits` tests**

Run: `cargo test --package rs_openai apis::edits::tests`
Expected: 2 tests pass.

- [ ] **Step 6: Migrate `CreateModerationRequest` to `bon`**

Replace the top of `rs_openai/src/interfaces/moderations.rs` (only the request struct's derive/attributes change; `ModerationInput` and everything below it are untouched by this task — `ModerationInput`'s `From` impls move in Task 13):

```rust
use bon::Builder;
use serde::{Deserialize, Serialize};

#[derive(Builder, Clone, Debug, Serialize)]
pub struct CreateModerationRequest {
    /// Input (or inputs) to classify. Can be a single string, an array of strings, or an array of multi-modal input objects similar to other models.
    #[builder(into)]
    pub input: ModerationInput,

    /// The content moderation model you would like to use. Learn more in [the moderation guide](https://platform.openai.com/docs/guides/moderation), and learn about available models [here](https://platform.openai.com/docs/models/moderation).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>, // default: "omni-moderation-latest"
}
```

(Everything from `#[derive(Debug, Serialize, Clone)] pub enum ModerationInput { ... }` onward in the file is unchanged.)

- [ ] **Step 7: Update `examples/moderations/src/main.rs`**

```rust
use dotenvy::dotenv;
use rs_openai::{moderations::CreateModerationRequest, OpenAI};
use std::env::var;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    let api_key = var("OPENAI_API_KEY").unwrap();

    let client = OpenAI::new(api_key);

    let req = CreateModerationRequest::builder()
        .input(["Hello, how are you today?", "This is a plain test message."])
        .build();

    let res = client.moderations().create(&req).await?;
    println!("{:?}", res);

    Ok(())
}
```

- [ ] **Step 8: Write the `moderations` tests**

Append to `rs_openai/src/apis/moderations.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::OpenAI;
    use crate::interfaces::moderations::CreateModerationRequest;
    use wiremock::matchers::{body_json, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn categories_fixture(flagged_violence: bool) -> serde_json::Value {
        serde_json::json!({
            "harassment": false, "harassment/threatening": false, "sexual": false,
            "hate": false, "hate/threatening": false, "illicit": false,
            "illicit/violent": false, "self-harm/intent": false,
            "self-harm/instructions": false, "self-harm": false,
            "sexual/minors": false, "violence": flagged_violence, "violence/graphic": false
        })
    }

    fn category_scores_fixture(violence_score: f64) -> serde_json::Value {
        serde_json::json!({
            "harassment": 0.0, "harassment/threatening": 0.0, "sexual": 0.0,
            "hate": 0.0, "hate/threatening": 0.0, "illicit": 0.0,
            "illicit/violent": 0.0, "self-harm/intent": 0.0,
            "self-harm/instructions": 0.0, "self-harm": 0.0,
            "sexual/minors": 0.0, "violence": violence_score, "violence/graphic": 0.0
        })
    }

    fn category_applied_input_types_fixture() -> serde_json::Value {
        serde_json::json!({
            "harassment": ["text"], "harassment/threatening": ["text"], "sexual": ["text"],
            "hate": ["text"], "hate/threatening": ["text"], "illicit": ["text"],
            "illicit/violent": ["text"], "self-harm/intent": ["text"],
            "self-harm/instructions": ["text"], "self-harm": ["text"],
            "sexual/minors": ["text"], "violence": ["text"], "violence/graphic": ["text"]
        })
    }

    #[tokio::test]
    async fn create_serializes_array_input_and_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/moderations"))
            .and(body_json(serde_json::json!({
                "input": ["Hello, how are you today?", "This is a plain test message."]
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "modr-123",
                "model": "omni-moderation-latest",
                "results": [{
                    "flagged": true,
                    "categories": categories_fixture(true),
                    "category_scores": category_scores_fixture(0.9),
                    "category_applied_input_types": category_applied_input_types_fixture()
                }]
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = CreateModerationRequest::builder()
            .input(["Hello, how are you today?", "This is a plain test message."])
            .build();

        let res = client.moderations().create(&req).await.unwrap();
        assert!(res.results[0].flagged);
        assert!(res.results[0].categories.violence);
        assert_eq!(res.results[0].category_scores.violence, 0.9);
    }

    #[tokio::test]
    async fn create_maps_api_error() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/moderations"))
            .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
                "error": {
                    "message": "input is required",
                    "type": "invalid_request_error",
                    "param": "input",
                    "code": null
                }
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = CreateModerationRequest::builder().input("hi").build();

        let err = client.moderations().create(&req).await.unwrap_err();
        match err {
            crate::shared::response_wrapper::OpenAIError::ApiError(e) => {
                assert_eq!(e.error.param, Some(serde_json::json!("input")));
            }
            other => panic!("expected ApiError, got {other:?}"),
        }
    }
}
```

- [ ] **Step 9: Run the `moderations` tests, then the whole workspace**

Run: `cargo test --package rs_openai apis::moderations::tests`
Expected: 2 tests pass.

Run: `cargo build --workspace --all-targets && cargo test --workspace`
Expected: clean build, all tests so far pass.

- [ ] **Step 10: Commit**

```bash
git add rs_openai/Cargo.toml rs_openai/src/interfaces/edits.rs rs_openai/src/interfaces/moderations.rs \
        rs_openai/src/apis/edits.rs rs_openai/src/apis/moderations.rs \
        examples/edits/src/main.rs examples/moderations/src/main.rs
git commit -m "refactor!: migrate edits and moderations request builders to bon

First two of the fifteen derive_builder request types migrated to bon.
Required fields (model, instruction, input) are now enforced at
compile time instead of silently defaulting to empty/Default values
at runtime — confirmed the old behavior did exactly that: building a
CreateChatRequest with no .model() call produced model == \"\" rather
than an error."
```

---

### Task 6: `embeddings` — migrate to `bon`, add tests

**Files:**
- Modify: `rs_openai/src/interfaces/embeddings.rs`
- Modify: `rs_openai/src/apis/embeddings.rs` (append tests)
- Modify: `examples/embeddings/src/main.rs`

**Interfaces:**
- Consumes: the bon-migration pattern from Task 5.
- Produces: nothing new consumed by later tasks — `embeddings` has no dependents among the other modules.

- [ ] **Step 1: Migrate `CreateEmbeddingRequest` to `bon`**

Replace the top of `rs_openai/src/interfaces/embeddings.rs` (the `EmbeddingInput` enum and everything below `CreateEmbeddingRequest` — `EmbeddingResponse`, `EmbeddingData`, `Usage` — are unchanged):

```rust
//! Get a vector representation of a given input that can be easily consumed by machine learning models and algorithms.
//!
//! Related guide: [Embeddings](https://platform.openai.com/docs/guides/embeddings)

use bon::Builder;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Clone)]
#[serde(untagged)]
pub enum EmbeddingInput {
    String(String),
    ArrayOfString(Vec<String>),
    ArrayOfTokens(Vec<u16>),
    ArrayOfTokenArrays(Vec<Vec<u16>>),
}

#[derive(Builder, Clone, Debug, Serialize)]
pub struct CreateEmbeddingRequest {
    /// ID of the model to use.
    /// Use the [List models](https://platform.openai.com/docs/api-reference/models/list) API to see all of your available models, or see our [Model overview](https://platform.openai.com/docs/models/overview) for descriptions of them.
    #[builder(into)]
    pub model: String,

    /// Input text to get embeddings for, encoded as a string or array of tokens.
    /// To get embeddings for multiple inputs in a single request, pass an array of strings or array of token arrays. Each input must not exceed 8192 tokens in length.
    #[builder(into)]
    pub input: EmbeddingInput,

    /// A unique identifier representing your end-user, which can help OpenAI to monitor and detect abuse. [Learn more](https://platform.openai.com/docs/guides/safety-best-practices/end-user-ids).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
}
```

- [ ] **Step 2: Update `examples/embeddings/src/main.rs`**

```rust
use dotenvy::dotenv;
use rs_openai::{embeddings::CreateEmbeddingRequest, OpenAI};
use std::env::var;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    let api_key = var("OPENAI_API_KEY").unwrap();

    let client = OpenAI::new(api_key);

    let req = CreateEmbeddingRequest::builder()
        .model("text-embedding-ada-002")
        .input([
            "The quick brown fox jumps over the lazy dog.",
            "Rust is a systems programming language.",
            "OpenAI provides a REST API for language models.",
            "This is a short paragraph used purely as embedding input.",
        ])
        .build();

    let res = client.embeddings().create(&req).await?;
    println!("{:?}", res);

    Ok(())
}
```

- [ ] **Step 3: Write the `embeddings` tests**

Append to `rs_openai/src/apis/embeddings.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::OpenAI;
    use crate::interfaces::embeddings::CreateEmbeddingRequest;
    use wiremock::matchers::{body_json, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn create_serializes_array_input_and_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/embeddings"))
            .and(body_json(serde_json::json!({
                "model": "text-embedding-ada-002",
                "input": ["hello", "world"]
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "object": "list",
                "data": [
                    {"object": "embedding", "embedding": [0.1, 0.2, 0.3], "index": 0},
                    {"object": "embedding", "embedding": [0.4, 0.5, 0.6], "index": 1}
                ],
                "model": "text-embedding-ada-002",
                "usage": {"prompt_tokens": 2, "total_tokens": 2}
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = CreateEmbeddingRequest::builder()
            .model("text-embedding-ada-002")
            .input(["hello", "world"])
            .build();

        let res = client.embeddings().create(&req).await.unwrap();
        assert_eq!(res.data.len(), 2);
        assert_eq!(res.data[1].embedding, vec![0.4, 0.5, 0.6]);
    }

    #[tokio::test]
    async fn create_maps_api_error() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/embeddings"))
            .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
                "error": {
                    "message": "input is required",
                    "type": "invalid_request_error",
                    "param": "input",
                    "code": null
                }
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = CreateEmbeddingRequest::builder()
            .model("text-embedding-ada-002")
            .input("x")
            .build();

        let err = client.embeddings().create(&req).await.unwrap_err();
        match err {
            crate::shared::response_wrapper::OpenAIError::ApiError(e) => {
                assert_eq!(e.error.param, Some(serde_json::json!("input")));
            }
            other => panic!("expected ApiError, got {other:?}"),
        }
    }
}
```

- [ ] **Step 4: Run the tests**

Run: `cargo test --package rs_openai apis::embeddings::tests`
Expected: 2 tests pass.

- [ ] **Step 5: Commit**

```bash
git add rs_openai/src/interfaces/embeddings.rs rs_openai/src/apis/embeddings.rs examples/embeddings/src/main.rs
git commit -m "refactor!: migrate embeddings request builder to bon"
```

---

### Task 7: `completions` — migrate to `bon`, add tests (including stream validation)

**Files:**
- Modify: `rs_openai/src/interfaces/completions.rs`
- Modify: `rs_openai/src/apis/completions.rs` (append tests)
- Modify: `examples/completions/src/main.rs`

**Interfaces:**
- Consumes: the bon-migration pattern from Tasks 5–6; `post_stream`'s fallible signature from Task 3.

- [ ] **Step 1: Migrate `CreateCompletionRequest` to `bon`**

Replace `rs_openai/src/interfaces/completions.rs` in full:

```rust
#[allow(unused)]
use crate::apis;
use crate::shared::types::Stop;
use bon::Builder;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum Prompt {
    String(String),
    ArrayOfString(Vec<String>),
    ArrayOfTokens(Vec<u16>),
    ArrayOfTokenArrays(Vec<Vec<u16>>),
}

#[derive(Builder, Clone, Debug, Serialize)]
pub struct CreateCompletionRequest {
    /// ID of the model to use.
    /// You can use the [List models](https://platform.openai.com/docs/api-reference/models/list) API to see all of your available models, or see our [Model overview](https://platform.openai.com/docs/models/overview) for descriptions of them.
    #[builder(into)]
    pub model: String,

    /// The prompt(s) to generate completions for, encoded as a string, array of strings, array of tokens, or array of token arrays.
    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt: Option<Prompt>, // default: <|endoftext|>

    /// The suffix that comes after a completion of inserted text.
    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suffix: Option<String>, // default: null

    /// The maximum number of [token](https://platform.openai.com/tokenizer) to generate in the completion.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u16>, // default: 16

    /// What sampling temperature to use, between 0 and 2.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>, // min: 0, max: 2, default: 1

    /// An alternative to sampling with temperature (nucleus sampling).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>, //  default: 1

    /// How many completions to generate for each prompt.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub n: Option<u8>, // default: 1

    /// Whether to stream back partial progress. For streamed progress, use [`create_with_stream`](apis::completions::Completions::create_with_stream).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>, // default: false

    /// Include the log probabilities on the `logprobs` most likely tokens, as well the chosen tokens.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logprobs: Option<u8>, // default: null

    /// Echo back the prompt in addition to the completion.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub echo: Option<bool>, // default: false

    /// Up to 4 sequences where the API will stop generating further tokens.
    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<Stop>, // default: null

    /// Number between -2.0 and 2.0. Positive values penalize new tokens based on whether they appear in the text so far.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<f32>, // min: -2.0, max: 2.0, default: 0

    /// Number between -2.0 and 2.0. Positive values penalize new tokens based on their existing frequency in the text so far.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<f32>, // min: -2.0, max: 2.0, default: 0

    /// Generates `best_of` completions server-side and returns the "best" one. Results cannot be streamed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub best_of: Option<f32>, // default: 1

    /// Modify the likelihood of specified tokens appearing in the completion.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logit_bias: Option<HashMap<String, serde_json::Value>>, // default: null

    /// A unique identifier representing your end-user, which can help OpenAI to monitor and detect abuse.
    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct CompletionResponse {
    pub id: String,
    pub object: String,
    pub created: u32,
    pub model: String,
    pub choices: Vec<CompletionChoice>,
    pub usage: Usage,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct CompletionChoice {
    pub text: String,
    pub index: u32,
    pub logprobs: Option<u8>,
    pub finish_reason: String,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct CompletionChoiceStream {
    pub text: String,
    pub index: usize,
    pub logprobs: Option<u8>,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct CompletionStreamResponse {
    pub id: String,
    pub object: String,
    pub created: u32,
    pub model: String,
    pub choices: Vec<CompletionChoiceStream>,
}
```

- [ ] **Step 2: Update `examples/completions/src/main.rs`**

```rust
use dotenvy::dotenv;
use futures::StreamExt;
use rs_openai::{completions::CreateCompletionRequest, OpenAI};
use std::env::var;
use std::io::{stdout, Write};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    let api_key = var("OPENAI_API_KEY").unwrap();

    let client = OpenAI::new(api_key);

    // create
    let req = CreateCompletionRequest::builder()
        .model("text-davinci-003")
        .prompt("What's your name?")
        .max_tokens(40_u16)
        .build();

    let res = client.completions().create(&req).await?;
    println!("{:?}", res);

    // create_with_stream
    let req = CreateCompletionRequest::builder()
        .model("text-davinci-003")
        .prompt("What's your name?")
        .max_tokens(40_u16)
        .stream(true)
        .build();

    let mut stream = client.completions().create_with_stream(&req).await?;

    let mut lock = stdout().lock();
    while let Some(data) = stream.next().await {
        data.unwrap().choices.iter().for_each(|choice| {
            write!(lock, "{}", choice.text).unwrap();
        });

        stdout().flush()?;
    }

    Ok(())
}
```

- [ ] **Step 3: Write the `completions` tests**

Append to `rs_openai/src/apis/completions.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::OpenAI;
    use crate::interfaces::completions::CreateCompletionRequest;
    use crate::shared::response_wrapper::OpenAIError;
    use wiremock::matchers::{body_json, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn create_serializes_request_and_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/completions"))
            .and(body_json(serde_json::json!({
                "model": "text-davinci-003",
                "prompt": "What's your name?",
                "max_tokens": 40
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "cmpl-123",
                "object": "text_completion",
                "created": 1700000000_u32,
                "model": "text-davinci-003",
                "choices": [{"text": "I'm a language model.", "index": 0, "logprobs": null, "finish_reason": "stop"}],
                "usage": {"prompt_tokens": 5, "completion_tokens": 6, "total_tokens": 11}
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = CreateCompletionRequest::builder()
            .model("text-davinci-003")
            .prompt("What's your name?")
            .max_tokens(40_u16)
            .build();

        let res = client.completions().create(&req).await.unwrap();
        assert_eq!(res.choices[0].text, "I'm a language model.");
    }

    #[tokio::test]
    async fn create_rejects_stream_true() {
        let client = OpenAI::new("sk-test");
        let req = CreateCompletionRequest::builder()
            .model("text-davinci-003")
            .stream(true)
            .build();

        let err = client.completions().create(&req).await.unwrap_err();
        assert!(matches!(err, OpenAIError::InvalidArgument(_)));
    }

    #[tokio::test]
    async fn create_maps_api_error() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/completions"))
            .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
                "error": {
                    "message": "model is required",
                    "type": "invalid_request_error",
                    "param": "model",
                    "code": null
                }
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = CreateCompletionRequest::builder().model("").build();

        let err = client.completions().create(&req).await.unwrap_err();
        match err {
            OpenAIError::ApiError(e) => {
                assert_eq!(e.error.param, Some(serde_json::json!("model")));
            }
            other => panic!("expected ApiError, got {other:?}"),
        }
    }
}
```

- [ ] **Step 4: Run the tests**

Run: `cargo test --package rs_openai apis::completions::tests`
Expected: 3 tests pass.

- [ ] **Step 5: Commit**

```bash
git add rs_openai/src/interfaces/completions.rs rs_openai/src/apis/completions.rs examples/completions/src/main.rs
git commit -m "refactor!: migrate completions request builder to bon"
```

---

### Task 8: `chat` — migrate to `bon`, add tests (including stream validation)

**Files:**
- Modify: `rs_openai/src/interfaces/chat.rs`
- Modify: `rs_openai/src/apis/chat.rs` (append tests)
- Modify: `examples/chat/src/main.rs`, `examples/chat_stream_with_actix_web/src/main.rs`

**Interfaces:**
- Consumes: the bon-migration pattern from Tasks 5–7; `post_stream`'s fallible signature from Task 3.

This is the module the README showcases, and the one whose broken re-export (Task 1) is most visible: the README's own usage example currently doesn't compile.

- [ ] **Step 1: Migrate `Role`, `ChatCompletionMessage`, and `CreateChatRequest` to `bon`**

Replace `rs_openai/src/interfaces/chat.rs` in full:

```rust
#[allow(unused)]
use crate::apis;
use crate::shared::types::Stop;
use bon::Builder;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize, Clone, Default, strum::Display)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    #[strum(serialize = "system")]
    System,
    #[default]
    #[strum(serialize = "user")]
    User,
    #[strum(serialize = "assistant")]
    Assistant,
}

#[derive(Builder, Debug, Clone, Deserialize, Serialize)]
pub struct ChatCompletionMessage {
    /// The role of the author of this message. One of `system`, `user`, or `assistant`.
    pub role: Role,

    /// The contents of the message.
    #[builder(into)]
    pub content: String,

    /// The name of the author of this message. May contain a-z, A-Z, 0-9, and underscores, with a maximum length of 64 characters.
    #[builder(into)]
    pub name: Option<String>,
}

#[derive(Builder, Clone, Debug, Serialize)]
pub struct CreateChatRequest {
    /// ID of the model to use.
    /// See the [model endpoint compatibility](https://platform.openai.com/docs/models/model-endpoint-compatibility) table for details on which models work with the Chat API.
    #[builder(into)]
    pub model: String,

    /// A list of messages describing the conversation so far.
    pub messages: Vec<ChatCompletionMessage>,

    /// What sampling temperature to use, between 0 and 2.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>, // min: 0, max: 2, default: 1

    /// An alternative to sampling with temperature, called nucleus sampling.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>, //  default: 1

    /// How many chat completion choices to generate for each input message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub n: Option<u8>, // default: 1

    /// If set, partial message deltas will be sent, like in ChatGPT.
    /// For streamed progress, use [`create_with_stream`](apis::chat::Chat::create_with_stream).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>, // default: false

    /// Up to 4 sequences where the API will stop generating further tokens.
    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<Stop>, // default: null

    /// The maximum number of tokens to generate in the chat completion.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,

    /// Number between -2.0 and 2.0. Positive values penalize new tokens based on whether they appear in the text so far.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<f32>, // min: -2.0, max: 2.0, default: 0

    /// Number between -2.0 and 2.0. Positive values penalize new tokens based on their existing frequency in the text so far.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<f32>, // min: -2.0, max: 2.0, default: 0

    /// Modify the likelihood of specified tokens appearing in the completion.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logit_bias: Option<HashMap<String, serde_json::Value>>, // default: null

    /// A unique identifier representing your end-user, which can help OpenAI to monitor and detect abuse.
    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ChatUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ChatChoice {
    pub message: ChatCompletionMessage,
    pub finish_reason: String,
    pub index: u32,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ChatResponse {
    pub id: String,
    pub object: String,
    pub created: u32,
    pub choices: Vec<ChatChoice>,
    pub usage: ChatUsage,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct Delta {
    pub content: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ChatChoiceStream {
    pub delta: Delta,
    pub finish_reason: Option<String>,
    pub index: u32,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ChatStreamResponse {
    pub id: String,
    pub object: String,
    pub model: String,
    pub created: u32,
    pub choices: Vec<ChatChoiceStream>,
}
```

- [ ] **Step 2: Update `examples/chat/src/main.rs`**

```rust
use dotenvy::dotenv;
use futures::StreamExt;
use rs_openai::{
    chat::{ChatCompletionMessage, CreateChatRequest, Role},
    OpenAI,
};
use std::env::var;
use std::io::{stdout, Write};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    let api_key = var("OPENAI_API_KEY").unwrap();

    let client = OpenAI::new(api_key);

    let req = CreateChatRequest::builder()
        .model("gpt-3.5-turbo")
        .messages(vec![ChatCompletionMessage::builder()
            .role(Role::User)
            .content("To Solve LeetCode's problem 81 in Rust.")
            .build()])
        .build();

    let res = client.chat().create(&req).await?;
    println!("{:?}", res);

    // stream mode
    let req = CreateChatRequest::builder()
        .model("gpt-3.5-turbo")
        .messages(vec![ChatCompletionMessage::builder()
            .role(Role::User)
            .content("To Solve LeetCode's problem 81 in Rust.")
            .build()])
        .stream(true)
        .build();

    let mut stream = client.chat().create_with_stream(&req).await?;

    let mut lock = stdout().lock();
    while let Some(response) = stream.next().await {
        response.unwrap().choices.iter().for_each(|choice| {
            if let Some(ref content) = choice.delta.content {
                write!(lock, "{}", content).unwrap();
            }
        });

        stdout().flush()?;
    }

    Ok(())
}
```

- [ ] **Step 3: Update `examples/chat_stream_with_actix_web/src/main.rs`**

Only the `rs_openai` import and the request-building block inside `create_chat` change:

```rust
use rs_openai::{
    chat::{ChatCompletionMessage, CreateChatRequest, Role},
    OpenAI,
};
```

```rust
    dotenv().ok();
    let api_key = var("OPENAI_API_KEY").unwrap();
    let client = OpenAI::new(api_key);

    let req = CreateChatRequest::builder()
        .model("gpt-3.5-turbo")
        .messages(vec![ChatCompletionMessage::builder()
            .role(Role::User)
            .content(&params.q)
            .build()])
        .stream(true)
        .user(&params.user_id)
        .build();
```

Everything else in the file (the actix-web server setup, the SSE forwarding loop) is unchanged.

- [ ] **Step 4: Write the `chat` tests**

Append to `rs_openai/src/apis/chat.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::OpenAI;
    use crate::interfaces::chat::{ChatCompletionMessage, CreateChatRequest, Role};
    use crate::shared::response_wrapper::OpenAIError;
    use wiremock::matchers::{body_json, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn create_serializes_request_and_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .and(body_json(serde_json::json!({
                "model": "gpt-4o-mini",
                "messages": [{"role": "user", "content": "hi"}]
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "chatcmpl-123",
                "object": "chat.completion",
                "created": 1700000000_u32,
                "choices": [{
                    "message": {"role": "assistant", "content": "Hello!", "name": null},
                    "finish_reason": "stop",
                    "index": 0
                }],
                "usage": {"prompt_tokens": 5, "completion_tokens": 2, "total_tokens": 7}
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = CreateChatRequest::builder()
            .model("gpt-4o-mini")
            .messages(vec![ChatCompletionMessage::builder()
                .role(Role::User)
                .content("hi")
                .build()])
            .build();

        let res = client.chat().create(&req).await.unwrap();
        assert_eq!(res.choices[0].message.content, "Hello!");
    }

    #[tokio::test]
    async fn create_rejects_stream_true() {
        let client = OpenAI::new("sk-test");
        let req = CreateChatRequest::builder()
            .model("gpt-4o-mini")
            .messages(vec![ChatCompletionMessage::builder()
                .role(Role::User)
                .content("hi")
                .build()])
            .stream(true)
            .build();

        let err = client.chat().create(&req).await.unwrap_err();
        assert!(matches!(err, OpenAIError::InvalidArgument(_)));
    }

    #[tokio::test]
    async fn create_with_stream_rejects_stream_false() {
        let client = OpenAI::new("sk-test");
        let req = CreateChatRequest::builder()
            .model("gpt-4o-mini")
            .messages(vec![ChatCompletionMessage::builder()
                .role(Role::User)
                .content("hi")
                .build()])
            .build();

        let err = client.chat().create_with_stream(&req).await.unwrap_err();
        assert!(matches!(err, OpenAIError::InvalidArgument(_)));
    }

    #[tokio::test]
    async fn create_maps_api_error() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({
                "error": {
                    "message": "Incorrect API key provided",
                    "type": "invalid_request_error",
                    "param": null,
                    "code": "invalid_api_key"
                }
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = CreateChatRequest::builder()
            .model("gpt-4o-mini")
            .messages(vec![ChatCompletionMessage::builder()
                .role(Role::User)
                .content("hi")
                .build()])
            .build();

        let err = client.chat().create(&req).await.unwrap_err();
        match err {
            OpenAIError::ApiError(e) => {
                assert_eq!(e.error.code, Some(serde_json::json!("invalid_api_key")));
            }
            other => panic!("expected ApiError, got {other:?}"),
        }
    }
}
```

- [ ] **Step 5: Run the tests, then the whole workspace**

Run: `cargo test --package rs_openai apis::chat::tests`
Expected: 4 tests pass.

Run: `cargo build --workspace --all-targets && cargo test --workspace`
Expected: clean build (including `chat_stream_with_actix_web`), all tests pass.

- [ ] **Step 6: Commit**

```bash
git add rs_openai/src/interfaces/chat.rs rs_openai/src/apis/chat.rs \
        examples/chat/src/main.rs examples/chat_stream_with_actix_web/src/main.rs
git commit -m "refactor!: migrate chat request/message builders to bon

Also fixes the README's own usage example, which hasn't compiled since
apis/interfaces were split (rs_openai::chat::{CreateChatRequestBuilder,
Role, ...} didn't resolve until Task 1's re-export fix)."
```

---

### Task 9: `images` and `files` — migrate to `bon`, fix a real serialization bug, add tests

**Files:**
- Modify: `rs_openai/src/interfaces/images.rs`, `rs_openai/src/interfaces/files.rs`
- Modify: `rs_openai/src/apis/images.rs`, `rs_openai/src/apis/files.rs`
- Modify: `examples/images/src/main.rs`, `examples/files/src/main.rs`

**Interfaces:**
- Consumes: the bon-migration pattern from Tasks 5–8.

Grouped together because both are multipart-upload modules with the same shape of fix needed. `images.rs` has a real, independently-discovered bug: `ImageSize`'s `#[serde(rename = ...)]` attributes are wrong for two of its three variants —

```rust
#[derive(Default, Debug, Serialize, Clone, strum::Display)]
pub enum ImageSize {
    #[strum(serialize = "256x256")]
    #[serde(rename = "256x256")]
    S256x256,

    #[strum(serialize = "512x512")]
    #[serde(rename = "256x256")]      // <- wrong, should be "512x512"
    S512x512,

    #[default]
    #[strum(serialize = "1024x1024")]
    #[serde(rename = "256x256")]      // <- wrong, should be "1024x1024"
    S1024x1024,
}
```

`Images::create` sends the whole `CreateImageRequest` (including `size`) as a JSON body via `serde`, so requesting `ImageSize::S512x512` or `ImageSize::S1024x1024` today silently sends `"256x256"` to OpenAI. `Images::create_edit`/`create_variations` don't hit this bug because they build multipart form fields with `size.to_string()`, which goes through the *correct* `strum::Display` impl, not serde — only the JSON path is broken. Separately, `CreateImageVariationRequest.size` is typed `Option<String>` while the other two request structs use `Option<ImageSize>` for the identical field — an inconsistency fixed here too (harmless to fix: the wire value is identical either way once the serde rename bug above is fixed, since `strum::Display` already produces the right strings).

- [ ] **Step 1: Migrate `images.rs` to `bon` and fix the `ImageSize` rename bug**

Replace `rs_openai/src/interfaces/images.rs` in full:

```rust
//! Given a prompt and/or an input image, the model will generate a new image.
//!
//! Related guide: [Image generation](https://platform.openai.com/docs/guides/images)

use crate::shared::types::File;
use bon::Builder;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Default, Clone, strum::Display)]
#[serde(rename_all = "snake_case")]
pub enum ResponseFormat {
    #[default]
    #[strum(serialize = "url")]
    Url,

    #[strum(serialize = "b64_json")]
    #[serde(rename = "b64_json")]
    B64Json,
}

#[derive(Default, Debug, Serialize, Clone, strum::Display)]
pub enum ImageSize {
    #[strum(serialize = "256x256")]
    #[serde(rename = "256x256")]
    S256x256,

    #[strum(serialize = "512x512")]
    #[serde(rename = "512x512")]
    S512x512,

    #[default]
    #[strum(serialize = "1024x1024")]
    #[serde(rename = "1024x1024")]
    S1024x1024,
}

#[derive(Builder, Clone, Debug, Serialize)]
pub struct CreateImageRequest {
    /// A text description of the desired image(s). The maximum length is 1000 characters.
    #[builder(into)]
    pub prompt: String,

    /// The number of images to generate. Must be between 1 and 10.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub n: Option<u8>, // default: 1, min: 1, max: 10

    /// The size of the generated images. Must be one of `256x256`, `512x512`, or `1024x1024`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<ImageSize>, // default: "1024x1024"

    /// The format in which the generated images are returned. Must be one of `url` or `b64_json`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<ResponseFormat>, // default: "url"

    /// A unique identifier representing your end-user, which can help OpenAI to monitor and detect abuse.
    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
}

#[derive(Builder, Clone, Debug, Serialize)]
pub struct CreateImageEditRequest {
    /// The image to edit. Must be a valid PNG file, less than 4MB, and square.
    /// If mask is not provided, image must have transparency, which will be used as the mask.
    pub image: File,

    /// A text description of the desired image(s). The maximum length is 1000 characters.
    #[builder(into)]
    pub prompt: String,

    /// An additional image whose fully transparent areas indicate where `image` should be edited.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mask: Option<File>,

    /// The number of images to generate. Must be between 1 and 10.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub n: Option<u8>, // default: 1, min: 1, max: 10

    /// The size of the generated images. Must be one of `256x256`, `512x512`, or `1024x1024`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<ImageSize>, // default: "1024x1024"

    /// The format in which the generated images are returned. Must be one of `url` or `b64_json`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<ResponseFormat>, // default: "url"

    /// A unique identifier representing your end-user.
    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
}

#[derive(Builder, Clone, Debug, Serialize)]
pub struct CreateImageVariationRequest {
    /// The image to use as the basis for the variation(s). Must be a valid PNG file, less than 4MB, and square.
    pub image: File,

    /// The number of images to generate. Must be between 1 and 10.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub n: Option<u8>, // default: 1, min: 1, max: 10

    /// The size of the generated images. Must be one of `256x256`, `512x512`, or `1024x1024`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<ImageSize>, // default: "1024x1024"

    /// The format in which the generated images are returned. Must be one of `url` or `b64_json`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<ResponseFormat>, // default: "url"

    /// A unique identifier representing your end-user.
    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ImageData {
    Url(String),

    #[serde(rename = "b64_json")]
    B64Json(String),
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ImageResponse {
    pub created: i64,
    pub data: Vec<ImageData>,
}
```

- [ ] **Step 2: Update `rs_openai/src/apis/images.rs`**

`create_variations` needs `.to_string()` now that `size` is `ImageSize` instead of `String`; every `.mime_str("application/octet-stream").unwrap()` becomes `.expect(...)` with a message explaining why it can't actually fail (this is not a behavior change — the literal is always a valid MIME string — just replacing a bare `.unwrap()` with a self-documenting `.expect()`):

```rust
//! Given a prompt and/or an input image, the model will generate a new image.
//!
//! Related guide: [Image generation](https://platform.openai.com/docs/guides/images)

use crate::client::OpenAI;
use crate::interfaces::images;
pub use crate::interfaces::images::*;
use crate::shared::response_wrapper::OpenAIResponse;
use reqwest::multipart::Form;

pub struct Images<'a> {
    openai: &'a OpenAI,
}

impl<'a> Images<'a> {
    pub fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }

    /// Creates an image given a prompt.
    pub async fn create(
        &self,
        req: &images::CreateImageRequest,
    ) -> OpenAIResponse<images::ImageResponse> {
        self.openai.post("/images/generations", req).await
    }

    /// Creates an edited or extended image given an original image and a prompt.
    pub async fn create_edit(
        &self,
        req: &images::CreateImageEditRequest,
    ) -> OpenAIResponse<images::ImageResponse> {
        let file_part = reqwest::multipart::Part::stream(req.image.buffer.clone())
            .file_name(req.image.filename.clone())
            .mime_str("application/octet-stream")
            .expect("\"application/octet-stream\" is always a valid mime type");

        let mut form = Form::new()
            .part("image", file_part)
            .text("prompt", req.prompt.to_string());

        if let Some(mask) = req.mask.clone() {
            let file_part = reqwest::multipart::Part::stream(req.image.buffer.clone())
                .file_name(mask.filename)
                .mime_str("application/octet-stream")
                .expect("\"application/octet-stream\" is always a valid mime type");

            form = form.part("mask", file_part);
        }

        if let Some(n) = req.n {
            form = form.text("n", n.to_string());
        }

        if let Some(size) = req.size.clone() {
            form = form.text("size", size.to_string());
        }

        if let Some(response_format) = req.response_format.clone() {
            form = form.text("response_format", response_format.to_string());
        }

        if let Some(user) = req.user.clone() {
            form = form.text("user", user);
        }

        self.openai.post_form("/images/edits", form).await
    }

    /// Creates a variation of a given image.
    pub async fn create_variations(
        &self,
        req: &images::CreateImageVariationRequest,
    ) -> OpenAIResponse<images::ImageResponse> {
        let file_part = reqwest::multipart::Part::stream(req.image.buffer.clone())
            .file_name(req.image.filename.clone())
            .mime_str("application/octet-stream")
            .expect("\"application/octet-stream\" is always a valid mime type");

        let mut form = Form::new().part("image", file_part);

        if let Some(n) = req.n {
            form = form.text("n", n.to_string());
        }

        if let Some(size) = req.size.clone() {
            form = form.text("size", size.to_string());
        }

        if let Some(response_format) = req.response_format.clone() {
            form = form.text("response_format", response_format.to_string());
        }

        if let Some(user) = req.user.clone() {
            form = form.text("user", user);
        }

        self.openai.post_form("/images/variations", form).await
    }
}
```

- [ ] **Step 3: Update `examples/images/src/main.rs`'s builder calls**

Only the import and the three `.build()` calls change (the `File`/`StdFile` fix from Task 1 stays as-is):

```rust
use dotenvy::dotenv;
use rs_openai::{
    images::{CreateImageEditRequest, CreateImageRequest, CreateImageVariationRequest, ImageSize, ResponseFormat},
    shared::types::File,
    OpenAI,
};
use std::io::prelude::*;
use std::{env::var, fs::File as StdFile};
```

```rust
    let req = CreateImageRequest::builder()
        .prompt("a hyperrealistic portrait photo of a city street at night")
        .response_format(ResponseFormat::Url)
        .size(ImageSize::S512x512)
        .n(10)
        .build();
```

```rust
    let req = CreateImageEditRequest::builder()
        .image(File {
            buffer: origin_buffer,
            filename: "worldcup.png".into(),
        })
        .mask(File {
            buffer: transparent_buffer,
            filename: "worldcup-transparent.png".into(),
        })
        .prompt("celebration ribbon")
        .n(2)
        .build();
```

```rust
    let req = CreateImageVariationRequest::builder()
        .image(File {
            buffer,
            filename: "worldcup.png".into(),
        })
        .n(2)
        .build();
```

- [ ] **Step 4: Write the `images` tests**

Append to `rs_openai/src/apis/images.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::OpenAI;
    use crate::interfaces::images::{CreateImageRequest, ImageSize, ResponseFormat};
    use crate::shared::response_wrapper::OpenAIError;
    use wiremock::matchers::{body_json, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn create_serializes_1024_size_correctly() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/images/generations"))
            .and(body_json(serde_json::json!({
                "prompt": "a red bicycle",
                "size": "1024x1024",
                "response_format": "url"
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "created": 1700000000_i64,
                "data": [{"url": "https://example.com/image.png"}]
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = CreateImageRequest::builder()
            .prompt("a red bicycle")
            .size(ImageSize::S1024x1024)
            .response_format(ResponseFormat::Url)
            .build();

        let res = client.images().create(&req).await.unwrap();
        assert_eq!(res.data.len(), 1);
    }

    #[tokio::test]
    async fn create_maps_api_error() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/images/generations"))
            .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
                "error": {
                    "message": "prompt is required",
                    "type": "invalid_request_error",
                    "param": "prompt",
                    "code": null
                }
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = CreateImageRequest::builder().prompt("").build();

        let err = client.images().create(&req).await.unwrap_err();
        match err {
            OpenAIError::ApiError(e) => assert_eq!(e.error.param, Some(serde_json::json!("prompt"))),
            other => panic!("expected ApiError, got {other:?}"),
        }
    }
}
```

The first test is the regression test for the `ImageSize` bug: it asserts the wiremock `body_json` matcher (which requires an exact match) accepts `"size": "1024x1024"` — this test would have failed against the old broken `#[serde(rename = "256x256")]` code, since the real serialized value would have been `"256x256"`.

- [ ] **Step 5: Migrate `files.rs` to `bon`**

Replace `rs_openai/src/interfaces/files.rs` in full:

```rust
use crate::shared::types::File;
use bon::Builder;
use serde::{Deserialize, Serialize};

#[derive(Builder, Clone, Debug, Serialize)]
pub struct UploadFileRequest {
    /// Name of the [JSON Lines](https://jsonlines.readthedocs.io/en/latest/) file to be uploaded.
    pub file: File,

    /// The intended purpose of the uploaded documents. Use "fine-tune" for [Fine-tuning](https://platform.openai.com/docs/api-reference/fine-tuning).
    #[builder(into)]
    pub purpose: String,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct FileResponse {
    pub id: String,
    pub object: String,
    pub bytes: u64,
    pub created_at: u32,
    pub filename: String,
    pub purpose: String,
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

- [ ] **Step 6: Update `rs_openai/src/apis/files.rs`'s `mime_str` call**

Only the `.unwrap()` → `.expect(...)` swap from Step 2's pattern applies here; the file otherwise already has its Task 1 `pub use` and `delete` rename:

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

- [ ] **Step 7: Update `examples/files/src/main.rs`'s builder call**

```rust
use dotenvy::dotenv;
use rs_openai::{files::UploadFileRequest, shared::types::File, OpenAI};
use std::io::prelude::*;
use std::{env::var, fs::File as StdFile};
```

```rust
    let req = UploadFileRequest::builder()
        .file(File {
            filename: "books_training_file.jsonl".into(),
            buffer,
        })
        .purpose("fine-tune".to_string())
        .build();
```

- [ ] **Step 8: Write the `files` tests**

Append to `rs_openai/src/apis/files.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::OpenAI;
    use crate::interfaces::files::UploadFileRequest;
    use crate::shared::response_wrapper::OpenAIError;
    use crate::shared::types::File;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn upload_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/files"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "file-123",
                "object": "file",
                "bytes": 42,
                "created_at": 1700000000_u32,
                "filename": "data.jsonl",
                "purpose": "fine-tune"
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = UploadFileRequest::builder()
            .file(File { buffer: vec![1, 2, 3], filename: "data.jsonl".into() })
            .purpose("fine-tune")
            .build();

        let res = client.files().upload(&req).await.unwrap();
        assert_eq!(res.id, "file-123");
    }

    #[tokio::test]
    async fn delete_maps_api_error() {
        let mock_server = MockServer::start().await;

        Mock::given(method("DELETE"))
            .and(path("/files/does-not-exist"))
            .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
                "error": {
                    "message": "No such file",
                    "type": "invalid_request_error",
                    "param": null,
                    "code": "file_not_found"
                }
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let err = client.files().delete("does-not-exist").await.unwrap_err();
        match err {
            OpenAIError::ApiError(e) => {
                assert_eq!(e.error.code, Some(serde_json::json!("file_not_found")));
            }
            other => panic!("expected ApiError, got {other:?}"),
        }
    }
}
```

- [ ] **Step 9: Run the tests, then the whole workspace**

Run: `cargo test --package rs_openai apis::images::tests apis::files::tests`
Expected: 4 tests pass.

Run: `cargo build --workspace --all-targets && cargo test --workspace`
Expected: clean build, all tests pass.

- [ ] **Step 10: Commit**

```bash
git add rs_openai/src/interfaces/images.rs rs_openai/src/interfaces/files.rs \
        rs_openai/src/apis/images.rs rs_openai/src/apis/files.rs \
        examples/images/src/main.rs examples/files/src/main.rs
git commit -m "fix!: correct ImageSize serde rename bug, migrate images/files to bon

ImageSize::S512x512 and ImageSize::S1024x1024 both serialized as
\"256x256\" in JSON request bodies (Images::create) due to copy-pasted
#[serde(rename)] attributes — the multipart-based create_edit/
create_variations methods were unaffected since they use strum's
Display instead. Added a regression test asserting the exact JSON
body OpenAI receives."
```

---

### Task 10: `audio` and `uploads` — migrate to `bon`, fix a misspelled form field, add tests

**Files:**
- Modify: `rs_openai/src/interfaces/audio.rs`, `rs_openai/src/interfaces/uploads.rs`
- Modify: `rs_openai/src/apis/audio.rs`, `rs_openai/src/apis/uploads.rs`
- Modify: `examples/audio/src/main.rs`

**Interfaces:**
- Consumes: the bon-migration pattern from Tasks 5–9.

`audio.rs` has a second real, independently-discovered bug, separate from the `ImageSize` one: `Audio::create_transcription_form` sends the language as a multipart field named `"laguage"` — missing the `n` — instead of `"language"`:

```rust
if let Some(language) = req.language.clone() {
    form = form.text("laguage", language.to_string());   // <- typo, OpenAI never sees this
}
```

Since OpenAI's API doesn't recognize a `laguage` field, it's silently ignored — the `language` hint on `CreateTranscriptionRequest` has never actually worked.

- [ ] **Step 1: Migrate the three `audio.rs` request structs to `bon`**

`Language` (100+ variants), `Voice`, `SttModel`, `TtsModel`, `TimestampGranularity`, `SttResponse`, `Segment`, and `Word` are untouched by this task — only the top `use` block and `CreateSpeechRequest`/`CreateTranscriptionRequest`/`CreateTranslationRequest` change. Replace the top of `rs_openai/src/interfaces/audio.rs`:

```rust
use crate::shared::types::File;
use bon::Builder;
use serde::{Deserialize, Serialize};
```

(drops the `use crate::shared::response_wrapper::OpenAIError;` and `use derive_builder::Builder;` lines) and replace the three request structs (leaving everything from `pub enum SttResponseFormat` down through `pub enum TtsModel` — i.e. everything *above* these three structs in the file — and everything from `pub enum TimestampGranularity` onward — i.e. everything *below* them — untouched):

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

#[derive(Builder, Clone, Debug, Serialize)]
pub struct CreateTranslationRequest {
    /// The audio file object (not file name) to transcribe.
    pub file: File,

    /// ID of the model to use. Only `whisper-1` is currently available.
    pub model: SttModel,

    /// An optional text to guide the model's style or continue a previous audio segment, in English.
    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,

    /// The format of the transcript output.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<SttResponseFormat>, // Defaults to json

    /// The sampling temperature, between 0 and 1.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
}
```

- [ ] **Step 2: Fix the `"laguage"` typo and swap `.unwrap()` → `.expect()` in `rs_openai/src/apis/audio.rs`**

```rust
    fn create_transcription_form(&self, req: &audio::CreateTranscriptionRequest) -> Form {
        let file_part = reqwest::multipart::Part::stream(req.file.buffer.clone())
            .file_name(req.file.filename.clone())
            .mime_str("application/octet-stream")
            .expect("\"application/octet-stream\" is always a valid mime type");

        let mut form = Form::new()
            .part("file", file_part)
            .text("model", req.model.to_string());

        if let Some(prompt) = req.prompt.clone() {
            form = form.text("prompt", prompt);
        }

        if let Some(response_format) = req.response_format.clone() {
            form = form.text("response_format", response_format.to_string());
        }

        if let Some(temperature) = req.temperature {
            form = form.text("temperature", temperature.to_string());
        }

        if let Some(language) = req.language.clone() {
            form = form.text("language", language.to_string());
        }

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

    fn create_translation_form(&self, req: &audio::CreateTranslationRequest) -> Form {
        let file_part = reqwest::multipart::Part::stream(req.file.buffer.clone())
            .file_name(req.file.filename.clone())
            .mime_str("application/octet-stream")
            .expect("\"application/octet-stream\" is always a valid mime type");

        let mut form = Form::new()
            .part("file", file_part)
            .text("model", req.model.to_string());

        if let Some(prompt) = req.prompt.clone() {
            form = form.text("prompt", prompt);
        }

        if let Some(response_format) = req.response_format.clone() {
            form = form.text("response_format", response_format.to_string());
        }

        if let Some(temperature) = req.temperature {
            form = form.text("temperature", temperature.to_string());
        }

        form
    }
```

(Only the body of these two private helper methods changes — same method signatures, same surrounding public methods in the rest of the file, unaffected.)

- [ ] **Step 3: Update `examples/audio/src/main.rs`'s builder calls**

Replace the file in full (this is Task 1's Step 3 version with only the builder syntax changed — same field values in all four functions):

```rust
use dotenvy::dotenv;
use rs_openai::{
    audio::{
        CreateTranscriptionRequest, CreateTranslationRequest, Language, SttModel,
        SttResponseFormat,
    },
    shared::types::File,
    OpenAI,
};
use std::io::prelude::*;
use std::{env::var, fs::File as StdFile};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    let api_key = var("OPENAI_API_KEY").unwrap();

    let client = OpenAI::new(api_key);

    let mut file = StdFile::open("./assets/dear_abe_san.mp4").unwrap();
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;

    create_transcription(&client, buffer.clone())?;
    create_transcription_with_text_response(&client, buffer.clone())?;
    create_translation(&client, buffer.clone())?;
    create_translation_with_text_response(&client, buffer.clone())?;

    Ok(())
}

#[tokio::main]
async fn create_transcription(
    client: &OpenAI,
    buffer: Vec<u8>,
) -> Result<(), Box<dyn std::error::Error>> {
    let req = CreateTranscriptionRequest::builder()
        .file(File {
            buffer,
            filename: "dear_abe_san.mp4".into(),
        })
        .model(SttModel::Whisper1)
        .response_format(SttResponseFormat::Json)
        .language(Language::Japanese)
        .build();

    let res = client.audio().create_transcription(&req).await?;
    println!("{:?}", res);

    Ok(())
}

#[tokio::main]
async fn create_transcription_with_text_response(
    client: &OpenAI,
    buffer: Vec<u8>,
) -> Result<(), Box<dyn std::error::Error>> {
    let req = CreateTranscriptionRequest::builder()
        .file(File {
            buffer,
            filename: "dear_abe_san.mp4".into(),
        })
        .model(SttModel::Whisper1)
        .response_format(SttResponseFormat::Vtt)
        .language(Language::Japanese)
        .build();

    let res = client
        .audio()
        .create_transcription_with_text_response(&req)
        .await?;
    println!("{:?}", res);

    Ok(())
}

#[tokio::main]
async fn create_translation(
    client: &OpenAI,
    buffer: Vec<u8>,
) -> Result<(), Box<dyn std::error::Error>> {
    let req = CreateTranslationRequest::builder()
        .file(File {
            buffer,
            filename: "dear_abe_san.mp4".into(),
        })
        .model(SttModel::Whisper1)
        .build();

    let res = client.audio().create_translation(&req).await?;
    println!("{:?}", res);

    Ok(())
}

#[tokio::main]
async fn create_translation_with_text_response(
    client: &OpenAI,
    buffer: Vec<u8>,
) -> Result<(), Box<dyn std::error::Error>> {
    let req = CreateTranslationRequest::builder()
        .file(File {
            buffer,
            filename: "dear_abe_san.mp4".into(),
        })
        .model(SttModel::Whisper1)
        .response_format(SttResponseFormat::Srt)
        .build();

    let res = client
        .audio()
        .create_translation_with_text_response(&req)
        .await?;
    println!("{:?}", res);

    Ok(())
}
```

- [ ] **Step 4: Write the `audio` tests, including a regression test for the `"laguage"` typo**

Append to `rs_openai/src/apis/audio.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::OpenAI;
    use crate::interfaces::audio::{CreateTranscriptionRequest, Language, SttModel};
    use crate::shared::types::File;
    use wiremock::{Mock, MockServer, Request, ResponseTemplate};
    use wiremock::matchers::{method, path};

    struct ContainsInBody(&'static str);

    impl wiremock::Match for ContainsInBody {
        fn matches(&self, request: &Request) -> bool {
            String::from_utf8_lossy(&request.body).contains(self.0)
        }
    }

    #[tokio::test]
    async fn create_transcription_sends_correctly_spelled_language_field() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/audio/transcriptions"))
            .and(ContainsInBody("name=\"language\""))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "text": "hello"
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = CreateTranscriptionRequest::builder()
            .file(File { buffer: vec![1, 2, 3], filename: "audio.mp4".into() })
            .model(SttModel::Whisper1)
            .language(Language::Japanese)
            .build();

        let res = client.audio().create_transcription(&req).await.unwrap();
        assert_eq!(res.text, "hello");
    }
}
```

This test would have failed against the old code: the mock only matches a request whose multipart body contains a `name="language"` part, and the old code sent `name="laguage"` instead.

- [ ] **Step 5: Migrate `uploads.rs` to `bon`**

Replace `rs_openai/src/interfaces/uploads.rs` in full:

```rust
use crate::shared::types::File;
use bon::Builder;
use serde::{Deserialize, Serialize};

#[derive(Builder, Clone, Debug, Serialize)]
pub struct UploadFileRequest {
    /// The File object (not file name) to be uploaded.
    pub file: File,

    /// The intended purpose of the uploaded file: "assistants", "vision", "batch", or "fine-tune".
    #[builder(into)]
    pub purpose: String,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct UploadFileResponse {
    id: String,
    object: String,
    bytes: u64,
    created_at: i64,
    filename: String,
    purpose: String,
    status: String,
    expires_at: i64,
}

#[derive(Builder, Clone, Debug, Serialize)]
pub struct AddUploadPartRequest {
    /// The chunk of bytes for this Part.
    pub data: File,
}

/// The upload [Part](https://platform.openai.com/docs/api-reference/uploads/part-object) object.
#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct AddUploadPartResponse {
    id: String,
    object: String,
    created_at: i64,
    upload_id: String,
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
    id: String,
    object: String,
    bytes: u64,
    created_at: i64,
    filename: String,
    purpose: String,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct CompleteUploadResponse {
    id: String,
    object: String,
    bytes: u64,
    created_at: i64,
    filename: String,
    purpose: String,
    status: String,
    expires_at: i64,
    file: CompleteUploadFile,
}
```

- [ ] **Step 6: Swap `.unwrap()` → `.expect()` in `rs_openai/src/apis/uploads.rs`**

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

    pub async fn add_upload_part(
        &self,
        upload_id: &str,
        req: &uploads::AddUploadPartRequest,
    ) -> OpenAIResponse<uploads::AddUploadPartResponse> {
        let file_part = reqwest::multipart::Part::stream(req.data.buffer.clone())
            .file_name(req.data.filename.clone())
            .mime_str("application/octet-stream")
            .expect("\"application/octet-stream\" is always a valid mime type");
        let form = Form::new().part("data", file_part);

        self.openai
            .post_form(&format!("/uploads/{upload_id}/parts"), form)
            .await
    }
```

(`complete_upload` and `cancel_upload`, further down the same file, are untouched — they don't build multipart forms. There is no `examples/uploads` crate to update — none exists today.)

- [ ] **Step 7: Write the `uploads` tests**

Append to `rs_openai/src/apis/uploads.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::OpenAI;
    use crate::interfaces::uploads::{CompleteUploadRequest, UploadFileRequest};
    use crate::shared::response_wrapper::OpenAIError;
    use crate::shared::types::File;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

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
            .file(File { buffer: vec![1, 2, 3], filename: "data.jsonl".into() })
            .purpose("fine-tune")
            .build();

        let res = client.upload().upload(&req).await.unwrap();
        assert_eq!(res.id, "upload_123");
    }

    #[tokio::test]
    async fn complete_upload_maps_api_error() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/uploads/upload_123/complete"))
            .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
                "error": {
                    "message": "byte count mismatch",
                    "type": "invalid_request_error",
                    "param": "part_ids",
                    "code": null
                }
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = CompleteUploadRequest::builder()
            .part_ids(vec!["part_1".to_string()])
            .build();

        let err = client
            .upload()
            .complete_upload("upload_123", &req)
            .await
            .unwrap_err();
        match err {
            OpenAIError::ApiError(e) => assert_eq!(e.error.param, Some(serde_json::json!("part_ids"))),
            other => panic!("expected ApiError, got {other:?}"),
        }
    }
}
```

- [ ] **Step 8: Run the tests, then the whole workspace**

Run: `cargo test --package rs_openai apis::audio::tests apis::uploads::tests`
Expected: 3 tests pass.

Run: `cargo build --workspace --all-targets && cargo test --workspace`
Expected: clean build, all tests pass.

- [ ] **Step 9: Commit**

```bash
git add rs_openai/src/interfaces/audio.rs rs_openai/src/interfaces/uploads.rs \
        rs_openai/src/apis/audio.rs rs_openai/src/apis/uploads.rs examples/audio/src/main.rs
git commit -m "fix!: correct 'laguage' multipart field typo, migrate audio/uploads to bon

Audio::create_transcription_form sent the language hint under the
field name \"laguage\" instead of \"language\", so OpenAI silently
ignored it. Added a regression test asserting the multipart body
actually contains a correctly-named language part."
```

---

### Task 11: `fine_tuning` and `batch` — migrate to `bon`, add tests

**Files:**
- Modify: `rs_openai/src/interfaces/fine_tuning.rs`, `rs_openai/src/interfaces/batch.rs`
- Modify: `rs_openai/src/apis/fine_tuning.rs`, `rs_openai/src/apis/batch.rs` (append tests only — neither file's existing logic needs changing)
- Modify: `examples/fine_tuning/src/main.rs`

**Interfaces:**
- Consumes: the bon-migration pattern from Tasks 5–10.

`CreateFineTuningRequest.suffix` is currently a private field (`suffix: Option<String>`, no `pub`) while every other field on the struct is `pub` — an inconsistency fixed here alongside the migration (adding `pub` is purely additive, not a breaking change).

- [ ] **Step 1: Migrate `CreateFineTuningRequest` and `ListFineTuningRequest` to `bon`**

Replace the top `use` block and `CreateFineTuningRequest` in `rs_openai/src/interfaces/fine_tuning.rs` (everything from `pub struct Hyperparameters` through `pub struct FineTuningResponse` is unchanged):

```rust
use bon::Builder;
use serde::{Deserialize, Serialize};

#[derive(Builder, Clone, Debug, Serialize)]
pub struct CreateFineTuningRequest {
    /// The name of the model to fine-tune. You can select one of the [supported models](https://platform.openai.com/docs/guides/fine-tuning/which-models-can-be-fine-tuned).
    #[builder(into)]
    pub model: String,

    /// The ID of an uploaded file that contains training data.
    #[builder(into)]
    pub training_file: String,

    /// The hyperparameters used for the fine-tuning job.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hyperparameters: Option<Hyperparameters>,

    /// A string of up to 40 characters that will be added to your fine-tuned model name.
    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suffix: Option<String>, // Defaults to null

    /// The ID of an uploaded file that contains validation data.
    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validation_file: Option<String>,

    /// A list of integrations to enable for your fine-tuning job.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub integrations: Option<Vec<Integration>>,

    /// The seed controls the reproducibility of the job. If not specified, one will be generated for you.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<f32>,
}
```

And, further down the same file, replace `ListFineTuningRequest` (everything from `pub struct FineTuningEventResponse` onward stays unchanged):

```rust
#[derive(Builder, Clone, Debug, Serialize)]
pub struct ListFineTuningRequest {
    /// Identifier for the last job from the previous pagination request.
    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after: Option<String>,
    /// Number of fine-tuning jobs to retrieve.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<f32>, // Defaults to 20
}
```

- [ ] **Step 2: Update `examples/fine_tuning/src/main.rs`**

Task 1 already fixed this file's missing `ListFineTuningRequest` arguments using the old `derive_builder` syntax; this step moves it to `bon` and also drops an unused `use futures::StreamExt;` import that was already dead in the original file (nothing in this example ever calls `.next()`):

```rust
use dotenvy::dotenv;
use rs_openai::{
    fine_tuning::{CreateFineTuningRequest, ListFineTuningRequest},
    OpenAI,
};
use std::env::var;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    let api_key = var("OPENAI_API_KEY").unwrap();

    let client = OpenAI::new(api_key);

    // create
    let req = CreateFineTuningRequest::builder()
        .training_file("YOUR_FINE_TUNE_FILE")
        .model("davinci")
        .build();

    let res = client.fine_tuning().create(&req).await?;
    println!("{:?}", res);

    let list_req = ListFineTuningRequest::builder().build();

    // list
    let res = client.fine_tuning().list(&list_req).await?;
    println!("{:?}", res);

    // list events
    let res = client.fine_tuning().list_events("", &list_req).await?;
    println!("{:?}", res);

    // list checkpoints
    let res = client
        .fine_tuning()
        .list_checkpoints("", &list_req)
        .await?;
    println!("{:?}", res);

    // retrieve
    let res = client.fine_tuning().retrieve("").await?;
    println!("{:?}", res);

    // cancel
    let res = client.fine_tuning().cancel("").await?;
    println!("{:?}", res);

    Ok(())
}
```

- [ ] **Step 3: Write the `fine_tuning` tests**

Append to `rs_openai/src/apis/fine_tuning.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::OpenAI;
    use crate::interfaces::fine_tuning::CreateFineTuningRequest;
    use crate::shared::response_wrapper::OpenAIError;
    use wiremock::matchers::{body_json, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn create_serializes_request_and_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/fine-tuning/jobs"))
            .and(body_json(serde_json::json!({
                "model": "davinci",
                "training_file": "file-abc"
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "object": "fine_tuning.job",
                "id": "ftjob-123",
                "model": "davinci",
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
                "estimated_finish": null
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = CreateFineTuningRequest::builder()
            .model("davinci")
            .training_file("file-abc")
            .build();

        let res = client.fine_tuning().create(&req).await.unwrap();
        assert_eq!(res.id, "ftjob-123");
    }

    #[tokio::test]
    async fn create_maps_api_error() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/fine-tuning/jobs"))
            .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
                "error": {
                    "message": "training_file is required",
                    "type": "invalid_request_error",
                    "param": "training_file",
                    "code": null
                }
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = CreateFineTuningRequest::builder().model("davinci").training_file("").build();

        let err = client.fine_tuning().create(&req).await.unwrap_err();
        match err {
            OpenAIError::ApiError(e) => {
                assert_eq!(e.error.param, Some(serde_json::json!("training_file")));
            }
            other => panic!("expected ApiError, got {other:?}"),
        }
    }
}
```

- [ ] **Step 4: Migrate `batch.rs` to `bon`**

Replace `rs_openai/src/interfaces/batch.rs` in full:

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
    pub meta: Option<serde_json::Value>,
}

#[derive(Builder, Clone, Debug, Serialize)]
pub struct ListBatchRequest {
    /// A cursor for use in pagination.
    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after: Option<String>,

    /// A limit on the number of objects to be returned. Limit can range between 1 and 100, and the default is 20.
    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct RequestCounts {
    total: u32,
    completed: u32,
    failed: u32,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct BatchResponse {
    id: String,
    object: String,
    endpoint: String,
    errors: Option<String>,
    input_file_id: String,
    completion_window: String,
    status: String,
    output_file_id: Option<String>,
    error_file_id: Option<String>,
    created_at: i64,
    in_progress_at: Option<i64>,
    expires_at: Option<i64>,
    finalizing_at: Option<i64>,
    completed_at: Option<i64>,
    failed_at: Option<i64>,
    expired_at: Option<i64>,
    cancelling_at: Option<i64>,
    cancelled_at: Option<i64>,
    request_counts: RequestCounts,
    metadata: serde_json::Value,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ListBatchResponse {
    object: String,
    data: Vec<BatchResponse>,
}
```

- [ ] **Step 5: Write the `batch` tests**

Append to `rs_openai/src/apis/batch.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::OpenAI;
    use crate::interfaces::batch::CreateBatchRequest;
    use crate::shared::response_wrapper::OpenAIError;
    use wiremock::matchers::{body_json, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn batch_response_json(id: &str, status: &str) -> serde_json::Value {
        serde_json::json!({
            "id": id, "object": "batch", "endpoint": "/v1/chat/completions", "errors": null,
            "input_file_id": "file-abc", "completion_window": "24h", "status": status,
            "output_file_id": null, "error_file_id": null, "created_at": 1700000000_i64,
            "in_progress_at": null, "expires_at": null, "finalizing_at": null,
            "completed_at": null, "failed_at": null, "expired_at": null,
            "cancelling_at": null, "cancelled_at": null,
            "request_counts": {"total": 0, "completed": 0, "failed": 0},
            "metadata": {}
        })
    }

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
            .respond_with(ResponseTemplate::new(200).set_body_json(batch_response_json("batch_123", "validating")))
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
}
```

- [ ] **Step 6: Run the tests, then the whole workspace**

Run: `cargo test --package rs_openai apis::fine_tuning::tests apis::batch::tests`
Expected: 4 tests pass.

Run: `cargo build --workspace --all-targets && cargo test --workspace`
Expected: clean build, all tests pass.

- [ ] **Step 7: Commit**

```bash
git add rs_openai/src/interfaces/fine_tuning.rs rs_openai/src/interfaces/batch.rs \
        rs_openai/src/apis/fine_tuning.rs rs_openai/src/apis/batch.rs examples/fine_tuning/src/main.rs
git commit -m "refactor!: migrate fine_tuning and batch request builders to bon"
```

---

### Task 12: `assistants`, `messages`, `threads`, `runs` — migrate to `bon`, add tests

**Files:**
- Modify: `rs_openai/src/interfaces/assistants.rs`, `messages.rs`, `threads.rs`, `runs.rs`
- Modify: `rs_openai/src/apis/assistants.rs`, `messages.rs`, `threads.rs`, `runs.rs` (append tests only)

**Interfaces:**
- Consumes: the bon-migration pattern from Tasks 5–11.

These four modules implement the Assistants API, which — per the approved spec — is being deprecated in favor of Responses + Conversations in Phase 2 (OpenAI itself sunsets the Assistants API on 2026-08-26). They still get the same builder migration and test coverage as everything else in Phase 1, since Phase 2 hasn't landed yet and this plan's job is to leave the whole crate in good shape; Phase 2 will delete or deprecate these modules outright rather than build further on them, which is also why this task does not attempt to deduplicate the `Content`/`ImageFileType`/`ImageUrl`/`Attachment`/etc. types that are copy-pasted identically across `messages.rs`, `threads.rs`, and `runs.rs` — restructuring code that Phase 2 is likely to delete isn't worth the risk. No example crates exist for any of these four modules today, so there are no `examples/` files to update in this task.

- [ ] **Step 1: Migrate `assistants.rs` to `bon`**

Replace the top `use` block and `AssistantRequest` in `rs_openai/src/interfaces/assistants.rs` (everything from `pub enum Tools` through `pub struct AssistantResponse` is unchanged):

```rust
use std::collections::HashMap;

use bon::Builder;
use serde::{Deserialize, Serialize};

#[derive(Builder, Clone, Debug, Serialize)]
pub struct AssistantRequest {
    /// ID of the model to use.
    #[builder(into)]
    pub model: String,

    /// The name of the assistant. The maximum length is 256 characters.
    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// The description of the assistant. The maximum length is 512 characters.
    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// The system instructions that the assistant uses. The maximum length is 256,000 characters.
    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instructions: Option<String>,

    /// A list of tools enabled on the assistant. There can be a maximum of 128 tools per assistant.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Tools>>,

    /// A set of resources that are used by the assistant's tools.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_resources: Option<ToolResources>,

    /// Set of 16 key-value pairs that can be attached to an object.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, serde_json::Value>>,

    /// What sampling temperature to use, between 0 and 2.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>, // Defaults to 1

    /// An alternative to sampling with temperature, called nucleus sampling.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f64>, // Defaults to 1

    /// Specifies the format that the model must output.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<ResponseType>,
}
```

And, further down the same file, replace `ListAssistantRequest` (`ListAssistantResponse` and `DeleteAssistantResponse` below it are unchanged):

```rust
#[derive(Builder, Clone, Debug, Serialize)]
pub struct ListAssistantRequest {
    /// A limit on the number of objects to be returned. Limit can range between 1 and 100, and the default is 20.
    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<String>,

    /// Sort order by the `created_at` timestamp of the objects. `asc` or `desc`.
    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order: Option<String>,

    /// A cursor for use in pagination.
    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after: Option<String>,

    /// A cursor for use in pagination.
    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub before: Option<String>,
}
```

- [ ] **Step 2: Write the `assistants` tests**

Append to `rs_openai/src/apis/assistants.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::OpenAI;
    use crate::interfaces::assistants::AssistantRequest;
    use crate::shared::response_wrapper::OpenAIError;
    use wiremock::matchers::{body_json, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn create_assistant_serializes_request_and_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/assistants"))
            .and(body_json(serde_json::json!({"model": "gpt-4o-mini"})))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "asst_123", "object": "assistant", "created_at": 1700000000_u64,
                "name": null, "description": null, "model": "gpt-4o-mini", "instructions": null,
                "tools": [], "tool_resources": null, "metadata": {},
                "temperature": 1.0, "top_p": 1.0, "response_format": "auto"
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = AssistantRequest::builder().model("gpt-4o-mini").build();

        let res = client.assistants().create_assistant(&req).await.unwrap();
        assert_eq!(res.id, "asst_123");
    }

    #[tokio::test]
    async fn delete_assistant_maps_api_error() {
        let mock_server = MockServer::start().await;

        Mock::given(method("DELETE"))
            .and(path("/assistants/does-not-exist"))
            .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
                "error": {
                    "message": "No assistant found with id 'does-not-exist'",
                    "type": "invalid_request_error",
                    "param": null,
                    "code": null
                }
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let err = client.assistants().delete_assistant("does-not-exist").await.unwrap_err();
        assert!(matches!(err, OpenAIError::ApiError(_)));
    }
}
```

- [ ] **Step 3: Migrate `messages.rs` to `bon`**

Replace `rs_openai/src/interfaces/messages.rs` in full:

```rust
use bon::Builder;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Builder, Clone, Debug, Serialize)]
pub struct CreateMessageRequest {
    /// The role of the entity that is creating the message: `user` or `assistant`.
    #[builder(into)]
    pub role: String,
    /// The content of the message in array of text and/or images.
    pub content: Content,
    /// A list of files attached to the message, and the tools they should be added to.
    pub attachments: Option<Vec<Attachment>>,
    /// Set of 16 key-value pairs that can be attached to an object.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Serialize, Deserialize, Clone, strum::Display)]
#[serde(untagged)]
pub enum Content {
    /// The text contents of the message.
    TextContent(String),
    /// An array of content parts with a defined type.
    ArrayOfContentParts(Vec<ArrayOfContentParts>),
}

impl Default for Content {
    fn default() -> Self {
        Content::TextContent("".to_string())
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, strum::Display)]
#[serde(untagged)]
pub enum ArrayOfContentParts {
    /// References an image [File](https://platform.openai.com/docs/api-reference/files) in the content of a message.
    ImageFileType(ImageFileType),
    /// References an image URL in the content of a message.
    ImageUrlType(ImageUrlType),
    /// The text content that is part of a message.
    TextType(TextType),
}

#[derive(Builder, Clone, Debug, Serialize, Deserialize)]
pub struct ImageFileType {
    /// Always `image_file`.
    #[builder(into)]
    pub r#type: String,
    pub image_file: ImageFile,
}

#[derive(Builder, Clone, Debug, Serialize, Deserialize)]
pub struct ImageFile {
    /// The [File](https://platform.openai.com/docs/api-reference/files) ID of the image in the message content.
    #[builder(into)]
    pub file_id: String,

    /// `low` uses fewer tokens, you can opt in to high resolution using `high`.
    #[builder(into)]
    pub detail: Option<String>, // Defaults to auto
}

#[derive(Builder, Clone, Debug, Serialize, Deserialize)]
pub struct ImageUrlType {
    /// The type of the content part.
    #[builder(into)]
    pub r#type: String,
    pub image_url: ImageUrl,
}

#[derive(Builder, Clone, Debug, Serialize, Deserialize)]
pub struct ImageUrl {
    /// The external URL of the image: jpeg, jpg, png, gif, or webp.
    #[builder(into)]
    pub url: String,
    /// `low` uses fewer tokens, you can opt in to high resolution using `high`. Defaults to auto.
    #[builder(into)]
    pub detail: Option<String>,
}

#[derive(Builder, Clone, Debug, Serialize, Deserialize)]
pub struct TextType {
    /// Always `text`.
    #[builder(into)]
    pub r#type: String,
    /// Text content to be sent to the model.
    #[builder(into)]
    pub text: String,
}

#[derive(Builder, Clone, Debug, Serialize, Deserialize)]
pub struct Attachment {
    /// The ID of the file to attach to the message.
    #[builder(into)]
    pub file_id: Option<String>,
    /// The tools to add this file to.
    pub tools: Option<Vec<Tool>>,
}

#[derive(Debug, Serialize, Deserialize, Clone, strum::Display)]
pub enum Tool {
    CodeInterpreterTool(CodeInterpreterTool),
    FileSearchTool(FileSearchTool),
}

#[derive(Builder, Clone, Debug, Serialize, Deserialize)]
pub struct CodeInterpreterTool {
    /// The type of tool being defined: `code_interpreter`.
    #[builder(into)]
    pub r#type: String,
}

#[derive(Builder, Clone, Debug, Serialize, Deserialize)]
pub struct FileSearchTool {
    /// The type of tool being defined: `file_search`.
    #[builder(into)]
    pub r#type: String,
}

/// Represents a thread that contains [messages](https://platform.openai.com/docs/api-reference/messages).
#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct MessageResponse {
    pub id: String,
    pub object: String,
    pub created_at: u64,
    pub thread_id: String,
    pub status: String,
    pub incomplete_details: Option<IncompleteDetails>,
    pub completed_at: u64,
    pub incomplete_at: u64,
    pub role: String,
    pub content: Content,
    pub assistant_id: Option<String>,
    pub run_id: Option<String>,
    pub attachments: Option<Vec<Attachment>>,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// The reason the message is incomplete.
#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct IncompleteDetails {
    pub reason: String,
}

#[derive(Builder, Clone, Debug, Serialize)]
pub struct ListMessageRequest {
    /// A limit on the number of objects to be returned. Limit can range between 1 and 100, and the default is 20.
    pub limit: Option<i32>,
    /// Sort order by the `created_at` timestamp of the objects.
    #[builder(into)]
    pub order: Option<String>,
    /// A cursor for use in pagination.
    #[builder(into)]
    pub after: Option<String>,
    /// A cursor for use in pagination.
    #[builder(into)]
    pub before: Option<String>,
    /// Filter messages by the run ID that generated them.
    #[builder(into)]
    pub run_id: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ListMessageResponse {
    pub object: String,
    pub data: Vec<MessageResponse>,
    first_id: String,
    last_id: String,
    has_more: bool,
}

#[derive(Builder, Clone, Debug, Serialize)]
pub struct ModifyMessageRequest {
    /// Set of 16 key-value pairs that can be attached to an object.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct DeleteMessageResponse {
    pub id: String,
    pub object: String,
    pub delete: bool,
}
```

- [ ] **Step 4: Write the `messages` tests**

Append to `rs_openai/src/apis/messages.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::OpenAI;
    use crate::interfaces::messages::{Content, CreateMessageRequest};
    use crate::shared::response_wrapper::OpenAIError;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn create_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/threads/thread_123/messages"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "msg_123", "object": "thread.message", "created_at": 1700000000_u64,
                "thread_id": "thread_123", "status": "completed", "incomplete_details": null,
                "completed_at": 1700000001_u64, "incomplete_at": 0, "role": "user",
                "content": "hello", "assistant_id": null, "run_id": null,
                "attachments": null, "metadata": {}
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = CreateMessageRequest::builder()
            .role("user")
            .content(Content::TextContent("hello".to_string()))
            .build();

        let res = client.messages().create("thread_123", &req).await.unwrap();
        assert_eq!(res.id, "msg_123");
    }

    #[tokio::test]
    async fn delete_maps_api_error() {
        let mock_server = MockServer::start().await;

        Mock::given(method("DELETE"))
            .and(path("/threads/thread_123/messages/msg_missing"))
            .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
                "error": {"message": "No message found", "type": "invalid_request_error", "param": null, "code": null}
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let err = client.messages().delete("thread_123", "msg_missing").await.unwrap_err();
        assert!(matches!(err, OpenAIError::ApiError(_)));
    }
}
```

- [ ] **Step 5: Migrate `threads.rs` to `bon`**

Replace `rs_openai/src/interfaces/threads.rs` in full — this file's `Message`/`Content`/`ArrayOfContentParts`/`ImageFileType`/`ImageFile`/`ImageUrlType`/`ImageUrl`/`TextType`/`Attachment`/`Tool`/`CodeInterpreterTool`/`FileSearchTool` are a separate, independent copy of the same shapes from `messages.rs` (already duplicated in the current code, not something this task introduces):

```rust
use super::assistants::ToolResources;
use bon::Builder;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Builder, Clone, Debug, Serialize)]
pub struct CreateThreadRequest {
    /// A list of [messages](https://platform.openai.com/docs/api-reference/messages) to start the thread with.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub messages: Option<Vec<Message>>,

    /// A set of resources that are used by the assistant's tools.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_resources: Option<ToolResources>,

    /// Set of 16 key-value pairs that can be attached to an object.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Builder, Clone, Debug, Serialize)]
pub struct Message {
    /// The role of the entity that is creating the message: `user` or `assistant`.
    #[builder(into)]
    pub role: String,
    pub content: Content,
    /// A list of files attached to the message, and the tools they should be added to.
    pub attachments: Option<Vec<Attachment>>,
    /// Set of 16 key-value pairs that can be attached to an object.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Serialize, Clone, strum::Display)]
pub enum Content {
    TextContent(String),
    ArrayOfContentParts(Vec<ArrayOfContentParts>),
}

#[derive(Debug, Serialize, Clone, strum::Display)]
pub enum ArrayOfContentParts {
    ImageFileType(ImageFileType),
    ImageUrlType(ImageUrlType),
    TextType(TextType),
}

#[derive(Builder, Clone, Debug, Serialize)]
pub struct ImageFileType {
    #[builder(into)]
    pub r#type: String,
    pub image_file: ImageFile,
}

#[derive(Builder, Clone, Debug, Serialize)]
pub struct ImageFile {
    #[builder(into)]
    pub file_id: String,
    #[builder(into)]
    pub detail: Option<String>,
}

#[derive(Builder, Clone, Debug, Serialize)]
pub struct ImageUrlType {
    #[builder(into)]
    pub r#type: String,
    pub image_url: ImageUrl,
}

#[derive(Builder, Clone, Debug, Serialize)]
pub struct ImageUrl {
    #[builder(into)]
    pub url: String,
    #[builder(into)]
    pub detail: Option<String>,
}

#[derive(Builder, Clone, Debug, Serialize)]
pub struct TextType {
    #[builder(into)]
    pub r#type: String,
    #[builder(into)]
    pub text: String,
}

#[derive(Builder, Clone, Debug, Serialize)]
pub struct Attachment {
    #[builder(into)]
    pub file_id: Option<String>,
    pub tools: Option<Vec<Tool>>,
}

#[derive(Debug, Serialize, Clone, strum::Display)]
pub enum Tool {
    CodeInterpreterTool(CodeInterpreterTool),
    FileSearchTool(FileSearchTool),
}

#[derive(Builder, Clone, Debug, Serialize)]
pub struct CodeInterpreterTool {
    #[builder(into)]
    pub r#type: String,
}

#[derive(Builder, Clone, Debug, Serialize)]
pub struct FileSearchTool {
    #[builder(into)]
    pub r#type: String,
}

/// Represents a thread that contains [messages](https://platform.openai.com/docs/api-reference/messages).
#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ThreadResponse {
    pub id: String,
    pub object: String,
    pub created_at: u64,
    pub metadata: HashMap<String, serde_json::Value>,
    pub tool_resources: Option<ToolResources>,
}

#[derive(Builder, Clone, Debug, Serialize)]
pub struct ModifyThreadRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_resources: Option<ToolResources>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct DeleteThreadResponse {
    pub id: String,
    pub object: String,
    pub delete: bool,
}
```

- [ ] **Step 6: Write the `threads` tests**

Append to `rs_openai/src/apis/threads.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::OpenAI;
    use crate::interfaces::threads::CreateThreadRequest;
    use crate::shared::response_wrapper::OpenAIError;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn create_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/threads"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "thread_123", "object": "thread", "created_at": 1700000000_u64,
                "metadata": {}, "tool_resources": null
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = CreateThreadRequest::builder().build();

        let res = client.threads().create(&req).await.unwrap();
        assert_eq!(res.id, "thread_123");
    }

    #[tokio::test]
    async fn delete_maps_api_error() {
        let mock_server = MockServer::start().await;

        Mock::given(method("DELETE"))
            .and(path("/threads/thread_missing"))
            .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
                "error": {"message": "No thread found", "type": "invalid_request_error", "param": null, "code": null}
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let err = client.threads().delete("thread_missing").await.unwrap_err();
        assert!(matches!(err, OpenAIError::ApiError(_)));
    }
}
```

- [ ] **Step 7: Migrate `runs.rs` to `bon`**

Replace `rs_openai/src/interfaces/runs.rs` in full (the same `Message`/`Content`/.../`FileSearchTool` duplication as `threads.rs`, independently copy-pasted here too):

```rust
use super::assistants::ToolResources;
use bon::Builder;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Builder, Clone, Debug, Serialize)]
pub struct CreateRunRequest {
    /// Additional fields to include in the response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub includes: Option<Vec<String>>,
}

#[derive(Builder, Clone, Debug, Serialize)]
pub struct Message {
    #[builder(into)]
    pub role: String,
    pub content: Content,
    pub attachments: Option<Vec<Attachment>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Serialize, Clone, strum::Display)]
pub enum Content {
    TextContent(String),
    ArrayOfContentParts(Vec<ArrayOfContentParts>),
}

#[derive(Debug, Serialize, Clone, strum::Display)]
pub enum ArrayOfContentParts {
    ImageFileType(ImageFileType),
    ImageUrlType(ImageUrlType),
    TextType(TextType),
}

#[derive(Builder, Clone, Debug, Serialize)]
pub struct ImageFileType {
    #[builder(into)]
    pub r#type: String,
    pub image_file: ImageFile,
}

#[derive(Builder, Clone, Debug, Serialize)]
pub struct ImageFile {
    #[builder(into)]
    pub file_id: String,
    #[builder(into)]
    pub detail: Option<String>,
}

#[derive(Builder, Clone, Debug, Serialize)]
pub struct ImageUrlType {
    #[builder(into)]
    pub r#type: String,
    pub image_url: ImageUrl,
}

#[derive(Builder, Clone, Debug, Serialize)]
pub struct ImageUrl {
    #[builder(into)]
    pub url: String,
    #[builder(into)]
    pub detail: Option<String>,
}

#[derive(Builder, Clone, Debug, Serialize)]
pub struct TextType {
    #[builder(into)]
    pub r#type: String,
    #[builder(into)]
    pub text: String,
}

#[derive(Builder, Clone, Debug, Serialize)]
pub struct Attachment {
    #[builder(into)]
    pub file_id: Option<String>,
    pub tools: Option<Vec<Tool>>,
}

#[derive(Debug, Serialize, Clone, strum::Display)]
pub enum Tool {
    CodeInterpreterTool(CodeInterpreterTool),
    FileSearchTool(FileSearchTool),
}

#[derive(Builder, Clone, Debug, Serialize)]
pub struct CodeInterpreterTool {
    #[builder(into)]
    pub r#type: String,
}

#[derive(Builder, Clone, Debug, Serialize)]
pub struct FileSearchTool {
    #[builder(into)]
    pub r#type: String,
}

/// Represents a thread that contains [messages](https://platform.openai.com/docs/api-reference/messages).
#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct RunResponse {
    pub id: String,
    pub object: String,
    pub created_at: u64,
    pub metadata: HashMap<String, serde_json::Value>,
    pub tool_resources: Option<ToolResources>,
}

#[derive(Builder, Clone, Debug, Serialize)]
pub struct ModifyRunRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_resources: Option<ToolResources>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct DeleteRunResponse {
    pub id: String,
    pub object: String,
    pub delete: bool,
}
```

- [ ] **Step 8: Write the `runs` tests**

Append to `rs_openai/src/apis/runs.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::OpenAI;
    use crate::interfaces::runs::CreateRunRequest;
    use crate::shared::response_wrapper::OpenAIError;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn create_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/threads/thread_123/runs"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "run_123", "object": "thread.run", "created_at": 1700000000_u64,
                "metadata": {}, "tool_resources": null
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = CreateRunRequest::builder().build();

        let res = client.runs().create("thread_123", &req).await.unwrap();
        assert_eq!(res.id, "run_123");
    }

    #[tokio::test]
    async fn delete_maps_api_error() {
        let mock_server = MockServer::start().await;

        Mock::given(method("DELETE"))
            .and(path("/runs/run_missing"))
            .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
                "error": {"message": "No run found", "type": "invalid_request_error", "param": null, "code": null}
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let err = client.runs().delete("run_missing").await.unwrap_err();
        assert!(matches!(err, OpenAIError::ApiError(_)));
    }
}
```

- [ ] **Step 9: Run all four modules' tests, then the whole workspace**

Run: `cargo test --package rs_openai apis::assistants::tests apis::messages::tests apis::threads::tests apis::runs::tests`
Expected: 8 tests pass.

Run: `cargo build --workspace --all-targets && cargo test --workspace`
Expected: clean build, all tests pass. This is the last of the 15 `derive_builder` files migrated — after this task, `grep -rl "derive(Builder" rs_openai/src` should show zero remaining matches under `interfaces/`.

- [ ] **Step 10: Commit**

```bash
git add rs_openai/src/interfaces/assistants.rs rs_openai/src/interfaces/messages.rs \
        rs_openai/src/interfaces/threads.rs rs_openai/src/interfaces/runs.rs \
        rs_openai/src/apis/assistants.rs rs_openai/src/apis/messages.rs \
        rs_openai/src/apis/threads.rs rs_openai/src/apis/runs.rs
git commit -m "refactor!: migrate assistants/messages/threads/runs builders to bon

Last of the 15 derive_builder request types. These four modules
implement the Assistants API, which OpenAI is sunsetting 2026-08-26;
Phase 2 will deprecate or remove them in favor of Responses +
Conversations, but they get the same Phase 1 treatment as everything
else in the crate until then."
```

---

### Task 13: Simplify `shared/macro.rs`'s "string or array" conversions

**Files:**
- Modify: `rs_openai/src/shared/macro.rs`

**Interfaces:**
- Consumes: nothing from earlier tasks (independent of the bon migration — `derive_builder`/`bon` don't touch these `From` impls either way).
- Produces: the same `Into`-based ergonomics every earlier task's example code already relies on (`.input(["a", "b"])`, `.stop("STOP")`, etc.) — this task changes only how those conversions are implemented, not their call-site behavior.

`Stop` and `Prompt`/`EmbeddingInput`/`ModerationInput` are already `#[serde(untagged)]` enums (defined in `shared/types.rs` and their respective `interfaces/*.rs` files — Tasks 5–12 didn't touch these enum definitions, only the request structs around them). What's actually oversized is `shared/macro.rs`: three `macro_rules!` definitions that hand-generate `From` impls for every array-size/const-generic combination — including `&[T; N]` and `Vec<[T; N]>` variants that nothing in this codebase's examples or tests ever calls. Confirmed via `grep -rn "\.input(&\[" rs_openai examples` and similar: zero call sites pass array *references*, only owned arrays and `Vec`s.

- [ ] **Step 1: Replace `shared/macro.rs` in full**

```rust
use crate::interfaces::completions::Prompt;
use crate::interfaces::embeddings::EmbeddingInput;
use crate::interfaces::moderations::ModerationInput;
use crate::shared::types::Stop;

macro_rules! impl_string_like_from {
    ($to_typ:ty, $string_variant:ident, $array_variant:ident) => {
        impl<S: Into<String>> From<S> for $to_typ {
            fn from(value: S) -> Self {
                Self::$string_variant(value.into())
            }
        }

        impl<S: Into<String>> From<Vec<S>> for $to_typ {
            fn from(value: Vec<S>) -> Self {
                Self::$array_variant(value.into_iter().map(Into::into).collect())
            }
        }

        impl<S: Into<String>, const N: usize> From<[S; N]> for $to_typ {
            fn from(value: [S; N]) -> Self {
                Self::$array_variant(value.into_iter().map(Into::into).collect())
            }
        }
    };
}

impl_string_like_from!(Stop, String, ArrayOfString);
impl_string_like_from!(Prompt, String, ArrayOfString);
impl_string_like_from!(EmbeddingInput, String, ArrayOfString);
impl_string_like_from!(ModerationInput, String, ArrayOfString);

impl Default for ModerationInput {
    fn default() -> Self {
        ModerationInput::String(String::new())
    }
}

impl Default for EmbeddingInput {
    fn default() -> Self {
        EmbeddingInput::String(String::new())
    }
}

macro_rules! impl_token_array_from {
    ($to_typ:ty) => {
        impl From<Vec<u16>> for $to_typ {
            fn from(value: Vec<u16>) -> Self {
                Self::ArrayOfTokens(value)
            }
        }

        impl From<Vec<Vec<u16>>> for $to_typ {
            fn from(value: Vec<Vec<u16>>) -> Self {
                Self::ArrayOfTokenArrays(value)
            }
        }
    };
}

impl_token_array_from!(Prompt);
impl_token_array_from!(EmbeddingInput);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stop_from_str_serializes_as_string() {
        let stop: Stop = "STOP".into();
        assert_eq!(serde_json::to_value(&stop).unwrap(), serde_json::json!("STOP"));
    }

    #[test]
    fn stop_from_array_serializes_as_array() {
        let stop: Stop = ["STOP1", "STOP2"].into();
        assert_eq!(
            serde_json::to_value(&stop).unwrap(),
            serde_json::json!(["STOP1", "STOP2"])
        );
    }

    #[test]
    fn prompt_from_token_vec_serializes_as_array_of_numbers() {
        let prompt: Prompt = vec![1_u16, 2, 3].into();
        assert_eq!(serde_json::to_value(&prompt).unwrap(), serde_json::json!([1, 2, 3]));
    }

    #[test]
    fn prompt_from_token_array_arrays_serializes_correctly() {
        let prompt: Prompt = vec![vec![1_u16, 2], vec![3_u16, 4]].into();
        assert_eq!(
            serde_json::to_value(&prompt).unwrap(),
            serde_json::json!([[1, 2], [3, 4]])
        );
    }

    #[test]
    fn embedding_input_from_array_of_str_serializes_as_array() {
        let input: EmbeddingInput = ["hello", "world"].into();
        assert_eq!(
            serde_json::to_value(&input).unwrap(),
            serde_json::json!(["hello", "world"])
        );
    }

    #[test]
    fn moderation_input_default_serializes_as_empty_string() {
        assert_eq!(
            serde_json::to_value(ModerationInput::default()).unwrap(),
            serde_json::json!("")
        );
    }
}
```

This collapses the file from ~200 lines of macro-generated const-generic impls down to ~110 lines including the new tests, with identical call-site ergonomics: every `.input([...])`, `.input("...")`, `.stop("...")`, `.prompt(vec![1u16, 2, 3])` call already written in Tasks 5–12's examples and tests continues to work unchanged.

- [ ] **Step 2: Run the tests**

Run: `cargo test --package rs_openai shared::macro::tests`
Expected: 6 tests pass.

- [ ] **Step 3: Run the whole workspace**

Run: `cargo build --workspace --all-targets && cargo test --workspace`
Expected: clean build, every test written across Tasks 3–13 passes.

- [ ] **Step 4: Commit**

```bash
git add rs_openai/src/shared/macro.rs
git commit -m "refactor: simplify string-or-array From impls in shared/macro.rs

Replaces per-array-size const-generic macro expansion (including
&[T; N] and Vec<[T; N]> forms nothing in this codebase ever calls)
with a handful of blanket generic impls. Same call-site ergonomics,
~90 fewer lines, no more macro_rules gymnastics to maintain."
```

---

### Task 14: Remove `derive_builder`, update the README, commit `Cargo.lock`, final verification

**Files:**
- Modify: `rs_openai/Cargo.toml`
- Modify: `README.md`
- Modify: `.gitignore`
- Create: `Cargo.lock` (un-ignored and committed for the first time)

**Interfaces:**
- Consumes: every earlier task — this is the closing task that confirms the whole Phase 1 scope actually holds together as one buildable, tested, lint-clean workspace.

- [ ] **Step 1: Remove `derive_builder`**

Run: `grep -rl "derive_builder\|derive(Builder" rs_openai/src`
Expected: no output (Tasks 5–12 migrated all 15 usages to `bon`).

```bash
cargo remove derive_builder
```

- [ ] **Step 2: Un-ignore and commit `Cargo.lock`**

The workspace now contains 13 binary crates (12 examples + the actix one) in addition to the `rs_openai` library; per Cargo's own guidance (the comment already in `.gitignore`: "Remove Cargo.lock from gitignore if creating an executable, leave it for libraries"), a workspace with binaries should commit its lock file for reproducible builds, even though the library crate alone wouldn't need to. Edit `.gitignore`:

```
# Generated by Cargo
# will have compiled files and executables
target

# These are backup files generated by rustfmt
**/*.rs.bk

# Env files
.env
```

(The `Cargo.lock` line is removed entirely; the explanatory comment above it goes with it since it no longer applies.)

- [ ] **Step 3: Update the README's usage examples for the new client/builder API**

In `README.md`, bump the version in the Installation block:

```toml
[dependencies]
rs_openai = { version = "0.6.0" }
```

Check off the "Enhances backoff" item in the Features list (now done via Task 3's `reqwest-middleware`/`reqwest-retry`):

```markdown
- [x] Enhances backoff
- [ ] Supports Microsoft Azure Endpoints
```

Replace the first usage example:

```rust
use dotenvy::dotenv;
use rs_openai::{
    chat::{ChatCompletionMessage, CreateChatRequest, Role},
    OpenAI,
};
use std::env::var;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    let api_key = var("OPENAI_API_KEY").unwrap();

    let client = OpenAI::new(api_key);

    let req = CreateChatRequest::builder()
        .model("gpt-3.5-turbo")
        .messages(vec![ChatCompletionMessage::builder()
            .role(Role::User)
            .content("To Solve LeetCode's problem 81 in Rust.")
            .build()])
        .build();

    let res = client.chat().create(&req).await?;
    println!("{:?}", res);

    Ok(())
}
```

Replace the Stream section's example:

```rust
use dotenvy::dotenv;
use futures::StreamExt;
use rs_openai::{
    chat::{ChatCompletionMessage, CreateChatRequest, Role},
    OpenAI,
};
use std::env::var;
use std::io::{stdout, Write};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    let api_key = var("OPENAI_API_KEY").unwrap();

    let client = OpenAI::new(api_key);

    // stream mode
    let req = CreateChatRequest::builder()
        .model("gpt-3.5-turbo")
        .messages(vec![ChatCompletionMessage::builder()
            .role(Role::User)
            .content("To Solve LeetCode's problem 81 in Rust.")
            .build()])
        .stream(true)
        .build();

    let mut stream = client.chat().create_with_stream(&req).await?;

    let mut lock = stdout().lock();
    while let Some(response) = stream.next().await {
        response.unwrap().choices.iter().for_each(|choice| {
            if let Some(ref content) = choice.delta.content {
                write!(lock, "{}", content).unwrap();
            }
        });

        stdout().flush()?;
    }

    Ok(())
}
```

Everything else in the README (badges, Contributing/Code of Conduct sections, Thanks, License) is unchanged.

- [ ] **Step 4: Update `rs_openai/src/lib.rs`'s doc-comment examples to match**

The two ` ```ignore ` blocks in `lib.rs` (the "Creating client" and "Making requests" examples) use the same old `OpenAI::new(&OpenAI { .. })` / builder pattern as the README. Update both to the constructor and builder shape from Step 3 (they stay `ignore`d, not `doctest`-run, but should still describe the real API):

```rust
//! ## Creating client
//!
//! ```ignore
//! use dotenvy::dotenv;
//! use rs_openai::OpenAI;
//! use std::env::var;
//!
//! dotenv().ok();
//! let api_key = var("OPENAI_API_KEY").unwrap();
//!
//! let client = OpenAI::new(api_key);
//! ```
//!
//! ## Making requests
//!
//!```ignore
//! use dotenvy::dotenv;
//! use rs_openai::{
//!     chat::{ChatCompletionMessage, CreateChatRequest, Role},
//!     OpenAI,
//! };
//! use std::env::var;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     dotenv().ok();
//!     let api_key = var("OPENAI_API_KEY").unwrap();
//!
//!     let client = OpenAI::new(api_key);
//!
//!     let req = CreateChatRequest::builder()
//!         .model("gpt-3.5-turbo")
//!         .messages(vec![ChatCompletionMessage::builder()
//!             .role(Role::User)
//!             .content("To Solve LeetCode's problem 81 in Rust.")
//!             .build()])
//!         .build();
//!
//!     let res = client.chat().create(&req).await?;
//!     println!("{:?}", res);
//!
//!     Ok(())
//! }
//!```
```

- [ ] **Step 5: Full workspace verification**

Run: `cargo fmt --all -- --check`
Expected: passes (run `cargo fmt --all` first if not, and fold the formatting diff into this commit).

Run: `cargo clippy --workspace --all-targets --all-features -- -D warnings`
Expected: passes with zero warnings — this is the first time in this plan every fix from Tasks 1–13 is checked against the CI lint level all at once.

Run: `cargo build --workspace --all-targets`
Expected: builds clean.

Run: `cargo test --workspace`
Expected: every test from Tasks 3–13 passes.

Run: `cargo doc --no-deps -p rs_openai`
Expected: builds without warnings (the crate's docs.rs badge implies this should always be clean).

Run: `git add Cargo.lock && git status`
Expected: `Cargo.lock` now tracked.

- [ ] **Step 6: Commit**

```bash
git add rs_openai/Cargo.toml README.md .gitignore Cargo.lock rs_openai/src/lib.rs
git commit -m "chore: remove derive_builder, commit Cargo.lock, update README for bon/new client API

Phase 1 complete: workspace merged and building for the first time,
apis::* re-exports interfaces::* again, three real bugs fixed (Files
method name, ImageSize serde rename, audio 'laguage' form field typo),
HTTP client reused with retry middleware, panics replaced with
propagated errors, all 15 request builders on bon, shared/macro.rs
simplified, and a wiremock test suite covering every API module."
```

---

## Phase 1 complete

At this point the crate has: a single buildable workspace (the first time in this repo's history every example crate has actually been compiled by anything), CI that actually gates on `fmt`/`clippy`/`test`, a reused+retried HTTP client with no panics on the request path, `bon`-based builders with compile-time-enforced required fields, a simplified `shared/macro.rs`, and wiremock test coverage for every API module. Along the way, building `examples/` for the first time surfaced five real, previously-invisible bugs, all fixed: the missing `apis::* → interfaces::*` re-export that broke the README's own usage example (Task 1), `Files::add_upload` actually being a delete method (Task 1), six API modules (`assistants`, `batch`, `messages`, `runs`, `threads`, `uploads`) having no `client.xxx()` accessor at all (Task 3), `ImageSize::S512x512`/`S1024x1024` both serializing as `"256x256"` in JSON request bodies (Task 9), and the `"laguage"` multipart field typo silently dropping the transcription language hint (Task 10). Phase 2 (the OpenAI API surface update against https://developers.openai.com/api/docs — Responses API, Conversations API, deprecating the Assistants-API modules, removing `edits`/`engines`, and a decision on the Realtime API) gets its own spec and plan once this one is implemented and merged.
