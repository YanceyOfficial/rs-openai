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
        if now.abs_diff(timestamp) > tolerance.as_secs() {
            return Err(WebhookVerificationError::TimestampOutOfTolerance);
        }

        let secret_body = secret
            .strip_prefix("whsec_")
            .ok_or(WebhookVerificationError::MissingWhsecPrefix)?;
        let secret_bytes = base64::engine::general_purpose::STANDARD.decode(secret_body)?;

        let signed_content = format!(
            "{}.{}.{}",
            headers.webhook_id, headers.webhook_timestamp, payload
        );

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
        format!(
            "v1,{}",
            base64::engine::general_purpose::STANDARD.encode(sig)
        )
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

        assert!(matches!(
            err,
            WebhookVerificationError::TimestampOutOfTolerance
        ));
    }

    #[test]
    fn verify_and_unwrap_rejects_extreme_timestamp_without_panicking() {
        // A crafted webhook-timestamp at the very edge of i64's range: `now - timestamp`
        // would overflow i64 if computed via signed subtraction (panics under overflow
        // checks, silently wraps in release builds) -- the fix uses i64::abs_diff, which
        // cannot overflow. This must return an error, not panic, on genuinely hostile input.
        let secret_body = "dGVzdHNlY3JldA==";
        let secret = format!("whsec_{secret_body}");
        let webhook_id = "wh_7";
        let extreme_timestamp = i64::MIN.to_string();
        let payload = r#"{"id":"evt_7","object":"event","type":"response.completed","created_at":1700000000,"data":{}}"#;
        let signature = sign(secret_body, webhook_id, &extreme_timestamp, payload);

        let client = OpenAI::new("sk-test");
        let headers = WebhookHeaders {
            webhook_id,
            webhook_timestamp: &extreme_timestamp,
            webhook_signature: &signature,
        };

        let err = client
            .webhooks()
            .verify_and_unwrap(payload, headers, &secret, None)
            .unwrap_err();

        assert!(matches!(
            err,
            WebhookVerificationError::TimestampOutOfTolerance
        ));
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
}
