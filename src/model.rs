//! AI model provider integration

use crate::error::{AgentError, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Trait for AI model providers
#[async_trait]
pub trait ModelProvider: Send + Sync {
    /// Send a request to the model and get a response
    async fn generate(&self, request: ModelRequest) -> Result<ModelResponse>;

    /// Check if the model is available and healthy
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

/// Ollama model provider implementation
pub struct OllamaProvider {
    client: reqwest::Client,
    base_url: String,
    model: String,
    timeout: Duration,
}

impl OllamaProvider {
    /// Create a new Ollama provider
    pub fn new(base_url: String, model: String, timeout: Duration) -> Self {
        let client = reqwest::Client::builder()
            .timeout(timeout)
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            base_url,
            model,
            timeout,
        }
    }
}

#[async_trait]
impl ModelProvider for OllamaProvider {
    async fn generate(&self, request: ModelRequest) -> Result<ModelResponse> {
        let start = std::time::Instant::now();

        // Build Ollama API request
        let ollama_request = serde_json::json!({
            "model": self.model,
            "prompt": request.prompt,
            "system": request.system_prompt,
            "context": self.build_context(&request.history),
            "options": {
                "temperature": request.parameters.temperature,
                "num_predict": request.parameters.max_tokens,
                "top_p": request.parameters.top_p,
                "top_k": request.parameters.top_k,
                "stop": request.parameters.stop_sequences,
            }
        });

        let response = self
            .client
            .post(format!("{}/api/generate", self.base_url))
            .json(&ollama_request)
            .send()
            .await
            .map_err(|e| AgentError::ModelProvider(format!("Ollama request failed: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(AgentError::ModelProvider(format!(
                "Ollama returned error: {}",
                error_text
            )));
        }

        let ollama_response: serde_json::Value = response
            .json()
            .await
            .map_err(|e| AgentError::ModelProvider(format!("Failed to parse response: {}", e)))?;

        // Extract response data
        let content = ollama_response["response"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let prompt_tokens = ollama_response["prompt_eval_count"].as_u64().unwrap_or(0) as usize;
        let completion_tokens = ollama_response["eval_count"].as_u64().unwrap_or(0) as usize;

        Ok(ModelResponse {
            content,
            usage: TokenUsage {
                prompt_tokens,
                completion_tokens,
                total_tokens: prompt_tokens + completion_tokens,
            },
            metadata: ollama_response,
            duration: start.elapsed(),
        })
    }

    async fn health_check(&self) -> Result<()> {
        let response = self
            .client
            .get(format!("{}/api/tags", self.base_url))
            .send()
            .await
            .map_err(|e| AgentError::ServiceUnavailable(format!("Ollama health check failed: {}", e)))?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(AgentError::ServiceUnavailable(
                "Ollama service is not healthy".to_string(),
            ))
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

impl OllamaProvider {
    /// Build context from conversation history
    fn build_context(&self, history: &[Message]) -> Vec<serde_json::Value> {
        history
            .iter()
            .map(|msg| {
                serde_json::json!({
                    "role": msg.role,
                    "content": msg.content,
                })
            })
            .collect()
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
            *timeout,
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