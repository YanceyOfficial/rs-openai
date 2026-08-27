# Containers: Skill Attachment + Raw File Upload — Design

Status: Approved
Date: 2026-08-28

## Context

Closes both of Containers' remaining known gaps (listed explicitly in the README's "Known gaps" section after the Skills API slice). Now that Skills exists as its own resource, `skill_reference` can be modeled precisely. Confirmed via `gh api` against `openai-python`'s `types/container_create_params.py`, `types/responses/{skill_reference_param,inline_skill_param,inline_skill_source_param}.py`, and `resources/containers/files/files.py`/`types/containers/file_create_{response,params}.py`.

## Goals

### Skill attachment on container creation

- `ContainerSkill` — a `type`-tagged 2-variant enum, matching this crate's established tagged-union pattern (`ResponseTool`, `ChunkingStrategy`, etc.): `SkillReference { skill_id: String, version: Option<String> }` (tag `skill_reference`) | `Inline { description: String, name: String, source: InlineSkillSource }` (tag `inline`).
- `InlineSkillSource { data: String, media_type: String }` — `data` is a base64-encoded zip bundle; `media_type` is always `"application/zip"` per source, modeled as a plain settable `String` field (matching this crate's convention for currently-single-valued-but-technically-open fields, e.g. `SpendAlert`'s `currency`/`interval`) rather than hardcoding it, since no other field in this crate hides a "constant" value from the caller.
- `CreateContainerRequest.skills: Option<Vec<ContainerSkill>>` — confirmed via source (`skills: Iterable[Skill]`).

### Raw file upload into a container

- `CreateContainerFileRequest` changes from `{ file_id: String }` (its only prior shape) to wrap a new `ContainerFileSource` enum: `FileId(String)` | `Upload(File)` (reusing the crate's existing `shared::types::File`) — confirmed via source (`file: FileTypes | Omit`, `file_id: str | Omit`, mutually exclusive, the SDK picks JSON-body vs. multipart based on which was provided).
- `ContainerFiles::create()` branches on the enum: `FileId` sends the existing plain JSON POST unchanged; `Upload` builds a `reqwest::multipart::Form` with one `file` part (mime `application/octet-stream`, matching this crate's established convention for arbitrary uploaded files) and calls `post_form` — mirroring the exact branching structure Skills' `create()` already established for its own zip/directory choice, applied here to a JSON/multipart choice instead. Response type (`ContainerFile`) is unchanged -- confirmed identical shape to the real API's `FileCreateResponse`.

## Non-goals

None -- this closes both items in the README's "Known gaps" list for Containers; no further Containers work is anticipated after this slice.

## Design notes

All new types added to `interfaces/containers.rs`. `apis/containers/mod.rs`'s `create()` picks up `skills` automatically (already generic over the full request struct). `apis/containers/files.rs`'s `create()` gets the new branching logic; no `client.rs` changes (reuses `post`/`post_form` as-is).

## Testing

4 new tests: `create` (container) serializes a `skill_reference` skill; `create` (container) serializes an `inline` skill; `Files::create` sends a plain JSON body for `FileId`; `Files::create` sends a multipart `file` part for `Upload`.

## Versioning

One deliberate breaking change (`CreateContainerFileRequest`'s shape, `file_id: String` → wrapping `ContainerFileSource`), otherwise additive.
