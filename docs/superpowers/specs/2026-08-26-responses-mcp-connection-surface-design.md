# Responses: `mcp` Connection Surface — Design

Status: Approved
Date: 2026-08-26

## Context

Closes `mcp`'s last deferred gap. Confirmed via `gh api` against `openai-python`'s `types/responses/tool_param.py` (`Mcp` TypedDict). While researching this, also confirmed `return_token_budget` (previously recorded as a deferred Responses field) does not exist anywhere in the current SDK -- that was a stale note, not a real gap; corrected in memory.

## Goals

- `server_url` changes from a required `String` to `Option<String>` -- confirmed via source: "One of `server_url`, `connector_id`, or `tunnel_id` must be provided" (mutually exclusive connection mechanisms, not enforced client-side per this crate's convention). `#[builder(into)]` keeps existing `.server_url("...")` call sites source-compatible.
- `connector_id: Option<McpConnectorId>` -- an 8-variant enum for OpenAI's service connectors (Dropbox, Gmail, Google Calendar/Drive, Microsoft Teams, Outlook Calendar/Email, SharePoint). Each variant spelled out explicitly (not `rename_all`) since wire values don't follow a uniform case conversion (`connector_googlecalendar`, not `connector_google_calendar`).
- `tunnel_id: Option<String>` -- the third connection alternative (Secure MCP Tunnel ID).
- `allowed_callers: Option<Vec<McpAllowedCaller>>` -- `McpAllowedCaller` is `Direct`/`Programmatic`.
- `allowed_tools` changes from `Option<Vec<String>>` to `Option<McpAllowedTools>`, an untagged union of a plain name list or a `McpToolFilter` object (`{read_only: Option<bool>, tool_names: Option<Vec<String>>}`).

## A genuine pre-existing gap found and fixed while touching related code

`McpToolNameFilter` (used by `require_approval`'s `always`/`never` filters) was confirmed via source to be missing `read_only` entirely and wrongly required `tool_names` (source has both fields optional, `TypedDict total=False`). Since `McpAllowedToolsMcpToolFilter` (needed for this slice's `allowed_tools`) is confirmed structurally identical, renamed `McpToolNameFilter` → `McpToolFilter`, added `read_only`, made `tool_names` optional, and reused it for both `require_approval`'s filters and `allowed_tools`' filter form -- a deliberate breaking change (2 pre-existing call sites, both test code, updated).

## Design notes

All new/changed types in `interfaces/responses.rs`, adjacent to the existing `ResponseMcpTool`/`McpRequireApproval`/`McpApprovalFilter`. No `client.rs` changes.

## Testing

Three new tests: `connector_id` + `allowed_callers` + `McpToolFilter`-form `allowed_tools` serialize together; `tunnel_id` serializes with `server_url` omitted entirely (proving the mutually-exclusive design works without a required `server_url`); the existing `require_approval` filter test updated for the renamed/reshaped `McpToolFilter`.

## Versioning

Two deliberate breaking changes (`server_url`'s type, `McpToolNameFilter`'s rename/reshape), otherwise additive. Closes `mcp`'s last known gap.
