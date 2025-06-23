//! Configuration types for the Alchemist agent

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Main configuration for the Alchemist agent
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AgentConfig {
    /// Agent identity configuration
    pub identity: IdentityConfig,
    
    /// Model provider configuration
    pub model: ModelConfig,
    
    /// NATS messaging configuration
    pub nats: NatsConfig,
    
    /// Service configuration
    pub service: ServiceConfig,
    
    /// Domain-specific configurations
    pub domains: DomainConfigs,
}

/// Identity configuration for the agent
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct IdentityConfig {
    /// Unique agent ID
    pub agent_id: String,
    
    /// Display name
    pub name: String,
    
    /// Description of agent capabilities
    pub description: String,
    
    /// Agent version
    pub version: String,
    
    /// Organization or owner
    pub organization: String,
}

/// Model provider configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "provider")]
pub enum ModelConfig {
    /// Ollama configuration
    Ollama {
        /// Base URL for Ollama API
        base_url: String,
        /// Model name (e.g., "vicuna", "llama2")
        model: String,
        /// Request timeout
        #[serde(with = "humantime_serde")]
        timeout: Duration,
        /// Temperature for generation
        temperature: f32,
        /// Maximum tokens to generate
        max_tokens: usize,
    },
    
    /// OpenAI configuration
    OpenAI {
        /// API key
        api_key: String,
        /// Model name (e.g., "gpt-4")
        model: String,
        /// Organization ID (optional)
        organization: Option<String>,
        /// Request timeout
        #[serde(with = "humantime_serde")]
        timeout: Duration,
    },
    
    /// Anthropic configuration
    Anthropic {
        /// API key
        api_key: String,
        /// Model name (e.g., "claude-3")
        model: String,
        /// Request timeout
        #[serde(with = "humantime_serde")]
        timeout: Duration,
    },
}

impl ModelConfig {
    /// Get the model name being used
    pub fn model_name(&self) -> String {
        match self {
            ModelConfig::Ollama { model, .. } => model.clone(),
            ModelConfig::OpenAI { model, .. } => model.clone(),
            ModelConfig::Anthropic { model, .. } => model.clone(),
        }
    }
}

/// NATS messaging configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NatsConfig {
    /// NATS server URLs
    pub servers: Vec<String>,
    
    /// Subject prefix for this agent
    pub subject_prefix: String,
    
    /// Authentication configuration
    pub auth: Option<NatsAuth>,
    
    /// Connection retry configuration
    pub retry: RetryConfig,
    
    /// JetStream configuration
    pub jetstream: Option<JetStreamConfig>,
}

/// NATS authentication options
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type")]
pub enum NatsAuth {
    /// Token authentication
    Token { token: String },
    
    /// Username/password authentication
    UserPassword { username: String, password: String },
    
    /// JWT authentication
    Jwt { jwt: String, seed: String },
    
    /// TLS certificate authentication
    Tls { cert_path: String, key_path: String },
}

/// Retry configuration for connections
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RetryConfig {
    /// Maximum number of retry attempts
    pub max_attempts: u32,
    
    /// Initial retry delay
    #[serde(with = "humantime_serde")]
    pub initial_delay: Duration,
    
    /// Maximum retry delay
    #[serde(with = "humantime_serde")]
    pub max_delay: Duration,
    
    /// Exponential backoff multiplier
    pub multiplier: f64,
}

/// JetStream configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct JetStreamConfig {
    /// Stream name for agent events
    pub stream_name: String,
    
    /// Durable consumer name
    pub consumer_name: String,
    
    /// Enable message deduplication
    pub dedupe_window: Option<Duration>,
}

/// Service configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServiceConfig {
    /// Service bind address
    pub bind_address: String,
    
    /// Service port
    pub port: u16,
    
    /// Health check interval
    #[serde(with = "humantime_serde")]
    pub health_check_interval: Duration,
    
    /// Metrics configuration
    pub metrics: MetricsConfig,
    
    /// Logging configuration
    pub logging: LoggingConfig,
}

/// Metrics configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MetricsConfig {
    /// Enable metrics collection
    pub enabled: bool,
    
    /// Metrics endpoint path
    pub endpoint: String,
    
    /// Prometheus push gateway URL (optional)
    pub push_gateway: Option<String>,
}

