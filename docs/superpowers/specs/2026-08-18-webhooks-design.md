# Webhooks: Signature Verification + Event Unwrapping — Design

Status: Approved
Date: 2026-08-18

## Context

Eighth of the 9 Phase 2a scaffolds to go from skeleton to working implementation, and architecturally unlike every other slice this project has shipped: Webhooks are not a CRUD resource — OpenAI pushes events TO a caller's own HTTP endpoint, and this crate's job is to help verify that a received request genuinely came from OpenAI (not a forger) and hasn't been replayed, then deserialize it into a typed event. No outbound HTTP call is involved at all. Phase 2a scaffolded `apis::webhooks::Webhooks<'a>` with zero methods, already noting this in its own doc comment: "needs an 'unwrap'-style helper... rather than HTTP-calling methods."

Live research this session (WebFetch against `developers.openai.com`'s webhooks guide, then the public "Standard Webhooks" spec OpenAI's guide explicitly defers to) confirmed: OpenAI signs webhook requests per the open Standard Webhooks specification, not a bespoke scheme. Three headers arrive with every webhook request — `webhook-id`, `webhook-timestamp`, `webhook-signature` — and the exact algorithm is: concatenate `{webhook_id}.{webhook_timestamp}.{raw_body}` (period-delimited), HMAC-SHA256 it using the base64-decoded secret (stripped of its `whsec_` identifying prefix), base64-encode the result, and compare against one or more `v1,<base64>` entries in the space-delimited `webhook-signature` header (multiple entries support zero-downtime secret rotation). A timestamp-tolerance check guards against replay attacks; the spec doesn't mandate a specific default window, so this crate uses a documented, overridable default of 300 seconds (5 minutes), matching common practice among other Standard-Webhooks-compliant SDKs.

**A genuine, confirmed research gap, the same shape as ChatKit's Threads/Items and Evals' Runs**: OpenAI's guide names one example event type (`response.completed`) and explicitly defers the complete event catalog to "the API reference," which wasn't independently enumerated this session. Per `AskUserQuestion` ("Generic envelope + raw JSON data (Recommended)"), the confirmed common envelope (`id`/`object`/`type`/`created_at`) is fully typed, but `data` stays `serde_json::Value` rather than a per-event-type union — avoiding a guess at an unconfirmed, likely-large catalog. A caller matches on `.kind` (the `type` field) and parses `.data` for whichever event types they care about.

**A genuinely new category of change for this crate**: this is the first slice needing cryptographic primitives. Per `AskUserQuestion`, adds three new dependencies — `hmac`, `sha2` (RustCrypto project, HMAC-SHA256), and `base64` — all at their current live-confirmed latest stable versions (checked via `cargo add --dry-run`, not assumed from training knowledge): `hmac = "0.13"`, `sha2 = "0.11"`, `base64 = "0.23"`. Signature comparison uses `hmac::Mac::verify_slice`, which is constant-time by construction (RustCrypto's own guarantee) — no additional `subtle`-style dependency needed.

## Goals

- `WebhookHeaders<'h>`: `webhook_id: &'h str`, `webhook_timestamp: &'h str`, `webhook_signature: &'h str` — a small borrowed struct grouping the three required headers, avoiding positional-argument mixups.
- `WebhookEvent`: `id: String`, `object: String`, `kind: String` (`#[serde(rename = "type")]`, the established wire-`"type"`-collision convention), `created_at: i64`, `data: serde_json::Value`.
- `WebhookVerificationError` (a NEW, standalone `thiserror`-based error type — distinct from `OpenAIError`, since webhook verification is a fundamentally different failure domain with no HTTP call involved): `MissingWhsecPrefix`, `InvalidSecretEncoding(base64::DecodeError)`, `InvalidHmacKeyLength`, `InvalidTimestamp(std::num::ParseIntError)`, `TimestampOutOfTolerance`, `SignatureMismatch`, `PayloadDeserialize(serde_json::Error)`.
- `Webhooks::verify_and_unwrap(&self, payload: &str, headers: WebhookHeaders, secret: &str, tolerance: Option<std::time::Duration>) -> Result<WebhookEvent, WebhookVerificationError>` — the sole method. `tolerance` defaults to 300 seconds when `None`.

## Non-goals (explicit, deferred to later slices)

- **A per-event-type union for `data`** — stays raw JSON, pending a confirmed complete event catalog.
- **Asymmetric (ed25519, `v1a`) signature verification** — the Standard Webhooks spec supports both symmetric and asymmetric signing; OpenAI's own guide example only shows `v1` (symmetric), so only that path is implemented. `v1a`-prefixed entries in the signature header are silently skipped during comparison, not rejected outright (forward-compatible: a future secret rotation to asymmetric signing wouldn't break existing `v1` verification for callers still on symmetric secrets).
- **A framework-specific "extract headers from an HTTP request" helper** — the caller is responsible for pulling the three header values out of whatever web framework they use (axum, actix-web, warp, a raw `hyper` handler, etc.); this crate only verifies once you have the three string values in hand.

## Design

### File organization

New file `rs_openai/src/interfaces/webhooks.rs` (registered via `pub mod webhooks;` in `interfaces/mod.rs`) holds `WebhookEvent`. `WebhookHeaders` and `WebhookVerificationError` live in `apis/webhooks.rs` directly (not `interfaces/webhooks.rs`) since they're behavior-adjacent (verification-call ergonomics and error reporting), not wire-format data types — matching this crate's `interfaces` = data / `apis` = behavior split, where `WebhookHeaders` is an input parameter shape for a method call, not something (de)serialized to/from JSON.

### `Cargo.toml`

