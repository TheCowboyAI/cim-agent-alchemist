//! # CIM Alchemist Agent
//!
//! The Alchemist agent is a specialized CIM agent designed to help users understand
//! and navigate the Composable Information Machine architecture. It combines multiple
//! domains to provide intelligent assistance:
//!
//! - **Agent Domain**: Core agent capabilities and lifecycle management
//! - **Dialog Domain**: Conversation management and context tracking
//! - **Identity Domain**: User and agent identity management
//! - **Graph Domain**: Visual representation of CIM concepts
//! - **Conceptual Spaces**: Semantic understanding of CIM architecture
//! - **Workflow Domain**: Guided workflows for CIM operations
//!
//! ## Architecture
//!
//! The Alchemist agent follows the CIM composition pattern:
//! 
//! ```mermaid
//! graph TD
//!     A[Alchemist Agent] --> B[Agent Domain]
//!     A --> C[Dialog Domain]
//!     A --> D[Identity Domain]
//!     A --> E[Graph Domain]
//!     A --> F[Conceptual Spaces]
//!     A --> G[Workflow Domain]
//!     A --> H[NATS Integration]
//!     A --> I[AI Model Service]
//!     
//!     H --> J[Event Streams]
//!     H --> K[Command Processing]
//!     H --> L[Query Handling]
//!     
//!     I --> M[Ollama]
//!     I --> N[OpenAI]
//!     I --> O[Anthropic]
//! ```

pub mod agent;
pub mod config;
pub mod error;
pub mod model;
pub mod nats_integration;
pub mod service;

// Re-export main types
pub use agent::{AlchemistAgent, AlchemistCapabilities};
pub use config::{AgentConfig, ModelConfig, NatsConfig};
pub use error::{AgentError, Result};
pub use model::{ModelProvider, ModelRequest, ModelResponse};
pub use service::{AgentService, ServiceStatus};

/// Version information for the Alchemist agent
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const NAME: &str = "alchemist";
pub const DESCRIPTION: &str = "CIM Architecture Assistant"; 