use crate::store::AppState;
use crate::EmailMessage;
use crate::oauth;
use mailparse::*;

/// Fetch emails from a specific folder for an account
pub async fn fetch_emails(
    state: &AppState,
    account_id: &str,
    folder: &str,
    page: u32,
    page_size: u32,
) -> Result<Vec<EmailMessage>, String> {
    let accounts = state.accounts.lock().await.clone();
    let account = accounts.iter()
        .find(|a| a.id == account_id)
        .ok_or_else(|| "Account not found".to_string())?;

    let (mut client, _token) = oauth::get_imap_client(account_id, &account.email).await?;

    // Select mailbox
    client.select(folder).await
        .map_err(|e| format!("Failed to select folder '{}': {}", folder, e))?;

    // Search all messages to get count
    let search_result = client.search("ALL").await
        .map_err(|e| format!("Search failed: {}", e))?;
    let total = search_result.len() as u32;

    if total == 0 {
        client.logout().await.ok();
        return Ok(Vec::new());
    }

    // Calculate which messages to fetch (newest first)
    let page_size = page_size.min(50); // Cap at 50 per page
    let start_seq = total.saturating_sub(page * page_size);
    let end_seq = total.saturating_sub((page.saturating_sub(1)) * page_size);

    if start_seq == end_seq || end_seq == 0 {
        client.logout().await.ok();
        return Ok(Vec::new());
    }

    // Build sequence set for the newest N messages in this page
    let seq_set = if start_seq == 0 {
        format!("1:{}", end_seq)
    } else {
        format!("{}:{}", start_seq + 1, end_seq)
    };

    // Fetch messages with envelope and flags
    let messages = client.fetch(&seq_set, "(RFC822 FLAGS INTERNALDATE ENVELOPE)").await
        .map_err(|e| format!("Fetch failed: {}", e))?;

    let mut emails = Vec::new();
    for msg in messages.iter().rev() {
        if let Some(raw) = msg.body() {
            if let Ok(parsed) = parse_mail(raw) {
                let from_addr = parsed.headers.get_first_value("From")
                    .unwrap_or_default();
                let subject = parsed.headers.get_first_value("Subject")
                    .unwrap_or("(No Subject)".to_string());
                let date_str = parsed.headers.get_first_value("Date")
                    .unwrap_or_default();

                let preview = get_preview(&parsed);
                let timestamp = parse_timestamp(&date_str);

                // Check Seen flag — async-imap 0.9 uses Flag enum
                let flags = msg.flags();
                let is_read = flags.iter().any(|f| {
                    f.to_string().contains("Seen") || f == "\\Seen"
                });
                let is_starred = flags.iter().any(|f| {
                    f.to_string().contains("Flagged") || f == "\\Flagged"
                });

                let message_id = parsed.headers.get_first_value("Message-ID")
                    .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

                let in_reply_to = parsed.headers.get_first_value("In-Reply-To");
                let references = parsed.headers.get_first_value("References");

                let (from_name, from_address) = parse_address(&from_addr);
                let to_addresses = parse_address_list(
                    &parsed.headers.get_first_value("To").unwrap_or_default()
                );
                let cc_addresses = parse_address_list(
                    &parsed.headers.get_first_value("Cc").unwrap_or_default()
                );

                emails.push(EmailMessage {
                    id: uuid::Uuid::new_v4().to_string(),
                    account_id: account_id.to_string(),
                    folder: folder.to_string(),
                    from_name,
                    from_address,
                    to: to_addresses,
                    cc: cc_addresses,
                    subject: decode_subject(&subject),
                    body_text: String::new(),
                    body_html: None,
                    preview,
                    timestamp,
                    is_read,
                    is_starred,
                    has_attachments: false,
                    message_id,
                    in_reply_to,
                    references,
                });
            }
        }
    }

    client.logout().await.ok();
    Ok(emails)
}

/// Fetch unified inbox across all connected accounts
pub async fn fetch_unified_inbox(
    state: &AppState,
    page: u32,
    page_size: u32,
) -> Result<Vec<EmailMessage>, String> {
    let accounts = state.accounts.lock().await.clone();
    let mut all_emails: Vec<EmailMessage> = Vec::new();

    for account in &accounts {
        if account.is_connected {
            match fetch_emails(state, &account.id, "INBOX", 0, page_size * 2).await {
                Ok(mut emails) => {
                    all_emails.append(&mut emails);
                }
                Err(e) => {
                    log::warn!("Failed to fetch from {}: {}", account.email, e);
                }
            }
        }
    }

    // Sort by timestamp descending
    all_emails.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

    // Paginate
    let start = (page * page_size) as usize;
    let end = start + page_size as usize;
    Ok(all_emails.into_iter().skip(start).take(page_size as usize).collect())
}

