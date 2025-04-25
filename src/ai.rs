use crate::config::Config;
use crate::errors::AppError;
use async_openai::{
    config::OpenAIConfig, types::{
        ChatCompletionRequestMessage, ChatCompletionRequestMessageContentPartImage, ChatCompletionRequestMessageContentPartText,
        ChatCompletionRequestSystemMessage, ChatCompletionRequestSystemMessageContent, ChatCompletionRequestUserMessage,
        ChatCompletionRequestUserMessageContent, ChatCompletionRequestUserMessageContentPart, CreateChatCompletionRequest,
        CreateChatCompletionResponse, ImageUrl
    }, Client
};
use std::sync::Arc;


#[derive(Clone)] // Client can be cloned
pub struct OpenAIClient {
    client: Client<OpenAIConfig>,
    system_prompt: String, // System prompt for OpenAI API
    // We might need config here for model choice, etc.
    // config: Arc<Config>, // If needed later
}

impl OpenAIClient {
    pub fn new(config: &Arc<Config>) -> Self {
        // New implicitly create the OpenAI client with default and env
        let openai_config = OpenAIConfig::new()
            .with_api_key(config.openai_api_key.clone()); // Use the API key from config
        let client = Client::with_config(openai_config); // Create the OpenAI client
        // If you need enterprise features or different host:
        // let client = Client::builder()
        //    .api_key(config.openai_api_key.clone())
        //    .base_url("...")
        //    .build();
        let system_prompt = config.openai_system_prompt.clone(); // Load system prompt from config
        tracing::info!("OpenAI client initialized with system prompt: {}", system_prompt); // Log the system prompt
        Self { client /*, config: config.clone() */, system_prompt } // Store the client and system prompt
    }

    // Send a chat completion request
    // prompt_text: the main text instruction from the user
    // image_data_base64: vector of Base64 encoded image strings + their mime types
    pub async fn create_chat_completion(
        &self,
        prompt_text: &str,
        image_data_base64: Vec<(String, String)>, // Vec<(base64_string, mime_type)>
    ) -> Result<CreateChatCompletionResponse, AppError> {

        // let model = if image_data_base64.is_empty() {
        //     // Use a text-only model if no images are provided
        //     "gpt-4-turbo-preview" // Or "gpt-4", or "gpt-3.5-turbo"
        // } else {
        //     // Use a vision model if images are provided
        //     "gpt-4-vision-preview"
        // };

        let model = "gpt-4.1";

        // Build the user message content
        let mut content_parts: Vec<ChatCompletionRequestUserMessageContentPart> = vec![];

        // Add the text part
        if !prompt_text.trim().is_empty() {
             content_parts.push(ChatCompletionRequestUserMessageContentPart::Text(
                ChatCompletionRequestMessageContentPartText {
                    text: prompt_text.to_string(),
                }
            ));
        }

        // Add image parts if any
        for (base64_string, mime_type) in image_data_base64 {
            // Ensure base64 string includes the data URL prefix
            let data_url = format!("data:{};base64,{}", mime_type, base64_string);
             content_parts.push(ChatCompletionRequestUserMessageContentPart::ImageUrl(
                ChatCompletionRequestMessageContentPartImage { image_url: ImageUrl {
                    url: data_url,
                    detail: None, // Use default detail level
                } }
            ));
        }

        // Handle case where only images are provided without text (add a default prompt)
        if content_parts.is_empty() {
             // This shouldn't happen if prompt_text is handled, but good defense
             content_parts.push(ChatCompletionRequestUserMessageContentPart::Text(
                ChatCompletionRequestMessageContentPartText {
                    text: "Analyze the provided image(s).".to_string(),
                }
            ));
        }


        let messages = vec![
            // System prompt
            ChatCompletionRequestMessage::System(
                ChatCompletionRequestSystemMessage {
                    content: ChatCompletionRequestSystemMessageContent::Text(self.system_prompt.to_string()),
                    name: Some("system".to_string())
                }
            ),
            // User message (can contain text and images)
            ChatCompletionRequestMessage::User(
                ChatCompletionRequestUserMessage {
                    content: ChatCompletionRequestUserMessageContent::Array(content_parts),
                    name: None
                }
            ),
            // Add assistant messages for conversation history if needed (not in this basic request)
        ];


        let request = CreateChatCompletionRequest {
            model: model.to_string(),
            messages,
            // max_tokens: Some(1000), // Limit response length (optional)
            // Other parameters like temperature, top_p, etc. can be added
            ..Default::default() // Use default for other fields
        };

        // Call the OpenAI API
        let response = self.client.chat().create(request).await?; // Propagates async_openai::Error

        Ok(response)
    }
}