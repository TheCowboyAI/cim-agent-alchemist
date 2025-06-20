//! NATS integration for agent communication
//!
//! This module handles all NATS-based messaging for the Alchemist agent,
//! including command processing, event publishing, and query handling.

use crate::error::{AgentError, Result};
use async_nats::{Client, Subscriber};
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

/// NATS subject patterns for the Alchemist agent
pub mod subjects {
    /// Command subjects
    pub const COMMANDS: &str = "cim.agent.alchemist.commands.>";
    
    /// Query subjects
    pub const QUERIES: &str = "cim.agent.alchemist.queries.>";
    
    /// Event subjects
    pub const EVENTS: &str = "cim.agent.alchemist.events.>";
    
    /// Dialog subjects
    pub const DIALOG: &str = "cim.dialog.alchemist.>";
    
    /// Health check subject
    pub const HEALTH: &str = "cim.agent.alchemist.health";
    
    /// Metrics subject
    pub const METRICS: &str = "cim.agent.alchemist.metrics";
}

/// NATS client wrapper for the agent
pub struct NatsClient {
    /// NATS connection
    connection: Client,
    
    /// JetStream context (if enabled)
    jetstream: Option<async_nats::jetstream::Context>,
    
    /// Subject prefix for this agent
    subject_prefix: String,
    
    /// Active subscriptions
    subscriptions: Arc<RwLock<Vec<Subscriber>>>,
}

impl NatsClient {
    /// Create a new NATS client
    pub async fn new(config: &crate::config::NatsConfig) -> Result<Self> {
        // Connect to NATS
        let mut options = async_nats::ConnectOptions::new();
        
        // Configure authentication if provided
        if let Some(auth) = &config.auth {
            options = match auth {
                crate::config::NatsAuth::Token { token } => options.token(token.clone()),
                crate::config::NatsAuth::UserPassword { username, password } => {
                    options.user_and_password(username.clone(), password.clone())
                }
                crate::config::NatsAuth::Jwt { jwt, seed } => {
                    options.jwt(jwt.clone(), seed.clone())
                }
                crate::config::NatsAuth::Tls { cert_path, key_path } => {
                    // TLS configuration would go here
                    options
                }
            };
        }
        
        // Set retry configuration
        options = options
            .max_reconnects(config.retry.max_attempts as usize)
            .retry_on_initial_connect();
        
        // Connect to NATS servers
        let client = async_nats::connect_with_options(
            config.servers.join(","),
            options,
        )
        .await?;
        
        // Create JetStream context if configured
        let jetstream = if let Some(js_config) = &config.jetstream {
            let js = async_nats::jetstream::new(client.clone());
            
            // Create or update stream
            let stream_config = async_nats::jetstream::stream::Config {
                name: js_config.stream_name.clone(),
                subjects: vec![
                    format!("{}.>", config.subject_prefix),
                ],
                retention: async_nats::jetstream::stream::RetentionPolicy::Limits,
                ..Default::default()
            };
            
            js.create_stream(stream_config).await.ok();
            
            Some(js)
        } else {
            None
        };
        
        Ok(Self {
            connection: client,
            jetstream,
            subject_prefix: config.subject_prefix.clone(),
            subscriptions: Arc::new(RwLock::new(Vec::new())),
        })
    }
    
    /// Subscribe to a subject pattern
    pub async fn subscribe(&self, subject: &str) -> Result<Subscriber> {
        let sub = self.connection.subscribe(subject).await?;
        
        // Track subscription
        let mut subs = self.subscriptions.write().await;
        subs.push(sub.clone());
        
        Ok(sub)
    }
    
    /// Publish a message
    pub async fn publish<T: Serialize>(&self, subject: &str, message: &T) -> Result<()> {
        let payload = serde_json::to_vec(message)?;
        self.connection.publish(subject, payload.into()).await?;
        Ok(())
    }
    
    /// Request-reply pattern
    pub async fn request<T: Serialize, R: for<'de> Deserialize<'de>>(
        &self,
        subject: &str,
        message: &T,
        timeout: std::time::Duration,
    ) -> Result<R> {
        let payload = serde_json::to_vec(message)?;
        
        let response = tokio::time::timeout(
            timeout,
            self.connection.request(subject, payload.into()),
        )
        .await
        .map_err(|_| AgentError::Timeout(format!("Request to {} timed out", subject)))?
        .map_err(|e| AgentError::Nats(e))?;
        
        let result: R = serde_json::from_slice(&response.payload)?;
        Ok(result)
    }
    
    /// Get JetStream context
    pub fn jetstream(&self) -> Option<&async_nats::jetstream::Context> {
        self.jetstream.as_ref()
    }
    
    /// Close all subscriptions
    pub async fn close(&self) -> Result<()> {
        let mut subs = self.subscriptions.write().await;
        for sub in subs.drain(..) {
            drop(sub);
        }
        Ok(())
    }
}

/// Message handler for incoming NATS messages
pub struct MessageHandler<H> {
    handler: H,
}

impl<H> MessageHandler<H> {
    pub fn new(handler: H) -> Self {
        Self { handler }
    }
}

