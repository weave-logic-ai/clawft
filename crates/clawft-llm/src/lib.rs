//! LLM provider abstraction for clawft.
//!
//! This crate provides a unified interface for calling LLM APIs via
//! OpenAI-compatible endpoints. It is a standalone library with no
//! dependencies on other clawft crates.
//!
//! # Architecture
//!
//! - [`Provider`] trait defines the chat completion interface
//! - [`OpenAiCompatProvider`] implements it for any OpenAI-compatible API
//! - [`ProviderRouter`] routes model names (e.g. "openai/gpt-4o") to providers
//! - [`ProviderConfig`] describes how to connect to a provider
//!
//! # Quick Start
//!
//! ```rust,ignore
//! use clawft_llm::{ProviderRouter, ChatRequest, ChatMessage};
//!
//! let router = ProviderRouter::with_builtins();
//! let (provider, model_name) = router.route("openai/gpt-4o").unwrap();
//!
//! let request = ChatRequest::new(model_name, vec![
//!     ChatMessage::system("You are a helpful assistant."),
//!     ChatMessage::user("What is Rust?"),
//! ]);
//!
//! let response = provider.complete(&request).await?;
//! println!("{}", response.choices[0].message.content);
//! ```

pub mod config;
pub mod error;
pub mod openai_compat;
pub mod provider;
pub mod router;
pub mod types;

pub use config::ProviderConfig;
pub use error::{ProviderError, Result};
pub use openai_compat::OpenAiCompatProvider;
pub use provider::Provider;
pub use router::ProviderRouter;
pub use types::{ChatMessage, ChatRequest, ChatResponse, ToolCall, Usage};
