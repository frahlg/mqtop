#![allow(dead_code)]

use anyhow::{Context, Result};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use p256::ecdsa::{signature::Signer, Signature, SigningKey};
use p256::SecretKey;

const JWT_LIFETIME_SECS: u64 = 86400; // 24 hours

/// Generate an ES256 JWT for Nova Core auth-callout.
///
/// The JWT header includes `device: identity_id` which Nova Core's
/// mqtt_auth.go uses to look up the identity and its public keys.
///
/// The JWT has a 24-hour expiry, matching Nova Core's internal service pattern.
pub fn generate_nova_jwt(identity_id: &str, private_key_pem: &[u8]) -> Result<String> {
    let pem_str =
        std::str::from_utf8(private_key_pem).context("private key PEM is not valid UTF-8")?;
    let secret_key = SecretKey::from_sec1_pem(pem_str).context("failed to parse EC private key")?;
    let signing_key = SigningKey::from(secret_key);

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .context("system clock before UNIX epoch")?
        .as_secs();
    let exp = now + JWT_LIFETIME_SECS;
    let jti = uuid::Uuid::new_v4().to_string();

    let header = serde_json::json!({
        "alg": "ES256",
        "typ": "JWT",
        "device": identity_id,
    });
    let payload = serde_json::json!({
        "iat": now,
        "exp": exp,
        "jti": jti,
    });

    let header_b64 = URL_SAFE_NO_PAD.encode(header.to_string().as_bytes());
    let payload_b64 = URL_SAFE_NO_PAD.encode(payload.to_string().as_bytes());
    let signing_input = format!("{}.{}", header_b64, payload_b64);

    let signature: Signature = signing_key.sign(signing_input.as_bytes());
    let sig_bytes = signature.to_bytes();
    let sig_b64 = URL_SAFE_NO_PAD.encode(sig_bytes);

    Ok(format!("{}.{}", signing_input, sig_b64))
}

#[cfg(test)]
mod tests {
    use super::*;
    use p256::ecdsa::{signature::Verifier, VerifyingKey};
    use p256::elliptic_curve::rand_core::OsRng;

    fn test_key_pem() -> (SecretKey, Vec<u8>) {
        let secret_key = SecretKey::random(&mut OsRng);
        let pem = secret_key
            .to_sec1_pem(Default::default())
            .expect("failed to encode test key as PEM");
        (secret_key, pem.as_bytes().to_vec())
    }

    #[test]
    fn test_generate_jwt_structure() {
        let (_sk, pem) = test_key_pem();
        let jwt = generate_nova_jwt("test-device-123", &pem).unwrap();
        let parts: Vec<&str> = jwt.split('.').collect();
        assert_eq!(parts.len(), 3, "JWT must have exactly 3 dot-separated parts");
        assert!(!parts[0].is_empty(), "header must not be empty");
        assert!(!parts[1].is_empty(), "payload must not be empty");
        assert!(!parts[2].is_empty(), "signature must not be empty");
    }

    #[test]
    fn test_jwt_header_contains_device() {
        let (_sk, pem) = test_key_pem();
        let identity = "my-device-id";
        let jwt = generate_nova_jwt(identity, &pem).unwrap();
        let header_b64 = jwt.split('.').next().unwrap();
        let header_bytes = URL_SAFE_NO_PAD.decode(header_b64).unwrap();
        let header: serde_json::Value = serde_json::from_slice(&header_bytes).unwrap();
        assert_eq!(header["alg"], "ES256");
        assert_eq!(header["typ"], "JWT");
        assert_eq!(header["device"], identity);
    }

    #[test]
    fn test_jwt_payload_has_required_claims() {
        let (_sk, pem) = test_key_pem();
        let jwt = generate_nova_jwt("device-1", &pem).unwrap();
        let payload_b64 = jwt.split('.').nth(1).unwrap();
        let payload_bytes = URL_SAFE_NO_PAD.decode(payload_b64).unwrap();
        let payload: serde_json::Value = serde_json::from_slice(&payload_bytes).unwrap();

        assert!(payload["iat"].is_u64(), "iat must be a number");
        assert!(payload["exp"].is_u64(), "exp must be a number");
        assert!(payload["jti"].is_string(), "jti must be a string");

        let iat = payload["iat"].as_u64().unwrap();
        let exp = payload["exp"].as_u64().unwrap();
        assert_eq!(exp - iat, JWT_LIFETIME_SECS, "exp must be exactly 24h after iat");
    }

    #[test]
    fn test_invalid_pem_rejected() {
        let result = generate_nova_jwt("device", b"not a valid PEM");
        assert!(result.is_err(), "garbage PEM must be rejected");
    }

    #[test]
    fn test_jwt_signature_valid() {
        let (secret_key, pem) = test_key_pem();
        let jwt = generate_nova_jwt("device-sig-test", &pem).unwrap();

        let parts: Vec<&str> = jwt.split('.').collect();
        let signing_input = format!("{}.{}", parts[0], parts[1]);
        let sig_bytes = URL_SAFE_NO_PAD.decode(parts[2]).unwrap();
        let signature = Signature::from_slice(&sig_bytes).unwrap();

        let verifying_key = VerifyingKey::from(secret_key.public_key());
        verifying_key
            .verify(signing_input.as_bytes(), &signature)
            .expect("JWT signature must be valid");
    }
}
