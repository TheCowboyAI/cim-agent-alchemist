//! Error types for the Alchemist agent

use thiserror::Error;

/// Result type alias for agent operations
pub type Result<T> = std::result::Result<T, AgentError>;

/// Main error type for the Alchemist agent
#[derive(Debug, Error)]
pub enum AgentError {
    /// Configuration errors
    #[error("Configuration error: {0}")]
    Configuration(String),

    /// NATS connection or messaging errors
    #[error("NATS error: {0}")]
    Nats(#[from] async_nats::Error),

    /// Model provider errors
    #[error("Model provider error: {0}")]
    ModelProvider(String),

    /// Domain operation errors
    #[error("Domain error: {domain} - {message}")]
    Domain { domain: String, message: String },

    /// Dialog management errors
    #[error("Dialog error: {0}")]
    Dialog(String),

    /// Identity verification errors
    #[error("Identity error: {0}")]
    Identity(String),

    /// Graph operation errors
    #[error("Graph error: {0}")]
    Graph(String),

    /// Workflow execution errors
    #[error("Workflow error: {0}")]
    Workflow(String),

    /// Serialization/deserialization errors
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Network request errors
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    /// Timeout errors
    #[error("Operation timed out: {0}")]
    Timeout(String),

    /// Resource not found
    #[error("Resource not found: {0}")]
    NotFound(String),

    /// Permission denied
    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    /// Service unavailable
    #[error("Service unavailable: {0}")]
    ServiceUnavailable(String),

    /// Generic internal error
    #[error("Internal error: {0}")]
    Internal(String),
}

impl AgentError {
    /// Create a domain error with specific domain context
    pub fn domain(domain: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Domain {
            domain: domain.into(),
            message: message.into(),
        }
    }

    /// Check if the error is retryable
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::Nats(_) | Self::Network(_) | Self::Timeout(_) | Self::ServiceUnavailable(_)
        )
    }

    /// Get the error severity for logging
    pub fn severity(&self) -> &'static str {
        match self {
            Self::Configuration(_) | Self::PermissionDenied(_) => "critical",
            Self::Domain { .. } | Self::Dialog(_) | Self::Identity(_) => "error",
            Self::Nats(_) | Self::Network(_) | Self::ServiceUnavailable(_) => "warning",
            _ => "info",
        }
    }
} 