/// Logging configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LoggingConfig {
    /// Log level (trace, debug, info, warn, error)
    pub level: String,
    
    /// Log format (json, pretty, compact)
    pub format: String,
    
    /// Enable ANSI colors
    pub colors: bool,
    
    /// Log file path (optional)
    pub file: Option<String>,
}

/// Domain-specific configurations
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DomainConfigs {
    /// Dialog domain configuration
    pub dialog: DialogConfig,
    
    /// Graph domain configuration
    pub graph: GraphConfig,
    
    /// Workflow domain configuration
    pub workflow: WorkflowConfig,
}

/// Dialog domain configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DialogConfig {
    /// Maximum conversation history to maintain
    pub max_history: usize,
    
    /// Context window size
    pub context_window: usize,
    
    /// Session timeout
    #[serde(with = "humantime_serde")]
    pub session_timeout: Duration,
}

/// Graph domain configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GraphConfig {
    /// Maximum nodes in visualization
    pub max_nodes: usize,
    
    /// Enable auto-layout
    pub auto_layout: bool,
    
    /// Default layout algorithm
    pub layout_algorithm: String,
}

/// Workflow domain configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WorkflowConfig {
    /// Maximum concurrent workflows
    pub max_concurrent: usize,
    
    /// Workflow timeout
    #[serde(with = "humantime_serde")]
    pub timeout: Duration,
    
    /// Enable workflow persistence
    pub persist: bool,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            identity: IdentityConfig {
                agent_id: uuid::Uuid::new_v4().to_string(),
                name: "Alchemist".to_string(),
                description: "CIM Architecture Assistant".to_string(),
                version: crate::VERSION.to_string(),
                organization: "CIM".to_string(),
            },
            model: ModelConfig::Ollama {
                base_url: "http://localhost:11434".to_string(),
                model: "vicuna".to_string(),
                timeout: Duration::from_secs(30),
                temperature: 0.7,
                max_tokens: 2048,
            },
            nats: NatsConfig {
                servers: vec!["nats://localhost:4222".to_string()],
                subject_prefix: "cim.agent.alchemist".to_string(),
                auth: None,
                retry: RetryConfig {
                    max_attempts: 5,
                    initial_delay: Duration::from_millis(100),
                    max_delay: Duration::from_secs(30),
                    multiplier: 2.0,
                },
                jetstream: Some(JetStreamConfig {
                    stream_name: "ALCHEMIST_EVENTS".to_string(),
                    consumer_name: "alchemist-consumer".to_string(),
                    dedupe_window: Some(Duration::from_secs(120)),
                }),
            },
            service: ServiceConfig {
                bind_address: "0.0.0.0".to_string(),
                port: 8080,
                health_check_interval: Duration::from_secs(30),
                metrics: MetricsConfig {
                    enabled: true,
                    endpoint: "/metrics".to_string(),
                    push_gateway: None,
                },
                logging: LoggingConfig {
                    level: "info".to_string(),
                    format: "json".to_string(),
                    colors: false,
                    file: None,
                },
            },
            domains: DomainConfigs {
                dialog: DialogConfig {
                    max_history: 100,
                    context_window: 10,
                    session_timeout: Duration::from_secs(3600),
                },
                graph: GraphConfig {
                    max_nodes: 1000,
                    auto_layout: true,
                    layout_algorithm: "force-directed".to_string(),
                },
                workflow: WorkflowConfig {
                    max_concurrent: 10,
                    timeout: Duration::from_secs(300),
                    persist: true,
                },
            },
        }
    }
}

// Add humantime_serde to Cargo.toml dependencies
use serde::{Deserialize as DeserializeHumantime, Serialize as SerializeHumantime};

mod humantime_serde {
    use serde::{self, Deserialize, Deserializer, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = format!("{}s", duration.as_secs());
        serializer.serialize_str(&s)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        // Simple parsing for now - just handle seconds
        if let Some(secs_str) = s.strip_suffix('s') {
            let secs: u64 = secs_str.parse().map_err(serde::de::Error::custom)?;
            Ok(Duration::from_secs(secs))
        } else {
            Err(serde::de::Error::custom("Invalid duration format"))
        }
    }
} 