/// Base message types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCommand {
    /// Command ID for tracking
    pub id: String,
    
    /// Command type
    pub command_type: String,
    
    /// Command payload
    pub payload: serde_json::Value,
    
    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    
    /// Originating user/system
    pub origin: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentQuery {
    /// Query ID for tracking
    pub id: String,
    
    /// Query type
    pub query_type: String,
    
    /// Query parameters
    pub parameters: serde_json::Value,
    
    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    
    /// Originating user/system
    pub origin: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentEvent {
    /// Event ID
    pub id: String,
    
    /// Event type
    pub event_type: String,
    
    /// Event payload
    pub payload: serde_json::Value,
    
    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    
    /// Agent ID that generated the event
    pub agent_id: String,
}

/// Dialog-specific messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialogMessage {
    /// Dialog ID
    pub dialog_id: String,
    
    /// Message content
    pub content: String,
    
    /// Sender (user or agent)
    pub sender: String,
    
    /// Message metadata
    pub metadata: serde_json::Value,
    
    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Health check response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    /// Agent status
    pub status: String,
    
    /// Agent version
    pub version: String,
    
    /// Uptime in seconds
    pub uptime_seconds: u64,
    
    /// Model provider status
    pub model_status: String,
    
    /// Active dialogs count
    pub active_dialogs: usize,
    
    /// Additional health metadata
    pub metadata: serde_json::Value,
}

/// Process incoming commands
pub async fn process_command_stream<F, Fut>(
    client: &NatsClient,
    mut handler: F,
) -> Result<()>
where
    F: FnMut(AgentCommand) -> Fut + Send,
    Fut: std::future::Future<Output = Result<serde_json::Value>> + Send,
{
    let mut sub = client.subscribe(subjects::COMMANDS).await?;
    
    info!("Listening for commands on {}", subjects::COMMANDS);
    
    while let Some(msg) = sub.next().await {
        match serde_json::from_slice::<AgentCommand>(&msg.payload) {
            Ok(command) => {
                debug!("Received command: {} ({})", command.command_type, command.id);
                
                match handler(command.clone()).await {
                    Ok(response) => {
                        // Publish response event
                        let event = AgentEvent {
                            id: uuid::Uuid::new_v4().to_string(),
                            event_type: format!("{}_completed", command.command_type),
                            payload: response,
                            timestamp: chrono::Utc::now(),
                            agent_id: crate::NAME.to_string(),
                        };
                        
                        if let Err(e) = client.publish(
                            &format!("{}.{}", subjects::EVENTS.trim_end_matches('>'), command.command_type),
                            &event,
                        ).await {
                            error!("Failed to publish command response: {}", e);
                        }
                    }
                    Err(e) => {
                        error!("Command handler error: {}", e);
                        
                        // Publish error event
                        let event = AgentEvent {
                            id: uuid::Uuid::new_v4().to_string(),
                            event_type: format!("{}_failed", command.command_type),
                            payload: serde_json::json!({
                                "error": e.to_string(),
                                "command_id": command.id,
                            }),
                            timestamp: chrono::Utc::now(),
                            agent_id: crate::NAME.to_string(),
                        };
                        
                        let _ = client.publish(
                            &format!("{}.error", subjects::EVENTS.trim_end_matches('>')),
                            &event,
                        ).await;
                    }
                }
            }
            Err(e) => {
                error!("Failed to parse command: {}", e);
            }
        }
    }
    
    Ok(())
}

/// Process incoming queries with request-reply
pub async fn process_query_stream<F, Fut>(
    client: &NatsClient,
    mut handler: F,
) -> Result<()>
where
    F: FnMut(AgentQuery) -> Fut + Send,
    Fut: std::future::Future<Output = Result<serde_json::Value>> + Send,
{
    let mut sub = client.subscribe(subjects::QUERIES).await?;
    
    info!("Listening for queries on {}", subjects::QUERIES);
    
    while let Some(msg) = sub.next().await {
        if let Some(reply) = msg.reply {
            match serde_json::from_slice::<AgentQuery>(&msg.payload) {
                Ok(query) => {
                    debug!("Received query: {} ({})", query.query_type, query.id);
                    
                    let response = match handler(query).await {
                        Ok(result) => serde_json::json!({
                            "success": true,
                            "result": result,
                        }),
                        Err(e) => serde_json::json!({
                            "success": false,
                            "error": e.to_string(),
                        }),
                    };
                    
                    let payload = serde_json::to_vec(&response)?;
                    if let Err(e) = msg.respond(payload.into()).await {
                        error!("Failed to send query response: {}", e);
                    }
                }
                Err(e) => {
                    error!("Failed to parse query: {}", e);
                    
                    let error_response = serde_json::json!({
                        "success": false,
                        "error": format!("Invalid query format: {}", e),
                    });
                    
                    let payload = serde_json::to_vec(&error_response)?;
                    let _ = msg.respond(payload.into()).await;
                }
            }
        }
    }
    
    Ok(())
}

/// Handle health check requests
pub async fn handle_health_checks<F>(
    client: &NatsClient,
    start_time: std::time::Instant,
    status_fn: F,
) -> Result<()>
where
    F: Fn() -> HealthResponse + Send + Sync,
{
    let mut sub = client.subscribe(subjects::HEALTH).await?;
    
    info!("Health check endpoint active on {}", subjects::HEALTH);
    
    while let Some(msg) = sub.next().await {
        if let Some(reply) = msg.reply {
            let mut health = status_fn();
            health.uptime_seconds = start_time.elapsed().as_secs();
            
            let payload = serde_json::to_vec(&health)?;
            let _ = msg.respond(payload.into()).await;
        }
    }
    
    Ok(())
} 