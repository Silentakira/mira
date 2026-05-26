use crate::store::AppState;
use crate::ComposeRequest;
use crate::oauth;

/// Send an email via Gmail SMTP using XOAUTH2 authentication
pub async fn send_email(
    state: &AppState,
    request: ComposeRequest,
) -> Result<(), String> {
    let accounts = state.accounts.lock().await.clone();
    let account = accounts.iter()
        .find(|a| a.id == request.account_id)
        .ok_or_else(|| "Account not found".to_string())?;

    let access_token = oauth::get_valid_token(&request.account_id).await?;
    let auth_string = oauth::xoauth2_string(&account.email, &access_token);

    // Build the email
    let mut email_builder = lettre::Message::builder()
        .from(format!("{} <{}>", account.name, account.email)
            .parse()
            .map_err(|e| format!("Invalid from address: {}", e))?);

    // Add recipients
    for to_addr in &request.to {
        email_builder = email_builder.to(to_addr.parse()
            .map_err(|e| format!("Invalid to address: {}", e))?);
    }
    for cc_addr in &request.cc {
        email_builder = email_builder.cc(cc_addr.parse()
            .map_err(|e| format!("Invalid cc address: {}", e))?);
    }

    // Set subject and reply headers
    if let Some(ref reply_to) = request.in_reply_to {
        email_builder = email_builder.header(lettre::header::InReplyTo::new(
            reply_to.parse().map_err(|e| format!("Invalid In-Reply-To: {}", e))?
        ));
    }
    if let Some(ref refs) = request.references {
        email_builder = email_builder.header(lettre::header::References::new(
            refs.parse().map_err(|e| format!("Invalid References: {}", e))?
        ));
    }

    let email = email_builder
        .subject(&request.subject)
        .body(request.body.clone())
        .map_err(|e| format!("Failed to build email: {}", e))?;

    // Send using lettre's transport with XOAUTH2 credentials via TLS
    send_with_xoauth2(
        "smtp.gmail.com",
        587,
        &account.email,
        &auth_string,
        &email,
    ).await
}

/// Send email using lettre's SmtpTransport with XOAUTH2 authentication over STARTTLS
async fn send_with_xoauth2(
    hostname: &str,
    port: u16,
    username: &str,
    xoauth2_auth: &str,
    email: &lettre::Message,
) -> Result<(), String> {
    // Build TLS connector for STARTTLS
    let tls = async_native_tls::TlsConnector::builder()
        .build()
        .map_err(|e| format!("TLS error: {}", e))?;

    // Use lettre's transport with relay + custom credentials
    // lettre 0.11 supports credentials-based auth including XOAUTH2
    let creds = lettre::transport::smtp::authentication::Credentials::new(
        username.to_string(),
        xoauth2_auth.to_string(),
    );

    let mailer = lettre::SmtpTransport::relay(hostname)
        .map_err(|e| format!("SMTP relay error: {}", e))?
        .port(port)
        .credentials(creds)
        .build();

    // Send the email (lettre 0.11 uses sync transport internally with tokio runtime)
    match mailer.send(email) {
        Ok(_) => {
            log::info!("Email sent successfully");
            Ok(())
        }
        Err(e) => Err(format!("Failed to send email: {}", e)),
    }
}
