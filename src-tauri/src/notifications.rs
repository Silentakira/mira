use tauri::AppHandle;
use tauri::Emitter;

/// Poll for new emails every 60 seconds and show native macOS notifications
pub async fn start_polling(app_handle: AppHandle) {
    log::info!("Starting email notification polling...");

    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60));

    loop {
        interval.tick().await;

        // Check if notifications are enabled
        let notif_enabled = match crate::store::load_setting("notifications_enabled") {
            Ok(Some(v)) => v == "true",
            Ok(None) => true, // Default enabled
            Err(_) => true,
        };

        if !notif_enabled {
            continue;
        }

        // Check for new emails from each connected account
        if let Ok(state) = app_handle.try_state::<crate::store::AppState>() {
            let accounts = state.accounts.lock().await.clone();

            for account in accounts.iter().filter(|a| a.is_connected) {
                match check_new_emails(&app_handle, &account).await {
                    Ok(new_count) if new_count > 0 => {
                        log::info!("{} new emails for {}", new_count, account.email);
                    }
                    Err(e) => {
                        log::warn!("Error checking emails for {}: {}", account.email, e);
                    }
                    _ => {}
                }
            }
        }
    }
}

/// Check for new emails since last check
async fn check_new_emails(
    _app_handle: &AppHandle,
    account: &crate::EmailAccount,
) -> Result<u32, String> {
    // Get the last seen UID for this account
    let last_uid_key = format!("last_uid_{}", account.id);
    let last_uid: u32 = crate::store::load_setting(&last_uid_key)?
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);

    // Connect via IMAP and check for new messages
    let (mut client, _) = crate::oauth::get_imap_client(&account.id, &account.email).await?;
    client.select("INBOX").await
        .map_err(|e| format!("Select failed: {}", e))?;

    // Search for recent unseen messages
    let search_result = client.search("UNSEEN").await
        .map_err(|e| format!("Search failed: {}", e))?;

    let new_uids: Vec<u32> = search_result.into_iter()
        .filter(|&uid| uid > last_uid)
        .collect();

    if !new_uids.is_empty() {
        // Fetch details of new emails for notification
        let seq_set = new_uids.iter()
            .map(|u| u.to_string())
            .collect::<Vec<_>>()
            .join(",");

        let messages = client.fetch(&seq_set, "(ENVELOPE)").await
            .map_err(|e| format!("Fetch failed: {}", e))?;

        for msg in &messages {
            // Extract sender and subject from envelope
            let envelope = msg.envelope();
            let (sender, subject) = if let Some(env) = envelope {
                let from_name = env.from.as_ref()
                    .and_then(|addrs| addrs.first())
                    .and_then(|addr| addr.display_name.clone())
                    .unwrap_or_else(|| "Someone".to_string());

                let subj = env.subject
                    .as_ref()
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "New email".to_string());
                (from_name, subj)
            } else {
                ("Someone".to_string(), "New email".to_string())
            };

            // Emit event to frontend for notification handling
            let _ = _app_handle.emit("new-email-notification", serde_json::json!({
                "title": format!("Mira — New email"),
                "body": format!("{}: {}", sender, subject),
            }));

            // Update last UID
            if let Some(uid) = new_uids.last() {
                let _ = crate::store::save_setting(&last_uid_key, &uid.to_string());
            }
        }

        client.logout().await.ok();
        return Ok(new_uids.len() as u32);
    }

    client.logout().await.ok();
    Ok(0)
}
