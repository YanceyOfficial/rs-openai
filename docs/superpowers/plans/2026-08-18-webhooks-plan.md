# Webhooks Signature Verification Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement `Webhooks::verify_and_unwrap` — verifies an incoming OpenAI webhook request's HMAC-SHA256 signature (per the Standard Webhooks specification) and a replay-protection timestamp check, then deserializes the payload into a typed `WebhookEvent`.

**Architecture:** Unlike every other slice in this project, this makes NO outbound HTTP call — it's pure local cryptographic verification. `interfaces/webhooks.rs` holds `WebhookEvent` (wire-format data). `apis/webhooks.rs` holds `WebhookHeaders` (a method-input parameter shape), `WebhookVerificationError` (a new, standalone `thiserror` error type distinct from `OpenAIError`), and the `verify_and_unwrap` method itself. Three new dependencies (`hmac`, `sha2`, `base64`) are added to `Cargo.toml` — the crate's first cryptographic dependencies.

**Tech Stack:** Rust, `hmac`/`sha2` (RustCrypto, HMAC-SHA256), `base64`, `serde`, `thiserror`. No `wiremock` needed this slice — tests are pure unit tests computing their own valid signatures, no HTTP mocking involved.

## Global Constraints

- Spec: `docs/superpowers/specs/2026-08-18-webhooks-design.md`. Confirmed via `AskUserQuestion`: add `hmac`/`sha2`/`base64` as new dependencies (exact versions below, verified live via `cargo add --dry-run`, not assumed); `WebhookEvent.data` stays raw `serde_json::Value` (no per-event-type union — the full event catalog was never confirmed by research).
- **The exact `hmac`/`base64` API calls in this plan were empirically verified in a scratch project before this plan was written** — a naive first attempt failed to compile twice (missing `use hmac::KeyInit;`, missing `use base64::Engine;`) before the working version below was confirmed to actually run correctly, including a genuine tampered-content negative case. Use the imports and calls in Step 5 verbatim. If your own attempt fails to compile with an error about a trait method not being found, the fix is almost always a missing trait import (`hmac::KeyInit` or `base64::Engine`) — check Step 5's exact import list before guessing at a different fix.
- `WebhookVerificationError` is a NEW, standalone error type (NOT a variant added to the existing `OpenAIError` in `shared/response_wrapper.rs`) — webhook verification has no HTTP call and is a genuinely different failure domain. Do not touch `OpenAIError` at all in this task.
- Signature comparison MUST use `Mac::verify_slice` (constant-time by construction) — do not manually compare signature bytes with `==` or `Vec::eq`, that would introduce a timing side-channel.
- The `webhook-signature` header can contain multiple space-separated entries (e.g. during secret rotation) and may contain non-`v1` schemes (e.g. `v1a` for asymmetric signing, out of scope this slice) — the verification loop must check every `v1,`-prefixed entry and succeed if ANY of them matches, silently skipping non-`v1` entries rather than erroring on them.
- Default timestamp tolerance is 300 seconds (5 minutes) when the caller passes `None` — this is a crate-chosen default (the Standard Webhooks spec doesn't mandate one), not itself confirmed against OpenAI docs; document it as such in the doc comment, don't present it as an OpenAI-mandated value.
- Stage commits with **explicit file paths** (`git add path/to/file.rs`), never `git add -A`.
- Run `cargo fmt` and `cargo clippy -p rs_openai --all-targets -- -D warnings` before every commit; both must be clean.
- Environment: this machine's shell routes HTTP through a local proxy that breaks `wiremock` tests. Prefix every `cargo test`/`build`/`clippy` invocation with `NO_PROXY=127.0.0.1,localhost`. This slice's own tests don't use `wiremock` (no HTTP involved), but `cargo build`/`test`/`clippy` for the whole crate still need the prefix since other modules' tests do. This is a local-shell-only concern — never write it into crate code, `Cargo.toml`, or CI config.

---

### Task 1: Webhook signature verification + event unwrapping

**Files:**
- Modify: `rs_openai/Cargo.toml` (add `hmac`, `sha2`, `base64` dependencies)
- Modify: `rs_openai/src/interfaces/mod.rs` (add `pub mod webhooks;`)
- Create: `rs_openai/src/interfaces/webhooks.rs` (`WebhookEvent`)
- Modify: `rs_openai/src/apis/webhooks.rs` (add `WebhookHeaders`, `WebhookVerificationError`, `verify_and_unwrap`, remove stale `#[allow(dead_code)]`)
- Test: `rs_openai/src/apis/webhooks.rs` (add a `#[cfg(test)] mod tests` block — none exists yet)

**Interfaces:**
- Consumes: existing `apis::webhooks::Webhooks<'a>` struct shape from Phase 2a (`Webhooks<'a> { openai: &'a OpenAI }`, unchanged — this method never reads `self.openai`, kept only for API-surface consistency with every other `client.<category>()` accessor in this crate).
- Produces: `interfaces::webhooks::WebhookEvent`, `apis::webhooks::{WebhookHeaders, WebhookVerificationError}`, `Webhooks::verify_and_unwrap` — this is the only task in this plan.

- [ ] **Step 1: Add the new dependencies**

In `rs_openai/Cargo.toml`, the `[dependencies]` section is NOT strictly alphabetically sorted (e.g. `rand`/`bon` already sit at the end, out of order) — new dependencies are appended, not carefully alphabetized. Add these three lines as the new last lines of the `[dependencies]` section, immediately after the existing `bon = "3.9.3"` line and before the blank line that precedes `[dev-dependencies]`:

```toml
base64 = "0.23"
hmac = "0.13"
sha2 = "0.11"
```

Run: `NO_PROXY=127.0.0.1,localhost cargo build -p rs_openai 2>&1 | tail -20`
Expected: builds successfully, pulling in the three new dependencies (and their transitive deps — `digest`, `crypto-common`, etc.). No errors.

- [ ] **Step 2: Write the failing test — a genuinely valid signature verifies and deserializes correctly**

Add a test module to `rs_openai/src/apis/webhooks.rs`:

```rust
#[cfg(test)]
mod tests {
    use crate::apis::webhooks::{WebhookHeaders, WebhookVerificationError};
    use crate::client::OpenAI;
    use base64::Engine;
    use hmac::{Hmac, KeyInit, Mac};
    use sha2::Sha256;

    type HmacSha256 = Hmac<Sha256>;

    /// Test-only helper: computes a valid "v1,<base64>" signature entry for a given
    /// secret/id/timestamp/payload, using the exact same algorithm `verify_and_unwrap`
    /// implements -- lets tests construct genuinely valid fixtures instead of hardcoding
    /// magic strings that would be hard to verify by inspection.
    fn sign(secret_b64_body: &str, webhook_id: &str, timestamp: &str, payload: &str) -> String {
        let secret_bytes = base64::engine::general_purpose::STANDARD
            .decode(secret_b64_body)
            .unwrap();
        let signed_content = format!("{webhook_id}.{timestamp}.{payload}");
        let mut mac = HmacSha256::new_from_slice(&secret_bytes).unwrap();
        mac.update(signed_content.as_bytes());
        let sig = mac.finalize().into_bytes();
        format!("v1,{}", base64::engine::general_purpose::STANDARD.encode(sig))
    }

    fn current_timestamp() -> String {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
            .to_string()
    }

    #[test]
    fn verify_and_unwrap_succeeds_with_valid_signature() {
        let secret_body = "dGVzdHNlY3JldA=="; // base64("testsecret")
        let secret = format!("whsec_{secret_body}");
        let webhook_id = "wh_1";
        let timestamp = current_timestamp();
        let payload = r#"{"id":"evt_1","object":"event","type":"response.completed","created_at":1700000000,"data":{"id":"resp_abc123"}}"#;
        let signature = sign(secret_body, webhook_id, &timestamp, payload);

        let client = OpenAI::new("sk-test");
        let headers = WebhookHeaders {
            webhook_id,
            webhook_timestamp: &timestamp,
            webhook_signature: &signature,
        };

        let event = client
            .webhooks()
            .verify_and_unwrap(payload, headers, &secret, None)
            .unwrap();

        assert_eq!(event.id, "evt_1");
        assert_eq!(event.kind, "response.completed");
        assert_eq!(event.data["id"], "resp_abc123");
    }
}
```

- [ ] **Step 3: Run test to verify it fails**

Run: `NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai verify_and_unwrap_succeeds_with_valid_signature 2>&1 | tail -40`
Expected: FAIL to compile — `WebhookHeaders`, `WebhookVerificationError`, and `Webhooks::verify_and_unwrap` don't exist yet.

- [ ] **Step 4: Create `rs_openai/src/interfaces/webhooks.rs`**

```rust
//! Typed webhook event envelope -- see docs/superpowers/specs/2026-08-18-webhooks-design.md.

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

Register it in `rs_openai/src/interfaces/mod.rs` by adding this new line as the last line in the file, after the existing `pub mod videos;`:

```rust
pub mod webhooks;
```

- [ ] **Step 5: Implement `WebhookHeaders`, `WebhookVerificationError`, and `verify_and_unwrap`, remove stale `#[allow(dead_code)]`**

Replace the full contents of `rs_openai/src/apis/webhooks.rs` (everything above the `#[cfg(test)]` block from Step 2 — leave that test module in place) with:

```rust
//! Signature verification and typed event payloads for OpenAI's outbound webhooks -- see
//! docs/superpowers/specs/2026-08-18-webhooks-design.md.

use crate::client::OpenAI;
use crate::interfaces::webhooks;
pub use crate::interfaces::webhooks::*;
use base64::Engine;
use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

const DEFAULT_TOLERANCE: std::time::Duration = std::time::Duration::from_secs(300);

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

pub struct Webhooks<'a> {
    openai: &'a OpenAI,
}

impl<'a> Webhooks<'a> {
    pub(crate) fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }

    /// Verifies an incoming OpenAI webhook request's signature (per the Standard Webhooks
    /// specification OpenAI's own guide defers to) and deserializes it into a typed
    /// `WebhookEvent`. `tolerance` bounds how far the `webhook-timestamp` header may drift
    /// from the current time before the request is rejected as a possible replay; `None`
    /// uses a crate-chosen default of 300 seconds (5 minutes) -- not itself an OpenAI-mandated
    /// value, since the spec leaves this to implementers.
    pub fn verify_and_unwrap(
        &self,
        payload: &str,
        headers: WebhookHeaders,
        secret: &str,
        tolerance: Option<std::time::Duration>,
    ) -> Result<webhooks::WebhookEvent, WebhookVerificationError> {
        let _ = self.openai; // unused: this method makes no HTTP call

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
                continue;
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

Note: `let _ = self.openai;` is REQUIRED, not optional — empirically confirmed in a scratch project before this plan was written: without any read of `self.openai` anywhere in the crate, `cargo clippy -D warnings` fails the build with `error: field \`openai\` is never read` (this is `-D dead-code`, implied by `-D warnings`). The field exists only for API-surface consistency with every other `client.<category>()` accessor in this crate (`Webhooks::new` must still accept and store it to match the established constructor signature `pub(crate) fn new(openai: &'a OpenAI) -> Self`), even though this specific method has no use for it. Keep this line.

- [ ] **Step 6: Run test to verify it passes**

Run: `NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai verify_and_unwrap_succeeds_with_valid_signature 2>&1 | tail -30`
Expected: PASS.

- [ ] **Step 7: Write the failing tests for the remaining verification paths**

Add to the same test module, after `verify_and_unwrap_succeeds_with_valid_signature`:

```rust
    #[test]
    fn verify_and_unwrap_fails_on_tampered_payload() {
        let secret_body = "dGVzdHNlY3JldA==";
        let secret = format!("whsec_{secret_body}");
        let webhook_id = "wh_2";
        let timestamp = current_timestamp();
        let original_payload = r#"{"id":"evt_2","object":"event","type":"response.completed","created_at":1700000000,"data":{}}"#;
        let signature = sign(secret_body, webhook_id, &timestamp, original_payload);

        let tampered_payload = r#"{"id":"evt_2","object":"event","type":"response.failed","created_at":1700000000,"data":{}}"#;

        let client = OpenAI::new("sk-test");
        let headers = WebhookHeaders {
            webhook_id,
            webhook_timestamp: &timestamp,
            webhook_signature: &signature,
        };

        let err = client
            .webhooks()
            .verify_and_unwrap(tampered_payload, headers, &secret, None)
            .unwrap_err();

        assert!(matches!(err, WebhookVerificationError::SignatureMismatch));
    }

    #[test]
    fn verify_and_unwrap_fails_on_timestamp_outside_tolerance() {
        let secret_body = "dGVzdHNlY3JldA==";
        let secret = format!("whsec_{secret_body}");
        let webhook_id = "wh_3";
        // 1 hour in the past -- outside the default 300-second tolerance.
        let old_timestamp = (std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
            - 3600)
            .to_string();
        let payload = r#"{"id":"evt_3","object":"event","type":"response.completed","created_at":1700000000,"data":{}}"#;
        let signature = sign(secret_body, webhook_id, &old_timestamp, payload);

        let client = OpenAI::new("sk-test");
        let headers = WebhookHeaders {
            webhook_id,
            webhook_timestamp: &old_timestamp,
            webhook_signature: &signature,
        };

        let err = client
            .webhooks()
            .verify_and_unwrap(payload, headers, &secret, None)
            .unwrap_err();

        assert!(matches!(err, WebhookVerificationError::TimestampOutOfTolerance));
    }

    #[test]
    fn verify_and_unwrap_fails_on_missing_whsec_prefix() {
        let webhook_id = "wh_4";
        let timestamp = current_timestamp();
        let payload = r#"{"id":"evt_4","object":"event","type":"response.completed","created_at":1700000000,"data":{}}"#;

        let client = OpenAI::new("sk-test");
        let headers = WebhookHeaders {
            webhook_id,
            webhook_timestamp: &timestamp,
            webhook_signature: "v1,irrelevant",
        };

        let err = client
            .webhooks()
            .verify_and_unwrap(payload, headers, "not-a-whsec-secret", None)
            .unwrap_err();

        assert!(matches!(err, WebhookVerificationError::MissingWhsecPrefix));
    }

    #[test]
    fn verify_and_unwrap_skips_unrecognized_signature_schemes() {
        let secret_body = "dGVzdHNlY3JldA==";
        let secret = format!("whsec_{secret_body}");
        let webhook_id = "wh_5";
        let timestamp = current_timestamp();
        let payload = r#"{"id":"evt_5","object":"event","type":"response.completed","created_at":1700000000,"data":{}}"#;
        let valid_v1 = sign(secret_body, webhook_id, &timestamp, payload);
        // A v1a (asymmetric) entry alongside the valid v1 entry -- must not cause a false
        // rejection; the loop should skip it and still find the valid v1 entry.
        let combined_header = format!("v1a,not-real-base64!!! {valid_v1}");

        let client = OpenAI::new("sk-test");
        let headers = WebhookHeaders {
            webhook_id,
            webhook_timestamp: &timestamp,
            webhook_signature: &combined_header,
        };

        let event = client
            .webhooks()
            .verify_and_unwrap(payload, headers, &secret, None)
            .unwrap();
        assert_eq!(event.id, "evt_5");
    }

    #[test]
    fn verify_and_unwrap_matches_any_signature_during_key_rotation() {
        let old_secret_body = "b2xkc2VjcmV0"; // base64("oldsecret")
        let new_secret_body = "bmV3c2VjcmV0"; // base64("newsecret")
        let webhook_id = "wh_6";
        let timestamp = current_timestamp();
        let payload = r#"{"id":"evt_6","object":"event","type":"response.completed","created_at":1700000000,"data":{}}"#;

        // Server signs with both the old and new secret during a rotation window.
        let old_sig = sign(old_secret_body, webhook_id, &timestamp, payload);
        let new_sig = sign(new_secret_body, webhook_id, &timestamp, payload);
        let combined_header = format!("{old_sig} {new_sig}");

        // Caller has already rotated to the new secret; only new_sig should match.
        let new_secret = format!("whsec_{new_secret_body}");
        let client = OpenAI::new("sk-test");
        let headers = WebhookHeaders {
            webhook_id,
            webhook_timestamp: &timestamp,
            webhook_signature: &combined_header,
        };

        let event = client
            .webhooks()
            .verify_and_unwrap(payload, headers, &new_secret, None)
            .unwrap();
        assert_eq!(event.id, "evt_6");
    }
```

- [ ] **Step 8: Run tests to verify they pass**

Run: `NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai webhooks:: 2>&1 | tail -30`
Expected: all 6 tests PASS.

- [ ] **Step 9: Format, lint, full test run (also serves as this plan's final cross-check — no separate task needed given the contained size of this slice)**

Run:
```bash
cargo fmt
NO_PROXY=127.0.0.1,localhost cargo clippy -p rs_openai --all-targets -- -D warnings
NO_PROXY=127.0.0.1,localhost cargo build -p rs_openai
NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai 2>&1 | tail -10
```
Expected: clippy clean, build clean, all tests pass (186 + 6 new = 192).

Also manually verify (read the file, don't just trust the count):
- `rs_openai/Cargo.toml` has exactly `hmac`, `sha2`, `base64` added, no other dependency changes.
- `rs_openai/src/apis/webhooks.rs`: `Webhooks` struct has NO `#[allow(dead_code)]` (has a real method now).
- No `OpenAIError` variant was added — `rs_openai/src/shared/response_wrapper.rs` is untouched by this task's diff.
- No per-event-type enum/union exists for `WebhookEvent.data` — it's `serde_json::Value` (scope check).
- Signature comparison uses `Mac::verify_slice` — grep `apis/webhooks.rs` for any raw `==` comparison of signature bytes; there should be none (scope/security check).

- [ ] **Step 10: Commit**

```bash
git add rs_openai/Cargo.toml rs_openai/src/interfaces/mod.rs rs_openai/src/interfaces/webhooks.rs rs_openai/src/apis/webhooks.rs
git commit -m "feat(webhooks): add signature verification and event unwrapping"
```
