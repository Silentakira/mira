use std::sync::Arc;
use tokio::sync::Mutex;
use serde::{Deserialize, Serialize};
use crate::EmailAccount;

pub struct AppState {
    pub accounts: Arc<Mutex<Vec<EmailAccount>>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            accounts: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

const SERVICE_NAME: &str = "mira-mail";

fn keyring_entry(key: &str) -> Result<keyring::Entry, String> {
    keyring::Entry::new(SERVICE_NAME, key)
        .map_err(|e| format!("Failed to create keychain entry: {}", e))
}

pub async fn save_account(state: &crate::store::AppState, account: &EmailAccount) -> Result<(), String> {
    let mut accounts = state.accounts.lock().await;

    // Check if account already exists
    if !accounts.iter().any(|a| a.id == account.id) {
        accounts.push(account.clone());
    } else {
        // Update existing
        for a in accounts.iter_mut() {
            if a.id == account.id {
                *a = account.clone();
                break;
            }
        }
    }

    // Persist account metadata (not tokens) to keychain
    let accounts_json = serde_json::to_string(&*accounts)
        .map_err(|e| format!("Failed to serialize accounts: {}", e))?;
    keyring_entry("accounts")?
        .set_password(&accounts_json)
        .map_err(|e| format!("Failed to save accounts: {}", e))?;

    Ok(())
}

pub fn save_token(account_id: &str, token: &str) -> Result<(), String> {
    let key = format!("token_{}", account_id);
    keyring_entry(&key)?
        .set_password(token)
        .map_err(|e| format!("Failed to save token: {}", e))
}

pub fn load_token(account_id: &str) -> Result<Option<String>, String> {
    let key = format!("token_{}", account_id);
    match keyring_entry(&key)?.get_password() {
        Ok(token) => Ok(Some(token)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(format!("Failed to load token: {}", e)),
    }
}

pub fn remove_token(account_id: &str) -> Result<(), String> {
    let key = format!("token_{}", account_id);
    match keyring_entry(&key)?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(format!("Failed to delete token: {}", e)),
    }
}

pub async fn load_accounts(state: &crate::store::AppState) -> Result<Vec<EmailAccount>, String> {
    let mut accounts = state.accounts.lock().await;
    if !accounts.is_empty() {
        return Ok(accounts.clone());
    }

    match keyring_entry("accounts")?.get_password() {
        Ok(json) => {
            let loaded: Vec<EmailAccount> = serde_json::from_str(&json)
                .map_err(|e| format!("Failed to parse accounts: {}", e))?;
            *accounts = loaded.clone();
            Ok(loaded)
        }
        Err(keyring::Error::NoEntry) => Ok(Vec::new()),
        Err(e) => Err(format!("Failed to load accounts: {}", e)),
    }
}

pub async fn remove_account(state: &crate::store::AppState, account_id: &str) -> Result<(), String> {
    let mut accounts = state.accounts.lock().await;
    accounts.retain(|a| a.id != account_id);

    let accounts_json = serde_json::to_string(&*accounts)
        .map_err(|e| format!("Failed to serialize accounts: {}", e))?;
    keyring_entry("accounts")?
        .set_password(&accounts_json)
        .map_err(|e| format!("Failed to update accounts: {}", e))?;

    remove_token(account_id)?;
    Ok(())
}

pub fn save_setting(key: &str, value: &str) -> Result<(), String> {
    let setting_key = format!("setting_{}", key);
    keyring_entry(&setting_key)?
        .set_password(value)
        .map_err(|e| format!("Failed to save setting: {}", e))
}

pub fn load_setting(key: &str) -> Result<Option<String>, String> {
    let setting_key = format!("setting_{}", key);
    match keyring_entry(&setting_key)?.get_password() {
        Ok(value) => Ok(Some(value)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(format!("Failed to load setting: {}", e)),
    }
}
