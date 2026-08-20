# Organization/Admin: Roles (org-level) — Design

Status: Approved
Date: 2026-08-20

## Context

Closes the last remaining gap in Organization/admin: Roles, deliberately deferred across all 4 sub-slices as a genuinely separate concern from core resource CRUD (custom permission definitions/assignments vs. who-exists/what's-in-what-group). Research via the established `gh api` SDK-source technique.

**Scope**: organization-level Roles only — top-level Roles CRUD (`/organization/roles`) + org Users.Roles (`/organization/users/{id}/roles`) + org Groups.Roles (`/organization/groups/{id}/roles`). Project-level Groups.Roles/Users.Roles (found during Projects sub-slice research) are deferred as a separate future increment — same reasoning as every prior sub-slice's scope discipline: this is already 3 resource areas, and project-scoped role assignment is a distinct enough unit to not fold in silently.

**A genuine shared-type design decision**: Users.Roles and Groups.Roles' `retrieve`/`list` response shape is a rich "role assignment detail" object (`id`, `assignment_sources`, `created_at`, `created_by`, `created_by_user_obj`, `description`, `metadata`, `name`, `permissions`, `predefined_role`, `resource_type`, `updated_at`) — confirmed byte-identical across all 4 independently-generated Python classes (`UsersRoleListResponse`/`UsersRoleRetrieveResponse`/`GroupsRoleListResponse`/`GroupsRoleRetrieveResponse`). Unlike Usage's `UsageEmbeddingsResult`/`UsageModerationsResult` (kept separate — genuinely different use-cases that happen to coincide today), this is the literal same concept ("the full detail of one role assignment") appearing at two scopes — modeled as ONE shared `RoleAssignmentDetail` type (+ shared `AssignmentSource`), used by both Users.Roles and Groups.Roles' `retrieve`/`list`. `create`'s response DOES genuinely differ between the two (embeds either a user summary or a group summary) and stays as two separate types.

**A confirmed non-reuse**: `create`'s embedded group summary (`{id, created_at, name, object, scim_managed}` — no `is_` prefix, no `group_type`) is NOT the same as the already-existing `interfaces::organization::groups::Group` (`{id, created_at, group_type, is_scim_managed, name}`) — genuinely different, smaller summary shape. New type, not reused.

## Goals

### Top-level Roles (`interfaces/organization/roles.rs`)

- `OrganizationRoles::create()` (`POST /organization/roles`), `::list()` (`GET /organization/roles`), `OrganizationRole::retrieve()` (`GET /organization/roles/{id}`), `::update()` (`POST /organization/roles/{id}`), `::delete()` (`DELETE /organization/roles/{id}`).
- `CreateRoleRequest`: `permissions: Vec<String>` (required), `role_name: String` (required), `description: Option<String>`.
- `Role`: `id: String`, `name: String`, `object: String`, `permissions: Vec<String>`, `predefined_role: bool`, `resource_type: String`, `description: Option<String>`.
- `UpdateRoleRequest`: `description: Option<String>`, `permissions: Option<Vec<String>>`, `role_name: Option<String>`.
- `ListRolesRequest`: `after: Option<String>`, `limit: Option<u32>`, `order: Option<SortOrder>`.
- `ListRolesResponse`: universal list shape, `data: Vec<Role>`.
- `DeleteRoleResponse`: `{id, deleted, object}`.

### Org Users.Roles (`interfaces/organization/users.rs`, appended — colocated per established precedent)

- `OrganizationUserRoles::create()` (`POST /organization/users/{user_id}/roles`), `::list()` (`GET .../roles`), `OrganizationUserRole::retrieve()` (`GET .../roles/{role_id}`), `::delete()` (`DELETE .../roles/{role_id}`).
- `AssignUserRoleRequest`: `role_id: String` (required).
- `AssignUserRoleResponse` (create only): `object: String` (`"user.role"`), `role: Role` (reuses top-level `Role`), `user: crate::interfaces::organization::users::OrganizationUser` (reuses the existing type — confirmed identical: the embedded `user` field's Python type IS `OrganizationUser`, not a separate summary).
- `AssignmentSource`: `principal_id: String`, `principal_type: String`.
- `RoleAssignmentDetail` (used for both `retrieve` and `list`): `id: String`, `name: String`, `permissions: Vec<String>`, `predefined_role: bool`, `resource_type: String`, `assignment_sources: Option<Vec<AssignmentSource>>`, `created_at: Option<i64>`, `created_by: Option<String>`, `created_by_user_obj: Option<serde_json::Value>` (untyped `Dict[str,object]` in source — raw JSON, matching this crate's established handling of confirmed-untyped-map fields), `description: Option<String>`, `metadata: Option<serde_json::Value>` (same untyped-map reasoning), `updated_at: Option<i64>`.
- `ListUserRolesRequest`: `after: Option<String>`, `limit: Option<u32>`, `order: Option<SortOrder>`.
- `ListUserRolesResponse`: `data: Vec<RoleAssignmentDetail>`.
- `RemoveUserRoleResponse`: `deleted: bool`, `object: String` — no `id`, matching every other membership-type delete in this crate.

### Org Groups.Roles (`interfaces/organization/groups.rs`, appended — colocated)

- `OrganizationGroupRoles::create()` (`POST /organization/groups/{group_id}/roles`), `::list()` (`GET .../roles`), `OrganizationGroupRole::retrieve()` (`GET .../roles/{role_id}`), `::delete()` (`DELETE .../roles/{role_id}`).
- `AssignGroupRoleRequest`: `role_id: String` (required).
- `RoleAssignmentGroupSummary` (new, distinct from `groups::Group`): `id: String`, `created_at: i64`, `name: String`, `object: String`, `scim_managed: bool`.
- `AssignGroupRoleResponse` (create only): `group: RoleAssignmentGroupSummary`, `object: String` (`"group.role"`), `role: Role` (reuses top-level `Role`).
- `ListGroupRolesRequest`: `after: Option<String>`, `limit: Option<u32>`, `order: Option<SortOrder>`.
- `ListGroupRolesResponse`: `data: Vec<crate::interfaces::organization::users::RoleAssignmentDetail>` (the shared type, defined once in `users.rs` since Users.Roles is implemented first, reused here).
- `RemoveGroupRoleResponse`: `deleted: bool`, `object: String` — no `id`.

## Non-goals

- Project-level Groups.Roles/Users.Roles — a separate future increment, not this one.

## Design notes

- File organization matches established precedent: top-level Roles gets its own new `interfaces/organization/roles.rs` (mirroring the existing `apis/organization/roles.rs` scaffold); Users.Roles/Groups.Roles types are appended to the existing `users.rs`/`groups.rs` files (matching how Groups.Users was colocated in `groups.rs` in sub-slice 2).
- No `client.rs` changes — all Bearer-auth `get`/`post`/`delete`, no new query-array or body-array needs.
- `apis::organization` continues its no-glob-re-export convention throughout.

## Testing

`wiremock` tests per resource area, following established patterns: top-level Roles (5: create/retrieve/update/list/delete), Users.Roles (4: create asserting the `user`-embedding response shape/retrieve+list asserting the shared `RoleAssignmentDetail`/delete asserting no `id`), Groups.Roles (4: create asserting the `group`-embedding response shape with the NEW distinct summary type/retrieve+list reusing the shared `RoleAssignmentDetail`/delete asserting no `id`).

## Versioning

Purely additive, no `client.rs` changes. Completes the LAST remaining gap in Organization/admin's org-level surface (project-level Roles remains a distinct, deferred future increment).
