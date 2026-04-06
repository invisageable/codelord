use codelord_protocol::automata::dto::{
  ChatCompletionRequest, ChatCompletionResponse,
};
use codelord_protocol::automata::model::Message;

use reqwest::Client;
use secrecy::{ExposeSecret, SecretString};

use std::time::Duration;

/// The OpenAI chat completions API URL.
const OPENAI_CHAT_URL: &str = "https://api.openai.com/v1/chat/completions";

/// OpenAI client that makes direct HTTP calls to OpenAI API.
///
/// This is the implementation detail that talks to the external service.
pub struct OpenAIClient {
  api_client: Client,
  api_key: SecretString,
}

impl OpenAIClient {
  pub fn new(api_key: SecretString) -> Self {
    let api_client = Client::builder()
      .danger_accept_invalid_certs(false) // Explicit TLS verification
      .use_native_tls()
      .connect_timeout(Duration::from_secs(1))
      .timeout(Duration::from_secs(2))
      .build()
      .expect("Failed to create HTTP client");

    Self {
      api_client,
      api_key,
    }
  }

  /// Complete a chat request by calling OpenAI's API directly.
  pub async fn complete(
    &self,
    request: ChatCompletionRequest,
  ) -> Result<ChatCompletionResponse, Box<dyn std::error::Error + Send + Sync>>
  {
    let response = self
      .api_client
      .post(OPENAI_CHAT_URL)
      .header(
        "Authorization",
        format!("Bearer {}", self.api_key.expose_secret()),
      )
      .json(&request)
      .send()
      .await?;

    if !response.status().is_success() {
      let status = response.status();
      let error_text = response.text().await?;

      return Err(format!("[OpenAI] API error {status}: {error_text}").into());
    }

    let response_text = response.text().await?;

    tracing::debug!("[OpenAI] raw response: {response_text}");

    let response_body =
      sonic_rs::from_str::<ChatCompletionResponse>(&response_text)?;

    Ok(response_body)
  }

  /// Helper method for simple prompt-based completions.
  #[allow(dead_code)]
  pub async fn complete_with_messages(
    &self,
    system_prompt: &str,
    user_message: &str,
    model: &str,
    temperature: f32,
    max_tokens: u32,
  ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let request = ChatCompletionRequest {
      model: model.into(),
      messages: vec![
        Message {
          role: "system".into(),
          content: system_prompt.into(),
        },
        Message {
          role: "user".into(),
          content: user_message.into(),
        },
      ],
      temperature,
      max_tokens,
    };

    let response = self.complete(request).await?;

    Ok(response.choices[0].message.content.clone())
  }
}
