use crate::store::{self, AppState};
use crate::EmailAccount;

// Google OAuth2 configuration — loaded from env vars at runtime
// Set MIRA_GOOGLE_CLIENT_ID and MIRA_GOOGLE_CLIENT_SECRET before running,
// or configure via Settings panel on first launch.
fn client_id() -> String {
    std::env::var("MIRA_GOOGLE_CLIENT_ID")
        .unwrap_or_else(|_| "YOUR_GOOGLE_CLIENT_ID.apps.googleusercontent.com".to_string())
}

fn client_secret() -> String {
    std::env::var("MIRA_GOOGLE_CLIENT_SECRET")
        .unwrap_or_else(|_| "YOUR_GOOGLE_CLIENT_SECRET".to_string())
}

const REDIRECT_URI: &str = "http://127.0.0.1:1420/auth/callback";

/// Scopes needed for Gmail access via IMAP + SMTP + profile
const SCOPES: &[&str] = &[
    "https://mail.google.com/",       // Full IMAP/SMTP access
    "https://www.googleapis.com/auth/userinfo.email",
    "https://www.googleapis.com/auth/userinfo.profile",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OAuthToken {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: Option<u64>,
    token_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct UserInfo {
    email: String,
    name: String,
    picture: Option<String>,
}

/// Generate the Google OAuth2 authorization URL
pub fn get_auth_url(state: &str) -> String {
    let scopes_encoded = urlencoding::encode(&SCOPES.join(" "));
    format!(
        "https://accounts.google.com/o/oauth2/v2/auth?\
         client_id={}&\
         redirect_uri={}&\
         response_type=code&\
         scope={}&\
         access_type=offline&\
         prompt=consent&\
         state={}",
        urlencoding::encode(client_id().as_str()),
        urlencoding::encode(REDIRECT_URI),
        scopes_encoded,
        urlencoding::encode(state),
    )
}

/// Exchange authorization code for tokens
async fn exchange_code(code: &str) -> Result<OAuthToken, String> {
    let client = reqwest::Client::new();
    let params = [
        ("code", code),
        ("client_id", client_id().as_str()),
        ("client_secret", client_secret().as_str()),
        ("redirect_uri", REDIRECT_URI),
        ("grant_type", "authorization_code"),
    ];

    let resp = client
        .post("https://oauth2.googleapis.com/token")
        .form(&params)
        .send()
        .await
        .map_err(|e| format!("Token exchange request failed: {}", e))?;

    if !resp.status().is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("Token exchange failed: {} - {}", resp.status(), body));
    }

    resp.json::<OAuthToken>()
        .await
        .map_err(|e| format!("Failed to parse token response: {}", e))
}

