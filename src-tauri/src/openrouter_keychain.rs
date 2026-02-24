//! Stores and retrieves the OpenRouter API key from macOS Keychain.

use serde::Serialize;
use std::process::Command;

const SERVICE: &str = "com.israelmirsky.claude-codex-usage.openrouter";
const ACCOUNT: &str = "openrouter_api_key";

#[derive(Debug, Clone, Serialize)]
pub struct OpenRouterKeyStatus {
    pub configured: bool,
    pub masked_key: Option<String>,
}

fn mask_key(key: &str) -> String {
    let trimmed = key.trim();
    if trimmed.len() <= 10 {
        return "********".into();
    }
    let start = &trimmed[..6];
    let end = &trimmed[trimmed.len() - 4..];
    format!("{start}...{end}")
}

pub fn read_openrouter_api_key() -> Result<Option<String>, String> {
    let out = Command::new("security")
        .args([
            "find-generic-password",
            "-a",
            ACCOUNT,
            "-s",
            SERVICE,
            "-w",
        ])
        .output()
        .map_err(|e| format!("Failed to query macOS Keychain: {}", e))?;

    if out.status.success() {
        let key = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if key.is_empty() {
            Ok(None)
        } else {
            Ok(Some(key))
        }
    } else {
        let stderr = String::from_utf8_lossy(&out.stderr).to_lowercase();
        if stderr.contains("could not be found") {
            Ok(None)
        } else {
            Err(format!(
                "Failed to read OpenRouter key from Keychain: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            ))
        }
    }
}

pub fn set_openrouter_api_key(api_key: &str) -> Result<(), String> {
    let key = api_key.trim();
    if key.is_empty() {
        return Err("API key cannot be empty".into());
    }

    let out = Command::new("security")
        .args([
            "add-generic-password",
            "-a",
            ACCOUNT,
            "-s",
            SERVICE,
            "-w",
            key,
            "-U",
        ])
        .output()
        .map_err(|e| format!("Failed to write to macOS Keychain: {}", e))?;

    if out.status.success() {
        Ok(())
    } else {
        Err(format!(
            "Failed to save OpenRouter key to Keychain: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ))
    }
}

pub fn clear_openrouter_api_key() -> Result<(), String> {
    let out = Command::new("security")
        .args(["delete-generic-password", "-a", ACCOUNT, "-s", SERVICE])
        .output()
        .map_err(|e| format!("Failed to delete from macOS Keychain: {}", e))?;

    if out.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&out.stderr).to_lowercase();
        if stderr.contains("could not be found") {
            Ok(())
        } else {
            Err(format!(
                "Failed to clear OpenRouter key from Keychain: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            ))
        }
    }
}

pub fn get_openrouter_key_status() -> Result<OpenRouterKeyStatus, String> {
    let key = read_openrouter_api_key()?;
    Ok(match key {
        Some(raw) => OpenRouterKeyStatus {
            configured: true,
            masked_key: Some(mask_key(&raw)),
        },
        None => OpenRouterKeyStatus {
            configured: false,
            masked_key: None,
        },
    })
}
