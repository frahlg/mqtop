//! Secure loading and validation of ES256 private keys.
//!
//! Keys are stored as PEM files on disk. This module validates file permissions
//! (chmod 600 on Unix) and PEM format before returning key material.

#![allow(dead_code)]

use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};
use tracing::warn;

/// Expand ~ to home directory and load a private key PEM file.
/// Returns the raw PEM bytes.
///
/// On Unix, warns if file permissions are more permissive than 0o600.
pub fn load_private_key(path: &str) -> Result<Vec<u8>> {
    let expanded = expand_tilde(path);
    validate_key_permissions(&expanded)?;
    let pem_bytes = std::fs::read(&expanded)
        .with_context(|| format!("Failed to read private key: {}", expanded.display()))?;
    validate_pem_format(&pem_bytes)?;
    Ok(pem_bytes)
}

/// Check file permissions on Unix. Warns (does not error) if too permissive.
fn validate_key_permissions(path: &Path) -> Result<()> {
    if !path.exists() {
        bail!("Private key file not found: {}", path.display());
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let metadata = std::fs::metadata(path)?;
        let mode = metadata.permissions().mode() & 0o777;
        if mode & 0o077 != 0 {
            warn!(
                "Private key file {} has permissions {:o}, recommended 600",
                path.display(),
                mode
            );
        }
    }

    Ok(())
}

/// Validate that the bytes contain a valid EC P-256 private key in PEM format.
fn validate_pem_format(pem_bytes: &[u8]) -> Result<()> {
    let pem_str =
        std::str::from_utf8(pem_bytes).context("Private key file is not valid UTF-8")?;
    p256::SecretKey::from_sec1_pem(pem_str)
        .map_err(|e| anyhow::anyhow!("Invalid EC P-256 private key: {}", e))?;
    Ok(())
}

/// Expand ~ to the user's home directory.
fn expand_tilde(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest);
        }
    }
    PathBuf::from(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn test_key_pem() -> Vec<u8> {
        use p256::elliptic_curve::rand_core::OsRng;
        let secret_key = p256::SecretKey::random(&mut OsRng);
        let pem = secret_key
            .to_sec1_pem(Default::default())
            .unwrap();
        pem.as_bytes().to_vec()
    }

    #[test]
    fn test_load_valid_key() {
        let pem = test_key_pem();
        let mut tmp = NamedTempFile::new().unwrap();
        tmp.write_all(&pem).unwrap();

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(tmp.path(), std::fs::Permissions::from_mode(0o600)).unwrap();
        }

        let result = load_private_key(tmp.path().to_str().unwrap());
        assert!(result.is_ok());
    }

    #[test]
    fn test_reject_nonexistent_file() {
        let result = load_private_key("/nonexistent/path/key.pem");
        assert!(result.is_err());
    }

    #[test]
    fn test_reject_invalid_pem() {
        let mut tmp = NamedTempFile::new().unwrap();
        tmp.write_all(b"not a valid PEM file").unwrap();
        let result = load_private_key(tmp.path().to_str().unwrap());
        assert!(result.is_err());
    }

    #[test]
    fn test_expand_tilde() {
        let expanded = expand_tilde("~/some/path");
        assert!(!expanded.to_str().unwrap().starts_with("~"));
    }
}
