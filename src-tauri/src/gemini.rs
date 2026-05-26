use serde::{Deserialize, Serialize};
use crate::GeminiDraftResponse;

const GEMINI_API_URL: &str = "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.0-flash:generateContent";

#[derive(Debug, Serialize)]
struct GeminiRequest {
    contents: Vec<Content>,
    system_instruction: Option<SystemInstruction>,
    generation_config: GenerationConfig,
}

#[derive(Debug, Serialize)]
struct Content {
    role: String,
    parts: Vec<Part>,
}

#[derive(Debug, Serialize)]
struct Part {
    text: String,
}

#[derive(Debug, Serialize)]
struct SystemInstruction {
    parts: Vec<Part>,
}

#[derive(Debug, Serialize)]
struct GenerationConfig {
    temperature: f32,
    max_output_tokens: u32,
}

#[derive(Debug, Deserialize)]
struct GeminiResponse {
    candidates: Vec<Candidate>,
}

#[derive(Debug, Deserialize)]
struct Candidate {
    content: ResponseContent,
}

#[derive(Debug, Deserialize)]
struct ResponseContent {
    parts: Vec<ResponsePart>,
}

#[derive(Debug, Deserialize)]
struct ResponsePart {
    text: String,
}

/// Get system instruction for a given writing style
fn get_style_instruction(style: &str) -> &'static str {
    match style.to_lowercase().as_str() {
        "professional" => "You are a professional email writer. Write in a polished, business-appropriate tone. Use proper grammar and structure. Be concise but thorough. Avoid slang or overly casual language.",
        "casual" => "You are a casual email writer. Write in a friendly, relaxed tone. Use conversational language. It's okay to use contractions and informal phrasing. Keep it natural.",
        "concise" => "You are a concise email writer. Write as briefly as possible while maintaining clarity. Use short sentences. Get straight to the point. Remove all filler words. Prefer bullet points when possible.",
        "formal" => "You are a formal email writer. Write in a dignified, official tone. Use complete sentences with proper punctuation. Maintain professional distance. Follow standard business correspondence conventions.",
        "friendly" => "You are a friendly email writer. Write in a warm, approachable tone. Be personable without being unprofessional. Use positive language. Include appropriate greetings and sign-offs.",
        _ => style, // Custom style description passed directly
    }
}

/// Draft an email using Google Gemini
pub async fn draft_email(
    api_key: &str,
    prompt: &str,
    style: &str,
    original_email: Option<&str>,
) -> Result<GeminiDraftResponse, String> {
    let style_instruction = get_style_instruction(style);

    // Build the user prompt
    let user_content = if let Some(original) = original_email {
        format!(
            "Original email:\n{}\n\n---\n\nUser's request: {}",
            original, prompt
        )
    } else {
        prompt.to_string()
    };

    let request_body = GeminiRequest {
        contents: vec![Content {
            role: "user".to_string(),
            parts: vec![Part { text: user_content }],
        }],
        system_instruction: Some(SystemInstruction {
            parts: vec![Part {
                text: format!(
                    "{}\n\nWrite ONLY the email body (no subject line, no 'Here is your draft' preamble). \
                     Output clean, ready-to-send email content.",
                    style_instruction
                ),
            }],
        }),
        generation_config: GenerationConfig {
            temperature: 0.7,
            max_output_tokens: 2048,
        },
    };

    let client = reqwest::Client::new();
    let url = format!("{}?key={}", GEMINI_API_URL, api_key);

    let resp = client
        .post(&url)
        .json(&request_body)
        .send()
        .await
        .map_err(|e| format!("Gemini API request failed: {}", e))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let error_text = resp.text().await.unwrap_or_default();
        return Err(format!("Gemini API error ({}): {}", status, error_text));
    }

    let gemini_resp: GeminiResponse = resp.json()
        .await
        .map_err(|e| format!("Failed to parse Gemini response: {}", e))?;

    // Extract the drafted text
    let draft = gemini_resp.candidates
        .first()
        .and_then(|c| c.content.parts.first())
        .map(|p| p.text.clone())
        .unwrap_or_else(|| "Failed to generate draft.".to_string());

    Ok(GeminiDraftResponse { draft })
}
