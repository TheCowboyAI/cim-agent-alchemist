//! AI model provider integration

use crate::error::{AgentError, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use std::collections::HashMap;

/// Trait for AI model providers
#[async_trait]
pub trait ModelProvider: Send + Sync {
    /// Generate a response from the model
    async fn generate(&self, prompt: &str) -> Result<String>;

    /// Generate with conversation context
    async fn generate_with_context(
        &self,
        prompt: &str,
        context: &[Message],
    ) -> Result<String>;

    /// Check if the model is available
    async fn health_check(&self) -> Result<()>;

    /// Get model information
    fn model_info(&self) -> ModelInfo;
}

/// Request to send to the AI model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelRequest {
    /// The prompt or messages to send
    pub prompt: String,

    /// Conversation history for context
    pub history: Vec<Message>,

    /// System prompt to set behavior
    pub system_prompt: Option<String>,

    /// Generation parameters
    pub parameters: GenerationParameters,

    /// Additional metadata
    pub metadata: serde_json::Value,
}

/// Response from the AI model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelResponse {
    /// Generated text response
    pub content: String,

    /// Token usage information
    pub usage: TokenUsage,

    /// Model-specific metadata
    pub metadata: serde_json::Value,

    /// Time taken for generation
    pub duration: Duration,
}

/// Message in conversation history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// Role (user, assistant, system)
    pub role: String,

    /// Message content
    pub content: String,

    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Generation parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationParameters {
    /// Temperature for randomness (0.0 - 2.0)
    pub temperature: f32,

    /// Maximum tokens to generate
    pub max_tokens: usize,

    /// Top-p sampling
    pub top_p: Option<f32>,

    /// Top-k sampling
    pub top_k: Option<usize>,

    /// Stop sequences
    pub stop_sequences: Vec<String>,

    /// Frequency penalty
    pub frequency_penalty: Option<f32>,

    /// Presence penalty
    pub presence_penalty: Option<f32>,
}

impl Default for GenerationParameters {
    fn default() -> Self {
        Self {
            temperature: 0.7,
            max_tokens: 2048,
            top_p: Some(0.9),
            top_k: None,
            stop_sequences: vec![],
            frequency_penalty: None,
            presence_penalty: None,
        }
    }
}

/// Token usage information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    /// Tokens in the prompt
    pub prompt_tokens: usize,

    /// Tokens in the completion
    pub completion_tokens: usize,

    /// Total tokens used
    pub total_tokens: usize,
}

/// Model information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    /// Provider name
    pub provider: String,

    /// Model name
    pub model: String,

    /// Model version
    pub version: Option<String>,

    /// Model capabilities
    pub capabilities: ModelCapabilities,
}

/// Model capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelCapabilities {
    /// Maximum context length
    pub max_context_length: usize,

    /// Supports streaming responses
    pub streaming: bool,

    /// Supports function calling
    pub function_calling: bool,

    /// Supports vision/image inputs
    pub vision: bool,

    /// Supports embeddings
    pub embeddings: bool,
}

/// Ollama model provider
pub struct OllamaProvider {
    client: reqwest::Client,
    base_url: String,
    model: String,
    options: HashMap<String, serde_json::Value>,
}

impl OllamaProvider {
    /// Create a new Ollama provider
    pub fn new(base_url: String, model: String, options: HashMap<String, serde_json::Value>) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url,
            model,
            options,
        }
    }
}

#[derive(Serialize)]
struct OllamaGenerateRequest {
    model: String,
    prompt: String,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    context: Option<Vec<i32>>,
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    options: HashMap<String, serde_json::Value>,
}

#[derive(Deserialize)]
struct OllamaGenerateResponse {
    response: String,
    done: bool,
    #[serde(default)]
    context: Vec<i32>,
}

#[derive(Serialize)]
struct OllamaChatRequest {
    model: String,
    messages: Vec<OllamaMessage>,
    stream: bool,
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    options: HashMap<String, serde_json::Value>,
}

#[derive(Serialize, Deserialize)]
struct OllamaMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct OllamaChatResponse {
    message: OllamaMessage,
    done: bool,
}

