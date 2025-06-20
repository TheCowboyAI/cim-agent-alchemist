//! Agent service implementation
//!
//! This module provides the main service that runs the Alchemist agent,
//! handling NATS connections, message processing, and lifecycle management.

use crate::agent::AlchemistAgent;
use crate::error::{AgentError, Result};
use crate::nats_integration::{
    NatsClient, process_command_stream, process_query_stream, handle_health_checks,
    HealthResponse, DialogMessage,
};
use futures::StreamExt;
use std::sync::Arc;
use std::pin::Pin;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use tracing::{error, info, warn};

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

/// The main agent service
#[derive(Clone)]
pub struct AgentService {
    /// The Alchemist agent
    agent: Arc<AlchemistAgent>,
    
    /// NATS client
    nats_client: Arc<NatsClient>,
    
    /// Service status
    status: Arc<RwLock<ServiceStatus>>,
    
    /// Active task handles
    tasks: Arc<RwLock<Vec<JoinHandle<()>>>>,
    
    /// Service start time
    start_time: std::time::Instant,
}

impl AgentService {
    /// Create a new agent service
    pub async fn new(
        config: crate::config::AgentConfig,
    ) -> Result<Self> {
        info!("Starting Alchemist agent service v{}", crate::VERSION);
        
        // Create model provider
        let model_provider = crate::model::create_provider(&config.model)?;
        
        // Create agent
        let agent = AlchemistAgent::new(config.clone(), model_provider).await?;
        
        // Create NATS client
        let nats_client = NatsClient::new(&config.nats).await?;
        
        Ok(Self {
            agent: Arc::new(agent),
            nats_client: Arc::new(nats_client),
            status: Arc::new(RwLock::new(ServiceStatus::Starting)),
            tasks: Arc::new(RwLock::new(Vec::new())),
            start_time: std::time::Instant::now(),
        })
    }
    
    /// Start the agent service
    pub async fn start(&self) -> Result<()> {
        info!("Starting agent service tasks");
        
        // Update status
        *self.status.write().await = ServiceStatus::Running;
        
        // Start command processor
        let command_task = self.start_command_processor();
        
        // Start query processor
        let query_task = self.start_query_processor();
        
        // Start dialog processor
        let dialog_task = self.start_dialog_processor();
        
        // Start health check handler
        let health_task = self.start_health_handler();
        
        // Store task handles
        let mut tasks = self.tasks.write().await;
        tasks.push(command_task);
        tasks.push(query_task);
        tasks.push(dialog_task);
        tasks.push(health_task);
        
        info!("Agent service started successfully");
        
        Ok(())
    }
    
    /// Stop the agent service
    pub async fn stop(&self) -> Result<()> {
        info!("Stopping agent service");
        
        // Update status
        *self.status.write().await = ServiceStatus::Stopping;
        
        // Close NATS subscriptions
        self.nats_client.close().await?;
        
        // Cancel all tasks
        let mut tasks = self.tasks.write().await;
        for task in tasks.drain(..) {
            task.abort();
        }
        
        // Update status
        *self.status.write().await = ServiceStatus::Stopped;
        
        info!("Agent service stopped");
        
        Ok(())
    }
    
    /// Get current service status
    pub async fn status(&self) -> ServiceStatus {
        self.status.read().await.clone()
    }
    
    /// Wait for service to complete
    pub async fn wait(&self) -> Result<()> {
        let tasks = self.tasks.read().await.clone();
        
        for task in tasks {
            if let Err(e) = task.await {
                if !e.is_cancelled() {
                    error!("Task failed: {}", e);
                }
            }
        }
        
        Ok(())
    }
    
    /// Start command processor task
    fn start_command_processor(&self) -> JoinHandle<()> {
        let agent = self.agent.clone();
        let nats_client = self.nats_client.clone();
        let status = self.status.clone();
        
        tokio::spawn(async move {
            if let Err(e) = process_command_stream(&nats_client, move |command| {
                let agent = agent.clone();
                Box::pin(async move {
                    agent.process_command(command).await
                })
            }).await {
                error!("Command processor error: {}", e);
                *status.write().await = ServiceStatus::Error(e.to_string());
            }
        })
    }
    
    /// Start query processor task
    fn start_query_processor(&self) -> JoinHandle<()> {
        let agent = self.agent.clone();
        let nats_client = self.nats_client.clone();
        let status = self.status.clone();
        
        tokio::spawn(async move {
            if let Err(e) = process_query_stream(&nats_client, move |query| {
                let agent = agent.clone();
                Box::pin(async move {
                    agent.process_query(query).await
                })
            }).await {
                error!("Query processor error: {}", e);
                *status.write().await = ServiceStatus::Error(e.to_string());
            }
        })
    }
    
    /// Start dialog processor task
    fn start_dialog_processor(&self) -> JoinHandle<()> {
        let agent = self.agent.clone();
        let nats_client = self.nats_client.clone();
        let status = self.status.clone();
        
        tokio::spawn(async move {
            match nats_client.subscribe(crate::nats_integration::subjects::DIALOG).await {
                Ok(mut sub) => {
                    info!("Listening for dialog messages on {}", crate::nats_integration::subjects::DIALOG);
                    
                    while let Some(msg) = sub.next().await {
                        match serde_json::from_slice::<DialogMessage>(&msg.payload) {
                            Ok(dialog_msg) => {
                                match agent.process_dialog_message(dialog_msg.clone()).await {
                                    Ok(response) => {
                                        // Send response
                                        let response_msg = DialogMessage {
                                            dialog_id: dialog_msg.dialog_id,
                                            content: response,
                                            sender: crate::NAME.to_string(),
                                            metadata: serde_json::json!({
                                                "agent_id": crate::NAME,
                                                "timestamp": chrono::Utc::now(),
                                            }),
                                            timestamp: chrono::Utc::now(),
                                        };
                                        
                                        let subject = format!(
                                            "cim.dialog.{}.response",
                                            dialog_msg.dialog_id
                                        );
                                        
                                        if let Err(e) = nats_client.publish(&subject, &response_msg).await {
                                            error!("Failed to send dialog response: {}", e);
                                        }
                                    }
                                    Err(e) => {
                                        error!("Dialog processing error: {}", e);
                                    }
                                }
                            }
                            Err(e) => {
                                error!("Failed to parse dialog message: {}", e);
                            }
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to subscribe to dialog messages: {}", e);
                    *status.write().await = ServiceStatus::Error(e.to_string());
                }
            }
        })
    }
    
    /// Start health check handler
    fn start_health_handler(&self) -> JoinHandle<()> {
        let agent = self.agent.clone();
        let nats_client = self.nats_client.clone();
        let status = self.status.clone();
        let start_time = self.start_time;
        
        tokio::spawn(async move {
            let status_fn = move || {
                HealthResponse {
                    status: "Running".to_string(), // TODO: Get from status
                    version: crate::VERSION.to_string(),
                    uptime_seconds: 0, // Will be set by handler
                    model_status: "healthy".to_string(), // TODO: Check model health
                    active_dialogs: 0, // TODO: Get from agent
                    metadata: serde_json::json!({
                        "agent_name": crate::NAME,
                        "capabilities": {
                            "explain_concepts": true,
                            "visualize_architecture": true,
                            "guide_workflows": true,
                            "analyze_patterns": true,
                            "suggest_improvements": true,
                        },
                    }),
                }
            };
            
            if let Err(e) = handle_health_checks(&nats_client, start_time, status_fn).await {
                error!("Health check handler error: {}", e);
            }
        })
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