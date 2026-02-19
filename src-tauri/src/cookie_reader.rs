use std::path::PathBuf;
use std::process::Command;

use aes::Aes128;
use cbc::{
    cipher::{block_padding::Pkcs7, BlockDecryptMut, KeyIvInit},
    Decryptor,
};
use pbkdf2::pbkdf2_hmac;
use rusqlite::Connection;
use sha1::Sha1;

type Aes128CbcDec = Decryptor<Aes128>;

const CHROME_COOKIES_PATH: &str =
    "Library/Application Support/Google/Chrome/Default/Cookies";
const PBKDF2_ITERATIONS: u32 = 1003;
const SALT: &[u8] = b"saltysalt";
const IV: [u8; 16] = [0x20; 16];

#[derive(Debug, thiserror::Error)]
pub enum CookieError {
    #[error("Chrome cookies database not found")]
    DbNotFound,
    #[error("Cookie not found for claude.ai")]
    CookieNotFound,
    #[error("Failed to get Chrome Safe Storage key from Keychain")]
    KeychainError,
    #[error("Decryption failed: {0}")]
    DecryptionError(String),
    #[error("SQLite error: {0}")]
    SqliteError(#[from] rusqlite::Error),
}

fn get_chrome_safe_storage_key() -> Result<String, CookieError> {
    let output = Command::new("security")
        .args(["find-generic-password", "-s", "Chrome Safe Storage", "-w"])
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
    if encrypted.len() < 3 || &encrypted[0..3] != b"v10" {
        return Ok(String::from_utf8_lossy(encrypted).to_string());
    }

    let ciphertext = &encrypted[3..];
    let mut buf = ciphertext.to_vec();

    let decrypted = Aes128CbcDec::new(key.into(), &IV.into())
        .decrypt_padded_mut::<Pkcs7>(&mut buf)
        .map_err(|e| CookieError::DecryptionError(e.to_string()))?;

    Ok(String::from_utf8_lossy(decrypted).to_string())
}

pub fn read_claude_session_cookie() -> Result<String, CookieError> {
    let home = dirs::home_dir().ok_or(CookieError::DbNotFound)?;
    let cookies_path: PathBuf = home.join(CHROME_COOKIES_PATH);

    if !cookies_path.exists() {
        return Err(CookieError::DbNotFound);
    }

    // Copy DB to avoid locking issues with Chrome
    let temp_path = std::env::temp_dir().join("claude_widget_cookies");
    std::fs::copy(&cookies_path, &temp_path).map_err(|_| CookieError::DbNotFound)?;

    let conn = Connection::open(&temp_path)?;
    let password = get_chrome_safe_storage_key()?;
    let key = derive_key(&password);

    // Query all cookies for claude.ai to find session-related ones
    let mut stmt = conn.prepare(
        "SELECT name, encrypted_value, value FROM cookies WHERE host_key LIKE '%claude.ai' ORDER BY name",
    )?;

    let mut session_cookie = None;

    let rows = stmt.query_map([], |row| {
        let name: String = row.get(0)?;
        let encrypted: Vec<u8> = row.get(1)?;
        let plain: String = row.get(2)?;
        Ok((name, encrypted, plain))
    })?;

    for row in rows {
        let (name, encrypted, plain) = row?;

        // Look for session-related cookies
        if name == "sessionKey"
            || name == "__Secure-next-auth.session-token"
            || name.contains("session")
        {
            let value = if !encrypted.is_empty() {
                decrypt_cookie_value(&encrypted, &key)?
            } else {
                plain
            };

            if !value.is_empty() {
                session_cookie = Some((name, value));
                break;
            }
        }
    }

    let _ = std::fs::remove_file(temp_path);

    session_cookie
        .map(|(_, v)| v)
        .ok_or(CookieError::CookieNotFound)
}
