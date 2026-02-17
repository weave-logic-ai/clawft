//! The core [`Provider`] trait for LLM chat completions.
//!
//! All LLM providers implement this trait, which provides a single
//! `complete` method for executing chat completion requests.

use async_trait::async_trait;

use crate::error::Result;
use crate::types::{ChatRequest, ChatResponse};

/// A provider that can execute chat completion requests.
///
/// Implementations handle the protocol details for a specific LLM API
/// (authentication, request formatting, response parsing). The main
/// implementation is [`OpenAiCompatProvider`](crate::openai_compat::OpenAiCompatProvider),
/// which works with any OpenAI-compatible endpoint.
///
/// # Example
///
/// ```rust,ignore
/// use clawft_llm::{Provider, ChatRequest, ChatMessage};
///
/// async fn call_llm(provider: &dyn Provider) -> clawft_llm::Result<String> {
///     let request = ChatRequest::new("gpt-4o", vec![
///         ChatMessage::user("What is 2+2?"),
///     ]);
///     let response = provider.complete(&request).await?;
///     Ok(response.choices[0].message.content.clone())
/// }
/// ```
#[async_trait]
pub trait Provider: Send + Sync {
    /// Returns the provider name (e.g. "openai", "anthropic", "groq").
    fn name(&self) -> &str;

    /// Execute a chat completion request and return the response.
    ///
    /// # Errors
    ///
    /// Returns [`ProviderError`](crate::error::ProviderError) if the request
    /// fails due to network issues, authentication problems, rate limiting,
    /// or invalid responses.
    async fn complete(&self, request: &ChatRequest) -> Result<ChatResponse>;
}
