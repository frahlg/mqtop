//! Clipboard access with an OSC 52 fallback for remote (SSH) sessions.

use std::io::{self, Write};

/// Largest text sent via OSC 52; many terminals drop longer sequences.
const OSC52_MAX_BYTES: usize = 100_000;

const BASE64_ALPHABET: &[u8; 64] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

fn base64_encode(input: &[u8]) -> String {
    let mut out = String::with_capacity(input.len().div_ceil(3) * 4);
    for chunk in input.chunks(3) {
        let b = [
            chunk[0],
            chunk.get(1).copied().unwrap_or(0),
            chunk.get(2).copied().unwrap_or(0),
        ];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        out.push(BASE64_ALPHABET[(n >> 18) as usize & 63] as char);
        out.push(BASE64_ALPHABET[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 {
            BASE64_ALPHABET[(n >> 6) as usize & 63] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            BASE64_ALPHABET[n as usize & 63] as char
        } else {
            '='
        });
    }
    out
}

fn osc52_sequence(text: &str) -> String {
    format!("\x1b]52;c;{}\x07", base64_encode(text.as_bytes()))
}

fn is_ssh_session() -> bool {
    std::env::var_os("SSH_CONNECTION").is_some() || std::env::var_os("SSH_TTY").is_some()
}

fn send_osc52(text: &str) -> io::Result<()> {
    let mut stdout = io::stdout();
    stdout.write_all(osc52_sequence(text).as_bytes())?;
    stdout.flush()
}

/// Copy text to the clipboard.
///
/// Uses the local system clipboard when available. In SSH sessions, or when
/// no system clipboard exists, the text is also sent via OSC 52 so the local
/// terminal can set its clipboard. Returns a short description of what was
/// used, or an error message.
pub fn copy_text(text: &str) -> Result<&'static str, &'static str> {
    let system_ok = arboard::Clipboard::new()
        .and_then(|mut c| c.set_text(text.to_owned()))
        .is_ok();
    let want_osc52 = !system_ok || is_ssh_session();

    if !want_osc52 {
        return Ok("clipboard");
    }
    if text.len() > OSC52_MAX_BYTES {
        return if system_ok {
            Ok("clipboard")
        } else {
            Err("Too large for terminal clipboard (OSC 52)")
        };
    }
    match send_osc52(text) {
        Ok(()) if system_ok => Ok("clipboard + terminal"),
        Ok(()) => Ok("terminal clipboard (OSC 52)"),
        Err(_) if system_ok => Ok("clipboard"),
        Err(_) => Err("Clipboard unavailable"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_matches_rfc4648_vectors() {
        assert_eq!(base64_encode(b""), "");
        assert_eq!(base64_encode(b"f"), "Zg==");
        assert_eq!(base64_encode(b"fo"), "Zm8=");
        assert_eq!(base64_encode(b"foo"), "Zm9v");
        assert_eq!(base64_encode(b"foob"), "Zm9vYg==");
        assert_eq!(base64_encode(b"fooba"), "Zm9vYmE=");
        assert_eq!(base64_encode(b"foobar"), "Zm9vYmFy");
    }

    #[test]
    fn osc52_sequence_format() {
        assert_eq!(osc52_sequence("foo"), "\x1b]52;c;Zm9v\x07");
    }
}
