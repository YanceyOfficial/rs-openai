//! Organization/admin tier types -- see docs/superpowers/specs/2026-08-19-organization-design.md.
//!
//! Separate admin-level API surface distinct from the "call the model" tier. Accessed via
//! `client.organization()`. All types support wire-format (de)serialization via serde.

use serde::{Deserialize, Serialize};

// ============================================================================
// Audit Logs
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLog {
    pub id: String,
    pub object: String,
    pub created_at: i64,
    pub actor: AuditLogActor,
    pub event: String,
    pub resource: AuditLogResource,
    pub context: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogActor {
    pub r#type: String, // "user", "service_account", "api_key"
    pub id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogResource {
    pub r#type: String, // resource type
    pub id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListAuditLogsRequest {
    pub limit: Option<i32>,
    pub after: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListAuditLogsResponse {
    pub object: String,
    pub data: Vec<AuditLog>,
    pub has_more: bool,
}

// ============================================================================
// Admin API Keys
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminApiKey {
    pub id: String,
    pub object: String,
    pub created_at: i64,
    pub name: Option<String>,
    pub last_used_at: Option<i64>,
    pub scopes: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAdminApiKeyRequest {
    pub name: Option<String>,
    pub scopes: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListAdminApiKeysRequest {
    pub limit: Option<i32>,
    pub after: Option<String>,
    pub order: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListAdminApiKeysResponse {
    pub object: String,
    pub data: Vec<AdminApiKey>,
    pub has_more: bool,
}

// ============================================================================
// Groups
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Group {
    pub id: String,
    pub object: String,
    pub created_at: i64,
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateGroupRequest {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateGroupRequest {
    pub name: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListGroupsRequest {
    pub limit: Option<i32>,
    pub after: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListGroupsResponse {
    pub object: String,
    pub data: Vec<Group>,
    pub has_more: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupUser {
    pub id: String,
    pub object: String,
    pub created_at: i64,
    pub user_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateGroupUserRequest {
    pub user_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListGroupUsersRequest {
    pub limit: Option<i32>,
    pub after: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListGroupUsersResponse {
    pub object: String,
    pub data: Vec<GroupUser>,
    pub has_more: bool,
}

// ============================================================================
// Invites
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Invite {
    pub id: String,
    pub object: String,
    pub created_at: i64,
    pub email: String,
    pub role: String,
    pub expires_at: Option<i64>,
    pub accepted_at: Option<i64>,
    pub invited_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateInviteRequest {
    pub email: String,
    pub role: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListInvitesRequest {
    pub limit: Option<i32>,
    pub after: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListInvitesResponse {
    pub object: String,
    pub data: Vec<Invite>,
    pub has_more: bool,
}

// ============================================================================
// Roles
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Role {
    pub id: String,
    pub object: String,
    pub created_at: Option<i64>,
    pub name: String,
    pub description: Option<String>,
    pub permissions: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateRoleRequest {
    pub name: String,
    pub description: Option<String>,
    pub permissions: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateRoleRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub permissions: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListRolesRequest {
    pub limit: Option<i32>,
    pub after: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListRolesResponse {
    pub object: String,
    pub data: Vec<Role>,
    pub has_more: bool,
}

// ============================================================================
// Users
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub object: String,
    pub created_at: i64,
    pub email: String,
    pub name: Option<String>,
    pub role: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateUserRequest {
    pub name: Option<String>,
    pub role: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListUsersRequest {
    pub limit: Option<i32>,
    pub after: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListUsersResponse {
    pub object: String,
    pub data: Vec<User>,
    pub has_more: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserRole {
    pub id: String,
    pub object: String,
    pub created_at: i64,
    pub role_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateUserRoleRequest {
    pub role_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListUserRolesRequest {
    pub limit: Option<i32>,
    pub after: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListUserRolesResponse {
    pub object: String,
    pub data: Vec<UserRole>,
    pub has_more: bool,
}

// ============================================================================
// Projects
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub object: String,
    pub created_at: i64,
    pub updated_at: Option<i64>,
    pub name: String,
    pub status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateProjectRequest {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateProjectRequest {
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListProjectsRequest {
    pub limit: Option<i32>,
    pub after: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListProjectsResponse {
    pub object: String,
    pub data: Vec<Project>,
    pub has_more: bool,
}

// ============================================================================
// Project API Keys (sub-resource)
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectApiKey {
    pub id: String,
    pub object: String,
    pub created_at: i64,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListProjectApiKeysRequest {
    pub limit: Option<i32>,
    pub after: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListProjectApiKeysResponse {
    pub object: String,
    pub data: Vec<ProjectApiKey>,
    pub has_more: bool,
}

// ============================================================================
// Project Groups (sub-resource)
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectGroup {
    pub id: String,
    pub object: String,
    pub created_at: i64,
    pub group_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListProjectGroupsRequest {
    pub limit: Option<i32>,
    pub after: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListProjectGroupsResponse {
    pub object: String,
    pub data: Vec<ProjectGroup>,
    pub has_more: bool,
}

// ============================================================================
// Project Rate Limits (sub-resource)
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectRateLimit {
    pub object: String,
    pub requests_per_minute: Option<i32>,
    pub tokens_per_minute: Option<i32>,
    pub batch_requests_per_day: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateProjectRateLimitRequest {
    pub requests_per_minute: Option<i32>,
    pub tokens_per_minute: Option<i32>,
    pub batch_requests_per_day: Option<i32>,
}

// ============================================================================
// Service Accounts (sub-resource)
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceAccount {
    pub id: String,
    pub object: String,
    pub created_at: i64,
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateServiceAccountRequest {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListServiceAccountsRequest {
    pub limit: Option<i32>,
    pub after: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListServiceAccountsResponse {
    pub object: String,
    pub data: Vec<ServiceAccount>,
    pub has_more: bool,
}

// ============================================================================
// Project Users (sub-resource)
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectUser {
    pub id: String,
    pub object: String,
    pub created_at: i64,
    pub user_id: String,
    pub role: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateProjectUserRequest {
    pub user_id: String,
    pub role: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateProjectUserRequest {
    pub role: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListProjectUsersRequest {
    pub limit: Option<i32>,
    pub after: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListProjectUsersResponse {
    pub object: String,
    pub data: Vec<ProjectUser>,
    pub has_more: bool,
}

// ============================================================================
// Usage (read-only)
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageData {
    pub object: String,
    pub snapshot_id: String,
    pub name: String,
    pub value: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageResponse {
    pub object: String,
    pub data: Vec<UsageData>,
}