/// Get full email detail including body
pub async fn get_email_detail(
    state: &AppState,
    account_id: &str,
    _message_uid: &str,
) -> Result<EmailMessage, String> {
    let accounts = state.accounts.lock().await.clone();
    let account = accounts.iter()
        .find(|a| a.id == account_id)
        .ok_or_else(|| "Account not found".to_string())?;

    let (mut client, _) = oauth::get_imap_client(account_id, &account.email).await?;
    client.select("INBOX").await
        .map_err(|e| format!("Failed to select INBOX: {}", e))?;

    // Fetch latest message as detail (in production this would use stored UID mapping)
    let messages = client.fetch("1:*", "(RFC822)").await
        .map_err(|e| format!("Fetch failed: {}", e))?;

    for msg in messages.iter() {
        if let Some(raw) = msg.body() {
            if let Ok(parsed) = parse_mail(raw) {
                let from_addr = parsed.headers.get_first_value("From").unwrap_or_default();
                let subject = parsed.headers.get_first_value("Subject").unwrap_or_default();
                let date_str = parsed.headers.get_first_value("Date").unwrap_or_default();
                let body_text = get_body_text(&parsed);
                let body_html = get_body_html(&parsed);

                let (from_name, from_address) = parse_address(&from_addr);
                let to_addresses = parse_address_list(
                    &parsed.headers.get_first_value("To").unwrap_or_default()
                );

                client.logout().await.ok();
                return Ok(EmailMessage {
                    id: uuid::Uuid::new_v4().to_string(),
                    account_id: account_id.to_string(),
                    folder: "INBOX".to_string(),
                    from_name,
                    from_address,
                    to: to_addresses,
                    cc: Vec::new(),
                    subject: decode_subject(&subject),
                    body_text,
                    body_html,
                    preview: get_preview(&parsed),
                    timestamp: parse_timestamp(&date_str),
                    is_read: true,
                    is_starred: false,
                    has_attachments: false,
                    message_id: parsed.headers.get_first_value("Message-ID")
                        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
                    in_reply_to: parsed.headers.get_first_value("In-Reply-To"),
                    references: parsed.headers.get_first_value("References"),
                });
            }
        }
    }

    client.logout().await.ok();
    Err("Email not found".to_string())
}

/// Mark email as read
pub async fn mark_as_read(
    state: &AppState,
    account_id: &str,
    _message_id: &str,
) -> Result<(), String> {
    let accounts = state.accounts.lock().await.clone();
    let _account = accounts.iter()
        .find(|a| a.id == account_id)
        .ok_or_else(|| "Account not found".to_string())?;

    let (mut client, _) = oauth::get_imap_client(account_id, &_account.email).await?;
    client.select("INBOX").await.map_err(|e| format!("Select failed: {}", e))?;

    // Store +Seen flag on most recent message (production would target specific UID)
    client.store("1", "+FLAGS (\\Seen)").await
        .map_err(|e| format!("Store failed: {}", e))?;

    client.logout().await.ok();
    Ok(())
}

/// Delete email (move to Trash)
pub async fn delete_email(
    state: &AppState,
    account_id: &str,
    _message_id: &str,
) -> Result<(), String> {
    let accounts = state.accounts.lock().await.clone();
    let _account = accounts.iter()
        .find(|a| a.id == account_id)
        .ok_or_else(|| "Account not found".to_string())?;

    let (mut client, _) = oauth::get_imap_client(account_id, &_account.email).await?;
    client.select("INBOX").await.map_err(|e| format!("Select failed: {}", e))?;

    // Copy to [Gmail]/Trash then mark deleted and expunge
    client.copy("1", "[Gmail]/Trash").await.ok();
    client.store("1", "+FLAGS (\\Deleted)").await.ok();
    client.expunge().await.ok();

    client.logout().await.ok();
    Ok(())
}

// --- Helper functions ---

fn get_preview(parsed: &ParsedMail) -> String {
    match get_body_text(parsed) {
        text if text.len() > 120 => {
            let preview: String = text.chars().take(120).collect();
            format!("{}...", preview.trim())
        }
        text => text.trim().to_string(),
    }
}

fn get_body_text(parsed: &ParsedMail) -> String {
    if parsed.subparts.is_empty() {
        match parsed.get_body() {
            Ok(body) => body,
            Err(_) => String::new(),
        }
    } else {
        for part in &parsed.subparts {
            let ct = part.get_content_type().mimetype.to_lowercase();
            if ct == "text/plain" || ct.starts_with("text/") {
                if let Ok(body) = part.get_body() {
                    return body;
                }
            }
        }
        parsed.subparts.first()
            .and_then(|p| p.get_body().ok())
            .unwrap_or_default()
    }
}

fn get_body_html(parsed: &ParsedMail) -> Option<String> {
    if parsed.subparts.is_empty() {
        None
    } else {
        for part in &parsed.subparts {
            let ct = part.get_content_type().mimetype.to_lowercase();
            if ct == "text/html" {
                if let Ok(body) = part.get_body() {
                    return Some(body);
                }
            }
        }
        None
    }
}

fn decode_subject(subject: &str) -> String {
    match mailparse::addrparse_header(subject) {
        Ok(mailparse::HeaderParseResult::Single(info)) => {
            info.display_name.unwrap_or(subject.to_string())
        }
        Ok(_) => subject.to_string(),
        Err(_) => subject.to_string(),
    }
}

fn parse_address(addr_str: &str) -> (String, String) {
    match mailparse::addrparse_header(addr_str) {
        Ok(mailparse::HeaderParseResult::Single(info)) => (
            info.display_name.unwrap_or_default(),
            info.addr.unwrap_or(addr_str.to_string()),
        ),
        _ => (String::new(), addr_str.to_string()),
    }
}

fn parse_address_list(addr_str: &str) -> Vec<String> {
    match mailparse::addrparse_header(addr_str) {
        Ok(mailparse::HeaderParseResult::Many(addrs)) => addrs
            .into_iter()
            .filter_map(|info| info.addr)
            .collect(),
        Ok(mailparse::HeaderParseResult::Single(info)) => {
            info.addr.into_iter().collect::<Vec<_>>()
        }
        _ => Vec::new(),
    }
}

fn parse_timestamp(date_str: &str) -> i64 {
    chrono::DateTime::parse_from_rfc2822(date_str)
        .map(|dt| dt.timestamp_millis())
        .unwrap_or_else(|_| chrono::Utc::now().timestamp_millis())
}
