#![allow(dead_code)]

use anyhow::{Context, Result};

pub fn generate_nova_jwt(identity_id: &str, private_key_pem: &[u8]) -> Result<String> {
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
    use p256::ecdsa::{signature::Signer, Signature, SigningKey};

    let pem_str = std::str::from_utf8(private_key_pem).context("Key not UTF-8")?;
    let secret_key = p256::SecretKey::from_sec1_pem(pem_str)
        .map_err(|e| anyhow::anyhow!("Invalid EC P-256 key: {}", e))?;
    let signing_key = SigningKey::from(secret_key);

    let header = serde_json::json!({"alg": "ES256", "typ": "JWT", "device": identity_id});
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    let payload = serde_json::json!({
        "iat": now,
        "exp": now + 86400,
        "jti": uuid::Uuid::new_v4().to_string()
    });

    let header_b64 = URL_SAFE_NO_PAD.encode(serde_json::to_vec(&header)?);
    let payload_b64 = URL_SAFE_NO_PAD.encode(serde_json::to_vec(&payload)?);
    let signing_input = format!("{}.{}", header_b64, payload_b64);

    let signature: Signature = signing_key.sign(signing_input.as_bytes());
    let sig_b64 = URL_SAFE_NO_PAD.encode(signature.to_bytes());

    Ok(format!("{}.{}", signing_input, sig_b64))
}