/// Get user info from Google using the access token
async fn get_user_info(access_token: &str) -> Result<UserInfo, String> {
    let client = reqwest::Client::new();
    let resp = client
        .get("https://www.googleapis.com/oauth2/v2/userinfo")
        .header("Authorization", format!("Bearer {}", access_token))
        .send()
        .await
        .map_err(|e| format!("User info request failed: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("User info failed: {}", resp.status()));
    }

    resp.json::<UserInfo>()
        .await
        .map_err(|e| format!("Failed to parse user info: {}", e))
}

/// Refresh an expired access token using the refresh token
pub async fn refresh_access_token(account_id: &str) -> Result<String, String> {
    let refresh_token_data = store::load_token(account_id)?
        .ok_or_else(|| "No refresh token stored".to_string())?;

    // Parse stored token data to extract refresh token
    let token_data: OAuthToken = serde_json::from_str(&refresh_token_data)
        .map_err(|e| format!("Failed to parse stored token: {}", e))?;

    let refresh_token = token_data.refresh_token
        .ok_or_else(|| "No refresh token available".to_string())?;

    let client = reqwest::Client::new();
    let params = [
        ("refresh_token", refresh_token.as_str()),
        ("client_id", client_id().as_str()),
        ("client_secret", client_secret().as_str()),
        ("grant_type", "refresh_token"),
    ];

    let resp = client
        .post("https://oauth2.googleapis.com/token")
        .form(&params)
        .send()
        .await
        .map_err(|e| format!("Token refresh request failed: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("Token refresh failed: {}", resp.status()));
    }

    let new_token: OAuthToken = resp.json()
        .await
        .map_err(|e| format!("Failed to parse refreshed token: {}", e))?;

    // Merge with existing refresh token (Google may not return one on refresh)
    let merged_token = OAuthToken {
        refresh_token: Some(new_token.refresh_token.unwrap_or_else(|| {
            token_data.refresh_token.unwrap_or_default()
        })),
        ..new_token
    };

    // Save updated token
    let token_json = serde_json::to_string(&merged_token)
        .map_err(|e| format!("Failed to serialize token: {}", e))?;
    store::save_token(account_id, &token_json)?;

    Ok(merged_token.access_token)
}

/// Get valid access token for an account, refreshing if necessary
pub async fn get_valid_token(account_id: &str) -> Result<String, String> {
    // Try loading cached token first
    if let Some(token_data) = store::load_token(account_id)? {
        let token: OAuthToken = serde_json::from_str(&token_data)
            .map_err(|e| format!("Failed to parse token: {}", e))?;
        return Ok(token.access_token);
    }

    // Otherwise refresh
    refresh_access_token(account_id).await
}

/// Generate XOAUTH2 string for IMAP/SMTP authentication
pub fn xoauth2_string(email: &str, access_token: &str) -> String {
    format!(
        "user={}\x01auth=Bearer {}\x01\x01",
        email, access_token
    )
}

/// Start the OAuth2 flow — opens browser, catches callback on loopback
pub async fn start_oauth_flow(state: &AppState) -> Result<EmailAccount, String> {
    let state_param = uuid::Uuid::new_v4().to_string();
    let auth_url = get_auth_url(&state_param);

    log::info!("Opening OAuth URL: {}", auth_url);

    // Start local callback server BEFORE opening browser
    let server_handle = start_callback_server();

    // Open browser for user to authenticate
    webbrowser::open(&auth_url)
        .map_err(|e| format!("Failed to open browser: {}", e))?;

    // Wait for the callback (with timeout)
    let code = tokio::time::timeout(
        std::time::Duration::from_secs(300), // 5 minute timeout
        wait_for_callback(server_handle),
    ).await
    .map_err(|_| "Timed out waiting for authentication. Please try again.".to_string())??;

    // Exchange code for tokens
    let token = exchange_code(&code).await?;

    // Get user info
    let user_info = get_user_info(&token.access_token).await?;

    // Create account
    let account_id = uuid::Uuid::new_v4().to_string();
    let account = EmailAccount {
        id: account_id.clone(),
        email: user_info.email.clone(),
        name: user_info.name,
        is_connected: true,
    };

    // Save token to keychain
    let token_json = serde_json::to_string(&token)
        .map_err(|e| format!("Failed to serialize token: {}", e))?;
    store::save_token(&account_id, &token_json)?;

    // Save account
    store::save_account(state, &account).await?;

    Ok(account)
}

/// Complete OAuth flow with a manually-entered authorization code
/// (Fallback if the loopback callback doesn't fire)
pub async fn complete_with_code(state: &AppState, code: String) -> Result<EmailAccount, String> {
    log::info!("Completing OAuth with manual code");

    // Exchange code for tokens
    let token = exchange_code(&code).await?;

    // Get user info
    let user_info = get_user_info(&token.access_token).await?;

    // Create account
    let account_id = uuid::Uuid::new_v4().to_string();
    let account = EmailAccount {
        id: account_id.clone(),
        email: user_info.email.clone(),
        name: user_info.name,
        is_connected: true,
    };

    // Save token to keychain
    let token_json = serde_json::to_string(&token)
        .map_err(|e| format!("Failed to serialize token: {}", e))?;
    store::save_token(&account_id, &token_json)?;

    // Save account
    store::save_account(state, &account).await?;

    Ok(account)
}

/// Start the local HTTP server to catch the OAuth callback
fn start_callback_server() -> tokio::sync::mpsc::Sender<Result<String, String>> {
    use tokio::sync::mpsc;
    let (tx, mut rx) = mpsc::channel::<Result<String, String>>(1);

    tokio::spawn(async move {
        use tokio::net::TcpListener;

        match TcpListener::bind("127.0.0.1:1420").await {
            Ok(listener) => {
                log::info!("OAuth callback server listening on port 1420...");

                if let Ok((mut socket, _addr)) = listener.accept().await {
                    use tokio::io::AsyncReadExt;
                    let mut buf = [0u8; 4096];
                    if let Ok(n) = socket.read(&mut buf).await {
                        let request = String::from_utf8_lossy(&buf[..n]);
                        if let Some(code) = extract_code_from_request(&request) {
                            let response = "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\n\
                                <html><body style='display:flex;align-items:center;justify-content:center;height:100vh;margin:0;font-family:-apple-system,sans-serif;background:#0e0e0e;color:#e5e5e7'>\
                                <div style='text-align:center;padding:40px'>\
                                <h1 style='font-size:24px;margin-bottom:12px'>Authentication Successful</h1>\
                                <p style='color:#8e8e93'>You can close this window and return to Mira.</p>\
                                </div></body></html>";
                            let _ = socket.write_all(response.as_bytes()).await;
                            let _ = tx.send(Ok(code));
                            return;
                        }
                    }
                }
                let _ = tx.send(Err("No valid callback received".to_string()));
            }
            Err(e) => {
                log::warn!("Failed to bind OAuth callback server: {}. User will need to paste code manually.", e);
                // Don't error — user can paste code as fallback
            }
        }
    });

    tx
}

/// Wait for the callback server to receive the authorization code
async fn wait_for_callback(_server_handle: tokio::sync::mpsc::Sender<Result<String, String>>) -> Result<String, String> {
    use tokio::sync::mpsc;

    // We need a receiver — but since we passed the sender out,
    // we create a new oneshot channel approach:
    // The server sends through its own channel, but we need a way to wait.
    // Simpler approach: just re-listen ourselves here.

    use tokio::net::TcpListener;

    let listener = TcpListener::bind("127.0.0.1:1420")
        .await
        .map_err(|e| format!("Failed to bind to port 1420: {}", e))?;

    log::info!("Waiting for OAuth callback on port 1420...");

    if let Ok((mut socket, _)) = listener.accept().await {
        use tokio::io::AsyncReadExt;
        let mut buf = [0u8; 4096];
        if let Ok(n) = socket.read(&mut buf).await {
            let request = String::from_utf8_lossy(&buf[..n]);
            if let Some(code) = extract_code_from_request(&request) {
                let response = "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\n\
                    <html><body style='background:#0e0e0e;color:#e5e5e7;text-align:center;padding:40px'>\
                    <h1>Mira Authenticated</h1><p>You can close this tab.</p></body></html>";
                let _ = socket.write_all(response.as_bytes()).await;
                return Ok(code);
            }
        }
    }

    Err("No callback received. Please enter the authorization code manually.".to_string())
}

fn extract_code_from_request(request: &str) -> Option<String> {
    // Parse GET /auth/callback?code=...&state=...
    for line in request.lines() {
        if line.starts_with("GET ") {
            if let Some(query_start) = line.find('?') {
                let query = &line[query_start + 1..];
                for part in query.split('&') {
                    if let Some(code) = part.strip_prefix("code=") {
                        return Some(urlencoding::decode(code).unwrap_or(code.to_string()).into_owned());
                    }
                }
            }
        }
    }
    None
}

/// Get an IMAP-compatible authenticated session using XOAUTH2
pub async fn get_imap_client(
    account_id: &str,
    email: &str,
) -> Result<(async_imap::Client<async_native_tls::TlsStream<std::net::TcpStream>>, String), String> {
    let access_token = get_valid_token(account_id).await?;
    let auth_string = xoauth2_string(email, &access_token);

    // Connect to Gmail IMAP over TLS
    let tls = async_native_tls::TlsConnector::builder()
        .build()
        .map_err(|e| format!("TLS error: {}", e))?;

    let client = async_imap::connect(
        "imap.gmail.com:993",
        "imap.gmail.com",
        tls,
    ).await
    .map_err(|e| format!("IMAP connect failed: {}", e))?;

    // Authenticate with XOAUTH2
    let auth_bytes = auth_string.as_bytes().to_vec();
    client.authenticate("XOAUTH2", &auth_bytes[..]).await
        .map_err(|e| format!("XOAUTH2 auth failed: {}", e))?;

    Ok((client, access_token))
}
