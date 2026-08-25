# Organization: Certificates, Data Retention, Spend Alerts, Spend Limit, Hosted Tool Permissions, Model Permissions — Design

Status: Approved
Date: 2026-08-25

## Context

Closes the last known gap in Organization/admin: 6 newer Projects-adjacent resources that were never scaffolded in Phase 2a (confirmed via `gh api` against `openai-python`'s `resources/admin/organization/`). Four exist at BOTH org and project scope (Certificates, Data Retention, Spend Alerts, Spend Limit); two exist at project scope only (Hosted Tool Permissions, Model Permissions).

## Goals

### Certificates (org: full CRUD + activate/deactivate; project: list/activate/deactivate only)

Confirmed via source: certificates are uploaded once at the org level; a project can only activate/deactivate/list them, not create/retrieve/update/delete its own. One shared type set, reused at both scopes (differ only in the wire-format `object` string, modeled as plain `String` — genuine reuse, not aliasing):

- `Certificate { id, certificate_details: CertificateDetails, created_at, name: Option<String>, object: String, active: Option<bool> }`
- `CertificateDetails { content: Option<String>, expires_at: Option<i64>, valid_at: Option<i64> }` — `content` only populated when `retrieve`'s `include=content` is passed; relying on serde's built-in "missing key -> `None`" behavior for `Option<T>` fields (no `#[serde(default)]` needed).
- `CreateCertificateRequest { certificate: String, name: Option<String> }` (org only)
- `UpdateCertificateRequest { name: Option<String> }` (org only)
- `RetrieveCertificateRequest { include: Option<Vec<String>> }` (org only) — `Vec<String>` query param, uses the established `to_query_pairs()`/`get_with_pairs` workaround.
- `ListCertificatesRequest { after, limit, order }`, `ListCertificatesResponse { object, data: Vec<Certificate>, first_id, last_id, has_more }` — both scopes.
- `DeleteCertificateResponse { id, object }` (org only)
- `ActivateCertificatesRequest`/`DeactivateCertificatesRequest { certificate_ids: Vec<String> }` — both scopes.
- `CertificatesPage { data: Vec<Certificate>, object: String }` — activate/deactivate response (confirmed via source: `SyncPage`, a flat non-cursor page, `{data, object}` only).

Accessors: `client.organization().certificates()` (create/list/activate/deactivate) + `.certificate(id)` (retrieve/update/delete); `client.organization().project(id).certificates()` (list/activate/deactivate only, no ID-scoped sub-accessor since no per-cert project operations exist).

### Data Retention (org + project, singleton resource)

- `DataRetentionType` (6-variant `strum`+`serde` snake_case enum: `ZeroDataRetention`/`ModifiedAbuseMonitoring`/`EnhancedZeroDataRetention`/`EnhancedModifiedAbuseMonitoring`/`OrganizationDefault`/`None` — project's set is confirmed a strict superset of org's 4; shared as one enum per this crate's "don't re-implement server-side validation" convention, matching the Vector Store Files precedent for a request field that has narrower valid values at one scope than the type technically allows).
- `DataRetention { object: String, r#type: DataRetentionType }`, `UpdateDataRetentionRequest { retention_type: DataRetentionType }` — reused at both scopes (object field differs only in wire string).

Accessors: `client.organization().data_retention()` (retrieve/update, no ID); `client.organization().project(id).data_retention()`.

### Spend Alerts (org + project, full CRUD)

- `SpendAlertNotificationChannel { recipients: Vec<String>, r#type: String, subject_prefix: Option<String> }`
- `SpendAlert { id, currency: String, interval: String, notification_channel: SpendAlertNotificationChannel, object: String, threshold_amount: i64 }`
- `SpendAlertRequest { currency: String, interval: String, notification_channel: SpendAlertNotificationChannel, threshold_amount: i64 }` — confirmed identical shape for both `create` and `update` (full replace, not partial patch) at both scopes; one type serves both methods.
- `SpendAlertDeleted { id, deleted: bool, object: String }`
- `ListSpendAlertsRequest { after, before, limit, order }`, `ListSpendAlertsResponse { object, data: Vec<SpendAlert>, first_id, last_id, has_more }`

Accessors: `client.organization().spend_alerts()` (create/list) + `.spend_alert(id)` (retrieve/update/delete); `client.organization().project(id).spend_alerts()` + `.spend_alert(id)`.

### Spend Limit (org + project, singleton resource)

- `SpendLimitEnforcement { status: String }`
- `SpendLimit { currency: String, enforcement: SpendLimitEnforcement, interval: String, object: String, threshold_amount: i64 }`
- `UpdateSpendLimitRequest { currency: String, interval: String, threshold_amount: i64 }`
- `SpendLimitDeleted { deleted: bool, object: String }` — confirmed NO `id` field (unlike `SpendAlertDeleted`, which has one) — a genuine per-resource asymmetry, not an oversight.

Accessors: `client.organization().spend_limit()` (retrieve/update/delete, no ID); `client.organization().project(id).spend_limit()`.

### Hosted Tool Permissions (project only, singleton resource)

- `HostedToolPermissionState { enabled: bool }` — confirmed identical shape reused for all 5 sub-fields (both response and update-params).
- `ProjectHostedToolPermissions { code_interpreter, file_search, image_generation, mcp, web_search: HostedToolPermissionState }`
- `UpdateHostedToolPermissionsRequest { code_interpreter, file_search, image_generation, mcp, web_search: Option<HostedToolPermissionState> }` — all 5 fields genuinely optional (partial update, standard `#[serde(skip_serializing_if)]`, unlike Vector Store Files' explicit-null case).

Accessor: `client.organization().project(id).hosted_tool_permissions()` (retrieve/update, no ID).

### Model Permissions (project only, singleton resource)

- `ModelPermissionsMode` (2-variant `strum`+`serde` snake_case enum: `AllowList`/`DenyList`)
- `ProjectModelPermissions { mode: ModelPermissionsMode, model_ids: Vec<String>, object: String }` — source aliases this field to avoid a Pydantic `BaseModel.model_*` name clash that doesn't exist in Rust, so no rename needed here.
- `UpdateModelPermissionsRequest { mode: ModelPermissionsMode, model_ids: Vec<String> }`
- `ModelPermissionsDeleted { deleted: bool, object: String }`

Accessor: `client.organization().project(id).model_permissions()` (retrieve/update/delete, no ID).

## Design notes

- New files: `interfaces/organization/{certificates,data_retention,spend_alerts,spend_limit}.rs` (org-scope types, reused by project-scope code), `apis/organization/{certificates,data_retention,spend_alerts,spend_limit}.rs` (org-scope methods); `apis/organization/projects/{certificates,data_retention,spend_alerts,spend_limit,hosted_tool_permissions,model_permissions}.rs` (project-scope methods, importing the org-level interface types); `interfaces/organization/projects/{hosted_tool_permissions,model_permissions}.rs` (project-only types).
- All routes at project scope carry the `/organization` prefix (confirmed via `path_template` calls in source) — unlike the earlier-discovered Roles routing quirk, this is the normal case.
- No `client.rs` changes needed (plain `get`/`post`/`delete` throughout, no beta headers).

## Testing

`wiremock` tests per resource per scope, covering: create/update-serializes-request, retrieve/list-deserializes-response, delete-maps-api-error (matching this crate's standing coverage pattern), plus one test per singleton resource's retrieve/update. Certificates additionally test `activate`/`deactivate` and the `include=content` query param.

## Versioning

Purely additive. No `client.rs` changes. Closes the last known gap in Organization/admin.
