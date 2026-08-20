# Organization/Admin: Project-scoped Roles (Groups.Roles/Users.Roles within a Project) — Design

Status: Approved
Date: 2026-08-20

## Context

Closes the last remaining deferred piece of Organization/admin: role assignment for groups/users scoped to a specific project (as opposed to the org-level Groups.Roles/Users.Roles already shipped). Research via the established `gh api` SDK-source technique.

**A confirmed, genuinely surprising routing quirk**: both endpoints use routes WITHOUT the `/organization` prefix that every other Projects endpoint has — `/projects/{project_id}/groups/{group_id}/roles` and `/projects/{project_id}/users/{user_id}/roles` (and their `/roles/{role_id}` sub-paths), not `/organization/projects/...`. Confirmed by reading the literal `path_template(...)` calls directly in both `projects/groups/roles.py` and `projects/users/roles.py` (create AND retrieve both checked independently) — not assumed from the resource's logical position in the hierarchy. This is exactly the kind of routing surprise this project's research method exists to catch (same class of finding as Usage's `costs` living at `/organization/costs` rather than `/organization/usage/costs`).

**A major reuse opportunity, confirmed field-by-field**: every type this slice needs already exists, byte-identical, from the org-level Roles work — `Role` (`interfaces::organization::roles`), `AssignmentSource`/`RoleAssignmentDetail` (`interfaces::organization::users`), `RoleAssignmentGroupSummary` (`interfaces::organization::groups`), and — most surprisingly — `create`'s embedded user object is confirmed via the source's own import (`from ...organization_user import OrganizationUser`) to be `OrganizationUser`, the ORG-level user type, not a project-scoped one, despite this being a project-scoped endpoint. All four types are reused directly; only new request/response wrapper types are needed.

## Goals

### Project Groups.Roles (`interfaces/organization/projects/groups.rs`)

- `OrganizationProjectGroupRoles::create()` (`POST /projects/{project_id}/groups/{group_id}/roles`), `::list()` (`GET .../roles`), `OrganizationProjectGroupRole::retrieve()` (`GET .../roles/{role_id}`), `::delete()` (`DELETE .../roles/{role_id}`).
- `AssignProjectGroupRoleRequest`: `role_id: String` (required).
- `AssignProjectGroupRoleResponse`: `group: crate::interfaces::organization::groups::RoleAssignmentGroupSummary`, `object: String`, `role: crate::interfaces::organization::roles::Role`.
- `ListProjectGroupRolesRequest`: `after: Option<String>`, `limit: Option<u32>`, `order: Option<SortOrder>`.
- `ListProjectGroupRolesResponse`: `data: Vec<crate::interfaces::organization::users::RoleAssignmentDetail>`.
- `RemoveProjectGroupRoleResponse`: `deleted: bool`, `object: String` — no `id`.

### Project Users.Roles (`interfaces/organization/projects/users.rs`)

- `OrganizationProjectUserRoles::create()` (`POST /projects/{project_id}/users/{user_id}/roles`), `::list()` (`GET .../roles`), `OrganizationProjectUserRole::retrieve()` (`GET .../roles/{role_id}`), `::delete()` (`DELETE .../roles/{role_id}`).
- `AssignProjectUserRoleRequest`: `role_id: String` (required).
- `AssignProjectUserRoleResponse`: `object: String`, `role: crate::interfaces::organization::roles::Role`, `user: crate::interfaces::organization::users::OrganizationUser`.
- `ListProjectUserRolesRequest`: `after: Option<String>`, `limit: Option<u32>`, `order: Option<SortOrder>`.
- `ListProjectUserRolesResponse`: `data: Vec<crate::interfaces::organization::users::RoleAssignmentDetail>`.
- `RemoveProjectUserRoleResponse`: `deleted: bool`, `object: String` — no `id`.

## Non-goals

None remaining — this closes the last known gap in Organization/admin's Roles surface (org-level and project-level both now covered).

## Design notes

- File organization: appended to the existing `interfaces/organization/projects/groups.rs`/`users.rs` and `apis/organization/projects/groups.rs`/`users.rs` files (matching the established colocation precedent), with new `OrganizationProjectGroupRoles`/`OrganizationProjectGroupRole` and `OrganizationProjectUserRoles`/`OrganizationProjectUserRole` structs plus `.roles()` accessors on `OrganizationProjectGroup`/`OrganizationProjectUser`.
- The confirmed missing `/organization` prefix is load-bearing: routes are built as `format!("/projects/{}/groups/{}/roles", ...)`, NOT `format!("/organization/projects/{}/groups/{}/roles", ...)`. A path-assertion test proves this explicitly for both resources.
- No `client.rs` changes.

## Testing

`wiremock` tests per resource (4 each: create/retrieve/list/delete), each asserting the exact route path (proving the missing `/organization` prefix is handled correctly) and the delete test proving no `id` field, matching established patterns.

## Versioning

Purely additive. Completes Organization/admin's Roles surface at both org and project scope.