```toml
hmac = "0.13"
sha2 = "0.11"
base64 = "0.23"
```

### Types

```rust
// interfaces/webhooks.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct WebhookEvent {
    pub id: String,
    pub object: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub created_at: i64,
    pub data: serde_json::Value,
}
```

```rust
// apis/webhooks.rs
#[derive(Debug, thiserror::Error)]
pub enum WebhookVerificationError {
    #[error("webhook secret is missing the required \"whsec_\" prefix")]
    MissingWhsecPrefix,

    #[error("webhook secret is not valid base64: {0}")]
    InvalidSecretEncoding(#[from] base64::DecodeError),

    #[error("webhook secret has an invalid length for HMAC-SHA256")]
    InvalidHmacKeyLength,

    #[error("webhook-timestamp header is not a valid integer: {0}")]
    InvalidTimestamp(#[from] std::num::ParseIntError),

    #[error("webhook-timestamp is outside the allowed tolerance window")]
    TimestampOutOfTolerance,

    #[error("webhook-signature did not match any expected signature")]
    SignatureMismatch,

    #[error("failed to deserialize webhook payload: {0}")]
    PayloadDeserialize(#[from] serde_json::Error),
}

pub struct WebhookHeaders<'h> {
    pub webhook_id: &'h str,
    pub webhook_timestamp: &'h str,
    pub webhook_signature: &'h str,
}
```

### Verification algorithm

**The exact `hmac`/`base64` API surface below was empirically verified against the live, currently-resolved crate versions in a scratch project before writing this plan** — `hmac::Mac::new_from_slice` requires `hmac::KeyInit` in scope (not just `hmac::Mac`), and `base64`'s `.encode()`/`.decode()` methods require `base64::Engine` in scope (they're trait methods, not inherent methods on `GeneralPurpose`). Both were confirmed by an initial naive attempt failing to compile, then fixed and re-verified to actually run correctly (including a genuine tampered-content negative case). Use these exact imports and calls verbatim — do not "simplify" by dropping either trait import, the code will not compile without them.

```rust
use base64::Engine;
use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

const DEFAULT_TOLERANCE: std::time::Duration = std::time::Duration::from_secs(300);

impl<'a> Webhooks<'a> {
    pub fn verify_and_unwrap(
        &self,
        payload: &str,
        headers: WebhookHeaders,
        secret: &str,
        tolerance: Option<std::time::Duration>,
    ) -> Result<webhooks::WebhookEvent, WebhookVerificationError> {
        let tolerance = tolerance.unwrap_or(DEFAULT_TOLERANCE);

        let timestamp: i64 = headers.webhook_timestamp.parse()?;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        if (now - timestamp).unsigned_abs() > tolerance.as_secs() {
            return Err(WebhookVerificationError::TimestampOutOfTolerance);
        }

        let secret_body = secret
            .strip_prefix("whsec_")
            .ok_or(WebhookVerificationError::MissingWhsecPrefix)?;
        let secret_bytes = base64::engine::general_purpose::STANDARD.decode(secret_body)?;

        let signed_content =
            format!("{}.{}.{}", headers.webhook_id, headers.webhook_timestamp, payload);

        let mut matched = false;
        for entry in headers.webhook_signature.split(' ') {
            let Some(sig_b64) = entry.strip_prefix("v1,") else {
                continue; // skip v1a (asymmetric) and any unrecognized scheme
            };
            let Ok(sig_bytes) = base64::engine::general_purpose::STANDARD.decode(sig_b64) else {
                continue;
            };
            let mut mac = HmacSha256::new_from_slice(&secret_bytes)
                .map_err(|_| WebhookVerificationError::InvalidHmacKeyLength)?;
            mac.update(signed_content.as_bytes());
            if mac.verify_slice(&sig_bytes).is_ok() {
                matched = true;
                break;
            }
        }

        if !matched {
            return Err(WebhookVerificationError::SignatureMismatch);
        }

        Ok(serde_json::from_str(payload)?)
    }
}
```

Signature comparison uses `Mac::verify_slice`, which RustCrypto documents as constant-time — no separate timing-attack mitigation needed.

## Testing

Unit tests (no `wiremock` needed — this slice makes no HTTP calls) covering: a genuinely valid payload/secret/signature triple verifying successfully and deserializing into `WebhookEvent` with the correct `kind`/`data`; an invalid signature (tampered payload or wrong secret) returning `SignatureMismatch`; a timestamp outside the tolerance window returning `TimestampOutOfTolerance`; a secret missing the `whsec_` prefix returning `MissingWhsecPrefix`; a `webhook-signature` header containing an unrelated `v1a,...` entry alongside a valid `v1,...` entry still verifying successfully (proving the crate correctly skips unrecognized schemes rather than rejecting the whole header); multiple `v1,...` entries where only the second one matches (key-rotation scenario) still verifying successfully.

Test fixtures need a real HMAC-SHA256 computation to construct valid signatures — this can be done either by hand-deriving one fixed valid triple (secret/payload/timestamp/id → signature) offline and hardcoding it in the test, or by using the crate's own `hmac`/`base64` dependencies inside the test to compute a valid signature from a test secret, then verifying it round-trips. The latter is preferred: it doesn't require an external fixture and stays correct automatically if the algorithm ever needs to change.

## Versioning

Additive at the crate level (new module, new error type, three new dependencies). New dependencies are a real, if minor, footprint increase — but `hmac`/`sha2`/`base64` are all `no_std`-capable, dependency-light, and already transitively present in many Rust projects' dependency trees, so the practical incremental cost is low. Ships as a pre-1.0 minor bump alongside whatever else accumulates before the next release, per established practice.
