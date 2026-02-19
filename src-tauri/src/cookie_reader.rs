use std::process::Command;

use aes::Aes128;
use cbc::{
    cipher::{block_padding::NoPadding, BlockDecryptMut, KeyIvInit},
    Decryptor,
};
use pbkdf2::pbkdf2_hmac;
use rusqlite::Connection;
use sha1::Sha1;

type Aes128CbcDec = Decryptor<Aes128>;

const CLAUDE_COOKIES_PATH: &str = "Library/Application Support/Claude/Cookies";
const PBKDF2_ITERATIONS: u32 = 1003;
const SALT: &[u8] = b"saltysalt";

#[derive(Debug, thiserror::Error)]
pub enum CookieError {
    #[error("Claude desktop app cookies not found")]
    DbNotFound,
    #[error("Required cookie not found: {0}")]
    CookieNotFound(String),
    #[error("Failed to get Claude Safe Storage key from Keychain")]
    KeychainError,
    #[error("Decryption failed: {0}")]
    DecryptionError(String),
    #[error("SQLite error: {0}")]
    SqliteError(#[from] rusqlite::Error),
}

pub struct ClaudeCookies {
    pub session_key: String,
    pub org_id: String,
    pub all_cookies: String, // formatted "name=value; name=value" for HTTP header
}

fn get_safe_storage_key() -> Result<String, CookieError> {
    let output = Command::new("security")
        .args([
            "find-generic-password",
            "-s",
            "Claude Safe Storage",
            "-w",
        ])
        .output()
        .map_err(|_| CookieError::KeychainError)?;

    if !output.status.success() {
        return Err(CookieError::KeychainError);
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn derive_key(password: &str) -> [u8; 16] {
    let mut key = [0u8; 16];
    pbkdf2_hmac::<Sha1>(password.as_bytes(), SALT, PBKDF2_ITERATIONS, &mut key);
    key
}

fn decrypt_cookie_value(encrypted: &[u8], key: &[u8; 16]) -> Result<String, CookieError> {
    // Unencrypted cookie
    if encrypted.len() < 3 || &encrypted[0..3] != b"v10" {
        return Ok(String::from_utf8_lossy(encrypted).to_string());
    }

    let data = &encrypted[3..];

    // Need at least 3 AES blocks (48 bytes): nonce(16) + iv(16) + ciphertext(16+)
    if data.len() < 48 || data.len() % 16 != 0 {
        return Err(CookieError::DecryptionError(
            "ciphertext too short".to_string(),
        ));
    }

    // Claude desktop app format: first 16 bytes are opaque, next 16 are the CBC IV
    let iv: [u8; 16] = data[16..32].try_into().unwrap();
    let ciphertext = &data[32..];

    let mut buf = ciphertext.to_vec();
    Aes128CbcDec::new(key.into(), &iv.into())
        .decrypt_padded_mut::<NoPadding>(&mut buf)
        .map_err(|e| CookieError::DecryptionError(e.to_string()))?;

    // Remove PKCS7 padding manually
    if let Some(&pad_len) = buf.last() {
        if pad_len >= 1 && pad_len <= 16 && buf.len() >= pad_len as usize {
            buf.truncate(buf.len() - pad_len as usize);
        }
    }

    Ok(String::from_utf8_lossy(&buf).to_string())
}

pub fn read_claude_cookies() -> Result<ClaudeCookies, CookieError> {
    let home = dirs::home_dir().ok_or(CookieError::DbNotFound)?;
    let cookies_path = home.join(CLAUDE_COOKIES_PATH);

    if !cookies_path.exists() {
        return Err(CookieError::DbNotFound);
    }

    // Copy to avoid locking issues
    let temp_path = std::env::temp_dir().join("claude_widget_cookies");
    std::fs::copy(&cookies_path, &temp_path).map_err(|_| CookieError::DbNotFound)?;

    let conn = Connection::open(&temp_path)?;
    let password = get_safe_storage_key()?;
    let key = derive_key(&password);

    let mut stmt = conn.prepare(
        "SELECT name, encrypted_value FROM cookies WHERE host_key LIKE '%claude.ai%'",
    )?;

    let rows = stmt.query_map([], |row| {
        let name: String = row.get(0)?;
        let encrypted: Vec<u8> = row.get(1)?;
        Ok((name, encrypted))
    })?;

    let mut session_key = None;
    let mut org_id = None;
    let mut cookie_parts = Vec::new();

    for row in rows {
        let (name, encrypted) = row?;
        match decrypt_cookie_value(&encrypted, &key) {
            Ok(value) if !value.is_empty() => {
                if name == "sessionKey" {
                    session_key = Some(value.clone());
                } else if name == "lastActiveOrg" {
                    org_id = Some(value.clone());
                }
                cookie_parts.push(format!("{}={}", name, value));
            }
            _ => {}
        }
    }

    let _ = std::fs::remove_file(temp_path);

    let session_key =
        session_key.ok_or_else(|| CookieError::CookieNotFound("sessionKey".into()))?;
    let org_id = org_id.ok_or_else(|| CookieError::CookieNotFound("lastActiveOrg".into()))?;

    Ok(ClaudeCookies {
        session_key,
        org_id,
        all_cookies: cookie_parts.join("; "),
    })
}
