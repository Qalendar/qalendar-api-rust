use async_openai::{
    config,
    types::{
        ChatCompletionResponseMessage,
    },
};
use axum::{
    extract::{State, Multipart},
    Json,
};
use bytes::Bytes; // For handling file bytes
use mime::Mime; // For MIME type handling
use base64::Engine as Base64Engine; // For Base64 encoding
use base64::engine::general_purpose::STANDARD as Base64Standard; // Standard Base64 alphabet
use crate::{
    ai::OpenAIClient, errors::AppError, middleware::auth::AuthenticatedUser, config::Config, AppState // Import our OpenAI client wrapper
};
use serde_json::json; // For simple JSON response


// Define the system prompt here or load from config (default behavior)
// const SYSTEM_PROMPT: &str = "Act as a helpful calendar assistant. Provide concise answers.";

// --- POST /api/me/ai-assistant handler ---
pub async fn handle_ai_prompt(
    State(state): State<AppState>,
    AuthenticatedUser { user_id }: AuthenticatedUser, // Ensure user is authenticated
    mut multipart: Multipart, // Extract multipart data
) -> Result<Json<serde_json::Value>, AppError> { // Return JSON response (AI text)

    let mut prompt_text: Option<String> = None;
    let mut image_data: Vec<(String, String)> = Vec::new(); // Vec<(base64_string, mime_type)>

    // Process multipart fields
    while let Some(field) = multipart.next_field().await.map_err(|e| AppError::InvalidMultipartData(format!("Failed to read multipart field: {}", e)))? {
        let name = field.name().ok_or(AppError::InvalidMultipartData("Multipart field missing name".to_string()))?;
        let content_type = field.content_type().map(|ct| ct.parse::<Mime>().ok()).flatten(); // Parse MIME type

        match name {
            "prompt" => {
                // Expecting text field for the prompt
                let text = field.text().await.map_err(|e| AppError::FileUploadError(format!("Failed to read prompt text: {}", e)))?;
                prompt_text = Some(text);
            }
            "files" => {
                // Expecting file field for images
                let file_name_str = field.file_name()
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "unknown".to_string());
                let bytes = field.bytes().await.map_err(|e| AppError::FileUploadError(format!("Failed to read file bytes: {}", e)))?;
                let file_size = bytes.len();
                tracing::info!("Received file '{}' with size {}", file_name_str, file_size);

                // Validate file type and size (Add limits!)
                // Example limits: 10MB per file, only allow image types
                 let mime_type = content_type.ok_or(AppError::FileUploadError(format!("File '{}' missing content type", file_name_str)))?;
                 if mime_type.type_() != mime::IMAGE {
                     tracing::warn!("Unsupported file type uploaded: {}", mime_type);
                     return Err(AppError::FileUploadError(format!("Unsupported file type: {}", mime_type)));
                 }
                 // Further check specific image subtypes if needed (jpeg, png, gif, webp)
                 // if mime_type.subtype() != mime::JPEG && mime_type.subtype() != mime::PNG { ... }

                 const MAX_FILE_SIZE: usize = 10 * 1024 * 1024; // 10MB
                 if file_size > MAX_FILE_SIZE {
                      tracing::warn!("File size exceeds limit: {} bytes", file_size);
                      return Err(AppError::FileUploadError(format!("File size exceeds limit (max {} bytes)", MAX_FILE_SIZE)));
                 }
                 // Consider total request size limit as well

                // Encode image bytes to Base64
                let base64_string = Base64Standard.encode(&bytes);

                image_data.push((base64_string, mime_type.to_string())); // Store base64 and mime type
            }
            _ => {
                // Ignore unexpected fields or return an error
                tracing::warn!("Ignoring unexpected multipart field: {}", name);
                // return Err(AppError::InvalidMultipartData(format!("Unexpected field: {}", name)));
            }
        }
    }

    // Ensure at least a prompt text was provided if no files
    let prompt_text = prompt_text.filter(|s| !s.trim().is_empty()); // Clear whitespace-only prompt
    if prompt_text.is_none() && image_data.is_empty() {
        return Err(AppError::InvalidMultipartData("Request must include 'prompt' text or 'files'".to_string()));
    }
    let prompt_text = prompt_text.unwrap_or_default(); // Use empty string if none provided, but files are present


    // Get the OpenAI client from state
    let openai_client = &state.openai_client; // Assuming openai_client is in AppState

    // Define system prompt (or load from config if implemented)
    // let system_prompt = "Act as a helpful calendar assistant. Provide concise answers.";
    // If loading from config: let system_prompt = &state.config.openai_system_prompt;

    // Call the OpenAI API client
    let response = openai_client.create_chat_completion(
        &prompt_text,
        image_data, // Pass the image data vector
    ).await?; // Propagates AppError::OpenAIError

    // Extract the text content from the response
    // Assuming a single choice and a single text message from the assistant
    let ai_text = response.choices
        .into_iter()
        .next() // Get the first choice
        .and_then(|choice| choice.message.content) // Get the message content
        .unwrap_or_else(|| "AI returned no text response.".to_string()); // Default message if response is empty

    // Return the AI's text response as JSON
    Ok(Json(json!({ "response": ai_text })))
}