#[async_trait]
impl ModelProvider for OllamaProvider {
    async fn generate(&self, prompt: &str) -> Result<String> {
        let request = OllamaGenerateRequest {
            model: self.model.clone(),
            prompt: prompt.to_string(),
            stream: false,
            context: None,
            options: self.options.clone(),
        };

        let response = self.client
            .post(format!("{}/api/generate", self.base_url))
            .json(&request)
            .send()
            .await
            .map_err(|e| AgentError::ModelError(format!("Failed to send request: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(AgentError::ModelError(format!(
                "Ollama API error: {} - {}",
                status, error_text
            )));
        }

        let ollama_response: OllamaGenerateResponse = response
            .json()
            .await
            .map_err(|e| AgentError::ModelError(format!("Failed to parse response: {}", e)))?;

        Ok(ollama_response.response)
    }

    async fn generate_with_context(
        &self,
        prompt: &str,
        context: &[Message],
    ) -> Result<String> {
        let mut messages: Vec<OllamaMessage> = context
            .iter()
            .map(|m| OllamaMessage {
                role: m.role.clone(),
                content: m.content.clone(),
            })
            .collect();

        messages.push(OllamaMessage {
            role: "user".to_string(),
            content: prompt.to_string(),
        });

        let request = OllamaChatRequest {
            model: self.model.clone(),
            messages,
            stream: false,
            options: self.options.clone(),
        };

        let response = self.client
            .post(format!("{}/api/chat", self.base_url))
            .json(&request)
            .send()
            .await
            .map_err(|e| AgentError::ModelError(format!("Failed to send request: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(AgentError::ModelError(format!(
                "Ollama API error: {} - {}",
                status, error_text
            )));
        }

        let ollama_response: OllamaChatResponse = response
            .json()
            .await
            .map_err(|e| AgentError::ModelError(format!("Failed to parse response: {}", e)))?;

        Ok(ollama_response.message.content)
    }

    async fn health_check(&self) -> Result<()> {
        let response = self.client
            .get(format!("{}/api/tags", self.base_url))
            .send()
            .await
            .map_err(|e| AgentError::ModelError(format!("Health check failed: {}", e)))?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(AgentError::ModelError(format!(
                "Ollama health check failed with status: {}",
                response.status()
            )))
        }
    }

    fn model_info(&self) -> ModelInfo {
        ModelInfo {
            provider: "Ollama".to_string(),
            model: self.model.clone(),
            version: None,
            capabilities: ModelCapabilities {
                max_context_length: 4096, // Typical for Vicuna
                streaming: true,
                function_calling: false,
                vision: false,
                embeddings: true,
            },
        }
    }
}

/// Mock provider for testing
pub struct MockProvider {
    response: String,
}

impl MockProvider {
    pub fn new(response: String) -> Self {
        Self { response }
    }
}

#[async_trait]
impl ModelProvider for MockProvider {
    async fn generate(&self, _prompt: &str) -> Result<String> {
        Ok(self.response.clone())
    }

    async fn generate_with_context(
        &self,
        _prompt: &str,
        _context: &[Message],
    ) -> Result<String> {
        Ok(self.response.clone())
    }

    async fn health_check(&self) -> Result<()> {
        Ok(())
    }
}

/// Factory function to create a model provider based on configuration
pub fn create_provider(config: &crate::config::ModelConfig) -> Result<Box<dyn ModelProvider>> {
    match config {
        crate::config::ModelConfig::Ollama {
            base_url,
            model,
            timeout,
            ..
        } => Ok(Box::new(OllamaProvider::new(
            base_url.clone(),
            model.clone(),
            HashMap::new(),
        ))),
        
        crate::config::ModelConfig::OpenAI { .. } => {
            Err(AgentError::Configuration(
                "OpenAI provider not yet implemented".to_string(),
            ))
        }
        
        crate::config::ModelConfig::Anthropic { .. } => {
            Err(AgentError::Configuration(
                "Anthropic provider not yet implemented".to_string(),
            ))
        }
    }
} 