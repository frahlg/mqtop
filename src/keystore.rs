#![allow(dead_code)]

use anyhow::{bail, Context, Result};
use std::path::PathBuf;
use tracing::warn;

pub fn load_private_key(path: &str) -> Result<Vec<u8>> {
    let expanded = expand_tilde(path);
    if !expanded.exists() {
        bail!("Private key not found: {}", expanded.display());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&expanded)?.permissions().mode() & 0o777;
        if mode & 0o077 != 0 {
            warn!(
                "Key {} has permissions {:o}, recommended 600",
                expanded.display(),
                mode
            );
        }
    }
    let pem_bytes =
        std::fs::read(&expanded).with_context(|| format!("Read key: {}", expanded.display()))?;
    let pem_str = std::str::from_utf8(&pem_bytes).context("Key not UTF-8")?;
    p256::SecretKey::from_sec1_pem(pem_str)
        .map_err(|e| anyhow::anyhow!("Invalid key: {}", e))?;
    Ok(pem_bytes)
}

fn expand_tilde(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest);
        }
    }
    PathBuf::from(path)
}
