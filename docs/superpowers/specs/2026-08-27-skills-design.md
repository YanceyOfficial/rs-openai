# Skills API — Design

Status: Approved
Date: 2026-08-27

## Context

A genuinely new top-level resource category for `rs_openai`, never scaffolded before (explicitly deferred during the Containers slice as its own future undertaking). A skill is a versioned, reusable bundle of files (instructions + scripts + assets) anchored by a `SKILL.md` manifest. Researched via the OpenAI docs guide (`developers.openai.com/api/docs/guides/tools-skills`) cross-checked against `gh api` reads of `openai-python`'s `resources/skills/` and `types/skill*.py`/`types/skills/*.py` source (the reference pages at `/api/reference/resources/skills/...` 404'd for direct fetch; the guide page and SDK source together gave a complete, field-level picture).

## Goals

### Types (`interfaces/skills.rs`, flat file)

- `Skill { id: String, created_at: i64, default_version: String, description: String, latest_version: String, name: String, object: String }`
- `DeletedSkill { id: String, deleted: bool, object: String }`
- `SkillVersion { id: String, created_at: i64, description: String, name: String, object: String, skill_id: String, version: String }`
- `DeletedSkillVersion { id: String, deleted: bool, object: String, version: String }`
- `SkillFiles` — an enum reused by both `create` endpoints: `Zip(File)` (a single zip bundle) | `Directory(Vec<File>)` (multiple files, each `File.filename` set to its full relative path within the skill's top-level folder, e.g. `my-skill/SKILL.md`). Reuses the crate's existing `shared::types::File { buffer: Vec<u8>, filename: String }` directly — no new file-payload type needed.
- `CreateSkillRequest { files: SkillFiles }`
- `UpdateSkillRequest { default_version: String }` — the only mutable field (confirmed via source: "Update the default version pointer for a skill").
- `ListSkillsRequest { after, limit, order }` / `ListSkillsResponse { object, data: Vec<Skill>, first_id, last_id, has_more }` — standard cursor pagination, matching every other list endpoint in this crate.
- `CreateSkillVersionRequest { default: Option<bool>, files: SkillFiles }`
- `ListSkillVersionsRequest { after, limit, order }` / `ListSkillVersionsResponse { object, data: Vec<SkillVersion>, first_id, last_id, has_more }`

### Multipart upload wiring

Both `create` endpoints accept `files: Union[SequenceNotStr[FileTypes], FileTypes]` — confirmed via source and the guide's curl examples:
- **Zip upload**: one multipart part named `files` (no brackets), mime `application/zip` — `-F 'files=@./skill.zip;type=application/zip'`.
- **Directory upload**: repeated multipart parts named `files[]`, each with its own `filename` set to the relative path — `-F 'files[]=@./path/SKILL.md;filename=folder/SKILL.md;type=text/markdown'`.

`SkillFiles::Zip` → one `form.part("files", ...)` call with mime `application/zip`. `SkillFiles::Directory` → loop building `form.part("files[]", ...)` per file with mime `application/octet-stream` (arbitrary file types; guessing per-extension MIME is out of scope, matching this crate's existing convention for `Files::upload`/`Images::create_edit`). This mirrors `Images::create_edit`'s already-proven `image[]` repeated-part pattern exactly — no `client.rs` changes needed, reusing the existing `post_form` method.

### Endpoints (`apis/skills/mod.rs` + `apis/skills/versions.rs`)

- `Skills::create(&CreateSkillRequest) -> Skill`, `Skills::list(&ListSkillsRequest) -> ListSkillsResponse`
- `Skill::retrieve() -> Skill`, `Skill::update(&UpdateSkillRequest) -> Skill`, `Skill::delete() -> DeletedSkill`, `Skill::content() -> Vec<u8>` (binary zip download via the existing `get_raw_bytes`)
- `Skill::versions() -> SkillVersions` (collection accessor) / `Skill::version(id) -> SkillVersion` (ID-scoped accessor) — matching the established `.file_batches()`/`.file_batch(id)` precedent from Vector Stores.
- `SkillVersions::create(&CreateSkillVersionRequest) -> SkillVersion`, `SkillVersions::list(&ListSkillVersionsRequest) -> ListSkillVersionsResponse`
- `SkillVersion::retrieve() -> SkillVersion`, `SkillVersion::delete() -> DeletedSkillVersion`, `SkillVersion::content() -> Vec<u8>` — confirmed no update/list-modification beyond this; versions are immutable once created.

Top-level `client.skills()`/`client.skill(id)` accessors added to `client.rs`, matching the `containers()`/`container(id)` and `videos()`/`video(id)` pattern exactly.

### Naming collision (expected, established precedent)

`apis::skills::Skill<'a>` (the ID-scoped accessor struct) collides with `interfaces::skills::Skill` (the data type) once `apis/skills/mod.rs` re-exports `crate::interfaces::skills::*` for consumer ergonomics — resolved via qualified paths (`skills::Skill` for the interface type) in method signatures, exactly matching the precedent already established for `apis::responses::Response<'a>` vs `interfaces::responses::Response`. `apis/skills/versions.rs` avoids the analogous `SkillVersion` collision by not doing its own glob re-export (mirroring `apis::vector_stores::files`'s approach).

## Non-goals

- Any first-party-skill-reference behavior (`skill_reference` objects referencing OpenAI-maintained skills like `openai-spreadsheets`) — that's a Responses/tool-configuration concern, not a Skills-resource CRUD concern, and out of scope for this slice.
- Per-file-extension MIME type inference for directory uploads — matches this crate's existing convention of a fixed `application/octet-stream` for arbitrary uploaded files.
- Client-side validation of the documented constraints (max 50MB zip, max 500 files, max 25MB uncompressed, exactly one `SKILL.md`) — matches this crate's standing convention of not re-implementing server-side validation.

## Testing

~13 `wiremock` tests: `create` with zip upload, `create` with directory upload (proving repeated `files[]` parts each carry distinct bytes, mirroring `Images::create_edit`'s existing test pattern), `retrieve`, `update`, `list`, `delete`, `content` download for `Skill`; `create` (with `default: true`), `retrieve`, `list`, `delete`, `content` download for `SkillVersion`.

## Versioning

Purely additive — a wholly new resource category, no existing types touched.
