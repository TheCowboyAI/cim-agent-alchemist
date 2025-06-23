//! CIM Alchemist Agent Library
//! 
//! This library provides the core functionality for the CIM Alchemist AI assistant.

pub mod agent;
pub mod config;
pub mod error;
pub mod model;
pub mod nats_integration;
pub mod service;

#[cfg(feature = "bevy")]
pub mod bevy_plugin;

// Re-export main types
pub use agent::AlchemistAgent;
pub use config::AgentConfig;
pub use error::{AgentError, Result};
pub use service::AgentService;
pub use nats_integration::NatsClient;
pub use model::ModelProvider;

#[cfg(feature = "bevy")]
pub use bevy_plugin::{
    AlchemistAgentPlugin,
    AgentQuestionEvent,
    AgentResponseEvent,
    AgentErrorEvent,
    AgentConfig,
    ask_agent,
    handle_agent_input,
};

/// Version information for the Alchemist agent
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const NAME: &str = env!("CARGO_PKG_NAME");
pub const DESCRIPTION: &str = env!("CARGO_PKG_DESCRIPTION"); 