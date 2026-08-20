# Organization/Admin: Projects + 5 nested sub-resources (sub-slice 4 of 4) — Design

Status: Approved
Date: 2026-08-20

## Context

Fourth and final sub-slice for Organization/admin (see sub-slices 1-3's specs for prior context — all shipped and merged). Completing this ships Phase 2a's full original 9-scaffold list.

**Research method**: same `gh api repos/openai/openai-python/contents/...` technique as every prior sub-slice.

**Scope confirmed via `AskUserQuestion`**: current research found Projects actually has 11 potential sub-resources in OpenAI's live API today (`api_keys`, `certificates`, `data_retention`, `groups`, `hosted_tool_permissions`, `model_permissions`, `rate_limits`, `roles`, `service_accounts`, `spend_alerts`, `spend_limit`, `users`) — 6 more than the 5 Phase 2a originally scaffolded (`api_keys`, `groups`, `rate_limits`, `service_accounts`, `users`). Per the original 4-sub-slice decomposition's own commitment, **this slice covers exactly the originally-scaffolded 5** (plus Projects CRUD itself). The 6 newer resources (`certificates`/`data_retention`/`hosted_tool_permissions`/`model_permissions`/`spend_alerts`/`spend_limit`) are unscaffolded, unresearched-beyond-a-directory-listing, and deferred entirely to a genuinely future addition — not silently folded in.

**A deeper-nesting discovery within the confirmed 5**: `service_accounts` itself has its own `api_keys` sub-resource (`POST /organization/projects/{id}/service_accounts/{id}/api_keys`, create-only) — not anticipated by the flat Phase 2a scaffold (`apis/organization/projects/service_accounts.rs` is a single file, no sub-directory). This is folded into this slice (not deferred) since it's a small, single-method extension of an already-in-scope resource — matching the established precedent of `OrganizationGroupUser`/`ContainerFile` being added beyond a flat scaffold when the real API needs one level deeper.

**Roles deferred everywhere, again**: `groups/roles.py` and `users/roles.py` sub-resources exist under Projects too (project-scoped role assignment) — deferred, matching org-level Roles/Users.Roles/Groups.Roles already deferred in sub-slice 2.

## Goals

### Projects CRUD (top-level, no sub-resource)

- `OrganizationProjects::create()` (`POST /organization/projects`), `::list()` (`GET /organization/projects`), `OrganizationProject::retrieve()` (`GET /organization/projects/{id}`), `::update()` (`POST /organization/projects/{id}`), `::archive()` (`POST /organization/projects/{id}/archive`) — **no `delete`; `archive` is the only lifecycle-ending operation**, confirmed via source (no `_delete` call anywhere in `projects.py`).
- `CreateProjectRequest`: `name: String` (required), `external_key_id: Option<String>`, `geography: Option<String>`.
- `UpdateProjectRequest`: `external_key_id: Option<String>`, `geography: Option<String>`, `name: Option<String>` (all optional — normal `Update*Request` convention here, unlike Groups' required-`name` exception in sub-slice 2).
- `Project` (response, used uniformly for create/retrieve/update/list): `id: String`, `created_at: i64`, `object: String`, `archived_at: Option<i64>`, `external_key_id: Option<String>`, `name: Option<String>`, `status: Option<String>` (`"active"`/`"archived"`, plain string).
- `ListProjectsRequest`: `after: Option<String>`, `include_archived: Option<bool>`, `limit: Option<u32>`.
- `ListProjectsResponse`: universal list shape, `data: Vec<Project>`.
- `archive()` returns the same `Project` type (confirmed: `_post` with `cast_to` matching the resource's own retrieve type in the source pattern used elsewhere — treated as returning `Project` with `status: "archived"`/`archived_at` populated, consistent with every other archive-style operation in REST APIs of this shape; not independently re-verified byte-for-byte since the source snippet didn't show its literal `cast_to`, flagged here for the implementer to confirm against a live response if uncertain, but modeled as `Project` since no separate `ArchiveProjectResponse` type file exists anywhere in the fetched source tree).

### Projects.ApiKeys

- `OrganizationProjectApiKeys::list()` (`GET /organization/projects/{id}/api_keys`), `OrganizationProjectApiKey::retrieve()` (`GET .../api_keys/{key_id}`), `::delete()` (`DELETE .../api_keys/{key_id}`) — **no `create`**, confirmed (project API keys are created by project members through the regular product UI, not admin-created).
- `ProjectApiKeyOwnerServiceAccount`: `id: String`, `created_at: i64`, `name: String`, `role: String`.
- `ProjectApiKeyOwnerUser`: `id: String`, `created_at: i64`, `email: String`, `name: String`, `role: String`.
- `ProjectApiKeyOwner`: `service_account: Option<ProjectApiKeyOwnerServiceAccount>`, `kind: Option<String>` (`#[serde(rename = "type")]`, `"user"`/`"service_account"`), `user: Option<ProjectApiKeyOwnerUser>`.
- `ProjectApiKey`: `id: String`, `created_at: i64`, `last_used_at: Option<i64>`, `name: String`, `object: String`, `owner: ProjectApiKeyOwner` (required), `owner_project_access: String` (`"active"`/`"inactive"`), `redacted_value: String`.
- `ListProjectApiKeysRequest`: `after: Option<String>`, `limit: Option<u32>`, `owner_project_access: Option<String>` (`"active"`/`"inactive"`/`"any"`).
- `ListProjectApiKeysResponse`/`DeleteProjectApiKeyResponse` (`{id, deleted, object}`).

### Projects.Groups

- `OrganizationProjectGroups::create()` (`POST .../groups`), `::list()` (`GET .../groups`), `OrganizationProjectGroup::retrieve()` (`GET .../groups/{group_id}`), `::delete()` (`DELETE .../groups/{group_id}`) — **no `update`** (a group's project role/membership is either granted or revoked, not partially edited — confirmed, no update params file exists).
- `AddProjectGroupRequest`: `group_id: String` (required), `role: String` (required — the project role to grant, e.g. `"owner"`/`"member"`, kept plain string).
- `ProjectGroup`: `created_at: i64`, `group_id: String`, `group_name: String`, `group_type: String` (`"group"`/`"tenant_group"`), `object: String`, `project_id: String`.
- `ListProjectGroupsRequest`: `after: Option<String>`, `limit: Option<u32>`, `order: Option<crate::interfaces::chat::SortOrder>`.
- `ListProjectGroupsResponse`: universal list shape.
- `RemoveProjectGroupResponse`: `deleted: bool`, `object: String` — **no `id`**, matching the exact asymmetry already established for org-level `RemoveGroupUserResponse` in sub-slice 2 (membership-type deletes in this crate's confirmed API surface consistently lack `id`).

### Projects.RateLimits

- `OrganizationProjectRateLimits::list()` (`GET .../rate_limits`), `OrganizationProjectRateLimit::update()` (`POST .../rate_limits/{rate_limit_id}`) — **no `create`/`delete`** (rate limits are pre-existing per-model configs, only ever updatable, confirmed: only `list_rate_limits`/`update_rate_limit` methods exist in source).
- `ProjectRateLimit`: `id: String`, `max_requests_per_1_minute: i64`, `max_tokens_per_1_minute: i64`, `model: String`, `object: String`, `batch_1_day_max_input_tokens: Option<i64>`, `max_audio_megabytes_per_1_minute: Option<i64>`, `max_images_per_1_minute: Option<i64>`, `max_requests_per_1_day: Option<i64>` (the four `Option`al fields are confirmed "only present for relevant models" — genuinely optional per-model, not a defensive widening).
- `ListProjectRateLimitsRequest`: `after: Option<String>`, `before: Option<String>`, `limit: Option<u32>`.
- `ListProjectRateLimitsResponse`: universal list shape, `data: Vec<ProjectRateLimit>`.
- `UpdateProjectRateLimitRequest`: all 6 numeric fields optional (`batch_1_day_max_input_tokens`, `max_audio_megabytes_per_1_minute`, `max_images_per_1_minute`, `max_requests_per_1_day`, `max_requests_per_1_minute`, `max_tokens_per_1_minute` — all `Option<i64>`; the source's own `project_id: Required[str]` is a path parameter already carried by the `OrganizationProjectRateLimit` accessor's stored `project_id`/`rate_limit_id`, not a request-body field in this crate's modeling, matching every other ID-scoped accessor's convention).
- `update()` returns `ProjectRateLimit` (confirmed: same shape, this endpoint is a partial-update returning the full updated resource, standard PATCH-via-POST pattern already used throughout this crate).

### Projects.ServiceAccounts (+ nested ApiKeys)

- `OrganizationProjectServiceAccounts::create()` (`POST .../service_accounts`), `::list()` (`GET .../service_accounts`), `OrganizationProjectServiceAccount::retrieve()` (`GET .../service_accounts/{id}`), `::update()` (`POST .../service_accounts/{id}`), `::delete()` (`DELETE .../service_accounts/{id}`).
- `CreateProjectServiceAccountRequest`: `name: String` (required), `create_service_account_only: Option<bool>`.
- `ProjectServiceAccount` (response for retrieve/update/list): `id: String`, `created_at: i64`, `name: String`, `object: String`, `role: String` (`"owner"`/`"member"`/`"none"`).
- `CreateProjectServiceAccountApiKey` (embedded, create-response-only): `id: String`, `created_at: i64`, `name: String`, `object: String`, `value: String`.
- `CreateProjectServiceAccountResponse` (response for create ONLY — genuinely different shape than `ProjectServiceAccount`, confirmed via source: has an extra `api_key: Option<CreateProjectServiceAccountApiKey>` field, and its `role` field is a NARROWER enum `"member"`/`"none"` (no `"owner"` — a newly-created service account can never start as owner) vs. the base type's `"owner"`/`"member"`/`"none"`. **A real, confirmed request/response asymmetry, kept as two separate types** per this crate's standing rule.
- `UpdateProjectServiceAccountRequest`: `name: Option<String>`, `role: Option<String>` (`"member"`/`"owner"` — confirmed different valid-value set than the base `role` field's 3 values, still kept plain `String` on both sides per this crate's "don't enum-close without a concrete reason" convention).
- `ListProjectServiceAccountsRequest`: `after: Option<String>`, `limit: Option<u32>`.
- `ListProjectServiceAccountsResponse`/`DeleteProjectServiceAccountResponse` (`{id, deleted, object}`).
- **Nested `ServiceAccounts.ApiKeys`**: `OrganizationProjectServiceAccountApiKeys::create()` (`POST .../service_accounts/{sa_id}/api_keys`) — the ONLY method on this sub-resource (confirmed: no list/retrieve/delete exist for this specific nested path).
- `CreateProjectServiceAccountApiKeyRequest`: `name: Option<String>`, `scopes: Option<Vec<String>>` — **the first `Vec<String>` REQUEST BODY field in this crate's Organization module** (not a query param — this is a JSON POST body, so the already-solved `to_query_pairs()`/`get_with_pairs` machinery from Usage does NOT apply here; a `Vec<String>` in a JSON body serializes completely normally via ordinary `serde`, this is only ever a problem for QUERY STRINGS specifically, not JSON bodies — confirmed no special handling needed).
- `CreateProjectServiceAccountApiKeyResponse`: `id: String`, `created_at: i64`, `name: String`, `object: String`, `value: String` — structurally identical to the embedded `CreateProjectServiceAccountApiKey` above but kept as a separate type (they're independently-defined response shapes in the source for two different endpoints, even though they happen to match today — same reasoning as sub-slice 3's `UsageEmbeddingsResult`/`UsageModerationsResult` non-merge).

### Projects.Users

- `OrganizationProjectUsers::create()` (`POST .../users`), `::list()` (`GET .../users`), `OrganizationProjectUser::retrieve()` (`GET .../users/{id}`), `::update()` (`POST .../users/{id}`), `::delete()` (`DELETE .../users/{id}`).
- `AddProjectUserRequest`: `role: String` (required, `"owner"`/`"member"`), `email: Option<String>`, `user_id: Option<String>` (confirmed both optional but at least one is presumably required server-side to identify the user — not enforced client-side, matching this crate's convention of not re-implementing server-side "at least one of" validation).
- `ProjectUser`: `id: String`, `added_at: i64`, `object: String`, `role: String`, `email: Option<String>`, `name: Option<String>`.
- `UpdateProjectUserRequest`: `role: Option<String>`.
- `ListProjectUsersRequest`: `after: Option<String>`, `limit: Option<u32>`.
- `ListProjectUsersResponse`/`DeleteProjectUserResponse` (`{id, deleted, object}`).

## Non-goals (explicit, deferred)

- **The 6 newer, unscaffolded project-level resources**: `certificates`, `data_retention`, `hosted_tool_permissions`, `model_permissions`, `spend_alerts`, `spend_limit` — genuinely separate future work, not part of the original 4-sub-slice commitment.
- **Roles-related sub-resources at every level** (top-level Roles, org Users.Roles/Groups.Roles from sub-slice 2, and project-level Groups.Roles/Users.Roles found this slice) — consistently deferred across the whole Organization/admin effort.

## Design notes

### File organization

Matches the existing scaffold exactly: `interfaces/organization/projects/{mod,api_keys,groups,rate_limits,service_accounts,users}.rs`, mirroring `apis/organization/projects/`'s existing directory split. `ServiceAccounts.ApiKeys` types live in `service_accounts.rs` (colocated, same precedent as Groups.Users living in `groups.rs` in sub-slice 2 — a small, single-parent-scoped sub-resource one level deep doesn't get its own file).

### Naming

`interfaces::organization::projects::api_keys::ProjectApiKey` vs. the already-existing top-level `interfaces::organization::admin_api_keys::AdminApiKey` — no collision (different names entirely). `ProjectServiceAccount`/`ProjectUser`/`ProjectGroup`/`ProjectRateLimit`/`ProjectApiKey` don't collide with anything else in the crate. Standard qualified-path convention throughout (`apis::organization::projects::*` imports its sibling `interfaces` submodule directly, no glob re-export — same as every other Organization/admin file).

### Confirmed asymmetries (do not assume symmetry — standing crate rule, now checked a fourth+ time)

- `CreateProjectServiceAccountResponse` (create-only) vs. `ProjectServiceAccount` (retrieve/update/list) are genuinely different shapes (extra `api_key` field, narrower `role` enum on create).
- `RemoveProjectGroupResponse` has no `id`, matching the established org-level `RemoveGroupUserResponse` pattern.
- Projects has `archive`, not `delete` — a genuinely different lifecycle operation, not a naming variant of delete.
- `Vec<String>` in `CreateProjectServiceAccountApiKeyRequest.scopes` is a JSON body field, NOT a query param — the Usage-slice `Vec<String>`-query-serialization limitation does not apply here at all; ordinary `#[derive(Serialize)]` handles this correctly with zero special handling.

## Testing

`wiremock` tests per sub-resource, following the exact established pattern (serialize request → deserialize response, asserting real field values, not tautologies): Projects CRUD (5 tests: create/retrieve/update/list/archive), ApiKeys (3: list/retrieve/delete), Groups (4: create/retrieve/list/delete — the delete test proves `RemoveProjectGroupResponse` has no `id`), RateLimits (2: list/update), ServiceAccounts (6: create — asserting the distinct `CreateProjectServiceAccountResponse` shape with `api_key` populated —/retrieve/update/list/delete, plus 1 for the nested `ServiceAccounts.ApiKeys::create`), Users (5: create/retrieve/update/list/delete).

## Versioning

Purely additive, no `client.rs` changes (all Projects endpoints reuse the crate's existing Bearer-auth `get`/`post`/`delete` — no query-array serialization needed anywhere in this slice, since Projects' only array-ish field, `scopes`, is a JSON body field). Pre-1.0 minor bump alongside the rest of Phase 2b, per established practice. **Completing this slice finishes Organization/admin (all 4 sub-slices) and, with it, the full original 9-scaffold Phase 2a list.**
