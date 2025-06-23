//! Agent service implementation
//!
//! This module provides the main service that runs the Alchemist agent,
//! handling NATS connections, message processing, and lifecycle management.

use crate::agent::AlchemistAgent;
use crate::config::AgentConfig;
use crate::error::{AgentError, Result};
use crate::model::{ModelProvider, OllamaProvider};
use crate::nats_integration::NatsClient;
use std::sync::Arc;
use tokio::task::JoinHandle;
use tracing::{error, info};

/// Status of the agent service
#[derive(Debug, Clone, PartialEq)]
pub enum ServiceStatus {
    /// Service is starting up
    Starting,
    
    /// Service is running and healthy
    Running,
    
    /// Service is shutting down
    Stopping,
    
    /// Service has stopped
    Stopped,
    
    /// Service encountered an error
    Error(String),
}

/// The main agent service that orchestrates all components
#[derive(Clone)]
pub struct AgentService {
    config: AgentConfig,
    agent: Arc<AlchemistAgent>,
    nats_client: Arc<NatsClient>,
    tasks: Arc<tokio::sync::Mutex<Vec<JoinHandle<()>>>>,
}

impl AgentService {
    /// Create a new agent service
    pub async fn new(config: AgentConfig) -> Result<Self> {
        // Create model provider based on configuration
        let model_provider = Self::create_model_provider(&config)?;
        
        // Create the Alchemist agent
        let agent = Arc::new(
            AlchemistAgent::new(config.identity.clone(), model_provider).await?
        );
        
        // Create NATS client
        let nats_client = Arc::new(NatsClient::new(config.nats.clone()).await?);
        
        Ok(Self {
            config,
            agent,
            nats_client,
            tasks: Arc::new(tokio::sync::Mutex::new(Vec::new())),
        })
    }
    
    /// Start the agent service
    pub async fn start(&self) -> Result<()> {
        info!("Starting Alchemist agent service");
        
        // Start NATS subscriptions
        self.start_nats_subscriptions().await?;
        
        // Start health check task
        self.start_health_check().await?;
        
        info!("Alchemist agent service started successfully");
        Ok(())
    }
    
    /// Stop the agent service
    pub async fn stop(&self) -> Result<()> {
        info!("Stopping Alchemist agent service");
        
        // Cancel all tasks
        let mut tasks = self.tasks.lock().await;
        for task in tasks.drain(..) {
            task.abort();
        }
        
        info!("Alchemist agent service stopped");
        Ok(())
    }
    
    /// Create model provider based on configuration
    fn create_model_provider(config: &AgentConfig) -> Result<Box<dyn ModelProvider>> {
        match &config.model {
            crate::config::ModelConfig::Ollama { base_url, model, .. } => {
                Ok(Box::new(OllamaProvider::new(
                    base_url.clone(),
                    model.clone(),
                    std::collections::HashMap::new(),
                )))
            }
            crate::config::ModelConfig::OpenAI { .. } => {
                Err(AgentError::Configuration(
                    "OpenAI provider not yet implemented".to_string()
                ))
            }
            crate::config::ModelConfig::Anthropic { .. } => {
                Err(AgentError::Configuration(
                    "Anthropic provider not yet implemented".to_string()
                ))
            }
        }
    }
    
    /// Start NATS subscriptions
    async fn start_nats_subscriptions(&self) -> Result<()> {
        let nats_client = self.nats_client.clone();
        let agent = self.agent.clone();
        
        // Start command subscription
        let cmd_task = tokio::spawn(async move {
            if let Err(e) = nats_client.subscribe_commands(agent.clone()).await {
                error!("Command subscription error: {}", e);
            }
        });
        
        let nats_client = self.nats_client.clone();
        let agent = self.agent.clone();
        
        // Start query subscription
        let query_task = tokio::spawn(async move {
            if let Err(e) = nats_client.subscribe_queries(agent.clone()).await {
                error!("Query subscription error: {}", e);
            }
        });
        
        let nats_client = self.nats_client.clone();
        let agent = self.agent.clone();
        
        // Start dialog subscription
        let dialog_task = tokio::spawn(async move {
            if let Err(e) = nats_client.subscribe_dialogs(agent.clone()).await {
                error!("Dialog subscription error: {}", e);
            }
        });
        
        // Store tasks
        let mut tasks = self.tasks.lock().await;
        tasks.push(cmd_task);
        tasks.push(query_task);
        tasks.push(dialog_task);
        
        Ok(())
    }
    
    /// Start health check task
    async fn start_health_check(&self) -> Result<()> {
        let nats_client = self.nats_client.clone();
        let interval = self.config.service.health_check_interval.as_secs();
        
        let health_task = tokio::spawn(async move {
            let mut interval = tokio::time::interval(
                std::time::Duration::from_secs(interval)
            );
            
            loop {
                interval.tick().await;
                if let Err(e) = nats_client.publish_health_check().await {
                    error!("Health check error: {}", e);
                }
            }
        });
        
        let mut tasks = self.tasks.lock().await;
        tasks.push(health_task);
        
        Ok(())
    }
    
    /// Wait for service to complete (blocks until stopped)
    pub async fn wait(&self) -> Result<()> {
        // Wait for all tasks to complete
        let tasks = self.tasks.lock().await;
        if let Some(task) = tasks.first() {
            // Wait for the first task (they should all run indefinitely)
            let _ = task.await;
        }
        Ok(())
    }
}

/// Run the agent service with the given configuration
pub async fn run(config: crate::config::AgentConfig) -> Result<()> {
    // Initialize tracing
    init_tracing(&config.service.logging);
    
    // Create and start service
    let service = AgentService::new(config).await?;
    service.start().await?;
    
    // Set up shutdown handler
    let shutdown_service = service.clone();
    tokio::spawn(async move {
        match tokio::signal::ctrl_c().await {
            Ok(()) => {
                info!("Received shutdown signal");
                if let Err(e) = shutdown_service.stop().await {
                    error!("Error during shutdown: {}", e);
                }
            }
            Err(e) => {
                error!("Failed to listen for shutdown signal: {}", e);
            }
        }
    });
    
    // Wait for service to complete
    service.wait().await?;
    
    Ok(())
}

/// Initialize tracing/logging
fn init_tracing(config: &crate::config::LoggingConfig) {
    use tracing_subscriber::{fmt, EnvFilter};
    
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(&config.level));
    
    let fmt_layer = fmt::layer()
        .with_ansi(config.colors);
    
    let subscriber = fmt_layer
        .with_env_filter(env_filter);
    
    match config.format.as_str() {
        "json" => {
            tracing_subscriber::fmt()
                .json()
                .with_env_filter(env_filter)
                .init();
        }
        "pretty" => {
            tracing_subscriber::fmt()
                .pretty()
                .with_env_filter(env_filter)
                .init();
        }
        _ => {
            tracing_subscriber::fmt()
                .compact()
                .with_env_filter(env_filter)
                .init();
        }
    }
    
    info!("Logging initialized with level: {}", config.level);
} 