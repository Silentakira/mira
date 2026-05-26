pub mod oauth;
pub mod imap;
pub mod smtp;
pub mod gemini;
pub mod store;
pub mod notifications;

use serde::{Deserialize, Serialize};
use tauri::Manager;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EmailAccount {
    pub id: String,
    pub email: String,
    pub name: String,
    pub is_connected: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EmailMessage {
    pub id: String,
    pub account_id: String,
    pub folder: String,
    pub from_name: String,
    pub from_address: String,
    pub to: Vec<String>,
    pub cc: Vec<String>,
    pub subject: String,
    pub body_text: String,
    pub body_html: Option<String>,
    pub preview: String,
    pub timestamp: i64,
    pub is_read: bool,
    pub is_starred: bool,
    pub has_attachments: bool,
    pub message_id: String,
    pub in_reply_to: Option<String>,
    pub references: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ComposeRequest {
    pub account_id: String,
    pub to: Vec<String>,
    pub cc: Vec<String>,
    pub subject: String,
    pub body: String,
    pub in_reply_to: Option<String>,
    pub references: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GeminiDraftRequest {
    pub api_key: String,
    pub prompt: String,
    pub style: String,
    pub original_email: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GeminiDraftResponse {
    pub draft: String,
}

#[tauri::command]
async fn get_accounts(state: tauri::State<'_, store::AppState>) -> Result<Vec<EmailAccount>, String> {
    let accounts = state.accounts.lock().await.clone();
    Ok(accounts)
}

#[tauri::command]
async fn connect_account(
    state: tauri::State<'_, store::AppState>,
) -> Result<EmailAccount, String> {
    let account = oauth::start_oauth_flow(&state).await?;
    Ok(account)
}

#[tauri::command]
async fn complete_auth_with_code(
    state: tauri::State<'_, store::AppState>,
    code: String,
) -> Result<EmailAccount, String> {
    let account = oauth::complete_with_code(&state, code).await?;
    Ok(account)
}

#[tauri::command]
async fn remove_account(
    state: tauri::State<'_, store::AppState>,
    account_id: String,
) -> Result<(), String> {
    store::remove_account(&state, &account_id).await
}

#[tauri::command]
async fn fetch_emails(
    state: tauri::State<'_, store::AppState>,
    account_id: String,
    folder: String,
    page: u32,
    page_size: u32,
) -> Result<Vec<EmailMessage>, String> {
    imap::fetch_emails(&state, &account_id, &folder, page, page_size).await
}

#[tauri::command]
async fn fetch_unified_inbox(
    state: tauri::State<'_, store::AppState>,
    page: u32,
    page_size: u32,
) -> Result<Vec<EmailMessage>, String> {
    imap::fetch_unified_inbox(&state, page, page_size).await
}

#[tauri::command]
async fn get_email_body(
    state: tauri::State<'_, store::AppState>,
    account_id: String,
    message_id: String,
) -> Result<EmailMessage, String> {
    imap::get_email_detail(&state, &account_id, &message_id).await
}

#[tauri::command]
async fn send_email(
    state: tauri::State<'_, store::AppState>,
    request: ComposeRequest,
) -> Result<(), String> {
    smtp::send_email(&state, request).await
}

#[tauri::command]
async fn draft_with_gemini(request: GeminiDraftRequest) -> Result<GeminiDraftResponse, String> {
    gemini::draft_email(&request.api_key, &request.prompt, &request.style, request.original_email.as_deref()).await
}

#[tauri::command]
async fn mark_as_read(
    state: tauri::State<'_, store::AppState>,
    account_id: String,
    message_id: String,
) -> Result<(), String> {
    imap::mark_as_read(&state, &account_id, &message_id).await
}

#[tauri::command]
async fn delete_email(
    state: tauri::State<'_, store::AppState>,
    account_id: String,
    message_id: String,
) -> Result<(), String> {
    imap::delete_email(&state, &account_id, &message_id).await
}

#[tauri::command]
async fn save_setting(key: String, value: String) -> Result<(), String> {
    store::save_setting(&key, &value)
}

#[tauri::command]
async fn load_setting(key: String) -> Result<Option<String>, String> {
    store::load_setting(&key)
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(store::AppState::default())
        .invoke_handler(tauri::generate_handler![
            get_accounts,
            connect_account,
            complete_auth_with_code,
            remove_account,
            fetch_emails,
            fetch_unified_inbox,
            get_email_body,
            send_email,
            draft_with_gemini,
            mark_as_read,
            delete_email,
            save_setting,
            load_setting,
        ])
        .setup(|app| {
            // Initialize notification polling
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                notifications::start_polling(handle).await;
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Mira");
}
