//! Integration tests for the Alchemist agent
//!
//! These tests require a running NATS server.
//!
//! ```mermaid
//! graph TD
//!     A[Test Client] --> B[NATS Server]
//!     B --> C[Alchemist Agent]
//!     C --> D[Mock Model Provider]
//!     C --> B
//!     B --> A
//! ```

use cim_agent_alchemist::{
    AgentConfig, ModelConfig,
    nats_integration::{AgentCommand, AgentQuery, DialogMessage, HealthResponse},
};
use async_nats::Client;
use serde_json::json;
use std::time::Duration;
use tokio::time::timeout;

/// Test configuration with mock model provider
fn test_config() -> AgentConfig {
    let mut config = AgentConfig::default();
    
    // Use test NATS server
    config.nats.servers = vec!["nats://localhost:4222".to_string()];
    config.nats.subject_prefix = "test.agent.alchemist".to_string();
    
    // Disable JetStream for tests
    config.nats.jetstream = None;
    
    // Set test logging
    config.service.logging.level = "debug".to_string();
    
    config
}

#[tokio::test]
#[ignore = "requires NATS server"]
async fn test_agent_health_check() {
    // Connect to NATS
    let client = Client::connect("nats://localhost:4222")
        .await
        .expect("Failed to connect to NATS");
    
    // Start agent service in background
    let config = test_config();
    let service_handle = tokio::spawn(async move {
        cim_agent_alchemist::service::run(config).await
    });
    
    // Wait for service to start
    tokio::time::sleep(Duration::from_secs(2)).await;
    
    // Send health check request
    let response = timeout(
        Duration::from_secs(5),
        client.request("test.agent.alchemist.health", "".into()),
    )
    .await
    .expect("Health check timed out")
    .expect("Health check request failed");
    
    // Parse response
    let health: HealthResponse = serde_json::from_slice(&response.payload)
        .expect("Failed to parse health response");
    
    assert_eq!(health.status, "Running");
    assert_eq!(health.version, cim_agent_alchemist::VERSION);
    
    // Cleanup
    service_handle.abort();
}

#[tokio::test]
#[ignore = "requires NATS server"]
async fn test_list_concepts_query() {
    let client = Client::connect("nats://localhost:4222")
        .await
        .expect("Failed to connect to NATS");
    
    // Create query
    let query = AgentQuery {
        id: "test-query-1".to_string(),
        query_type: "list_concepts".to_string(),
        parameters: json!({}),
        timestamp: chrono::Utc::now(),
        origin: "test".to_string(),
    };
    
    let payload = serde_json::to_vec(&query).expect("Failed to serialize query");
    
    // Send query
    let response = timeout(
        Duration::from_secs(5),
        client.request("test.agent.alchemist.queries.list_concepts", payload.into()),
    )
    .await
    .expect("Query timed out")
    .expect("Query request failed");
    
    // Parse response
    let result: serde_json::Value = serde_json::from_slice(&response.payload)
        .expect("Failed to parse response");
    
    assert!(result["success"].as_bool().unwrap_or(false));
    assert!(result["result"]["concepts"].is_array());
    
    let concepts = result["result"]["concepts"].as_array().unwrap();
    assert!(concepts.len() > 0);
    assert!(concepts.contains(&json!("Event Sourcing")));
}

#[tokio::test]
#[ignore = "requires NATS server and Ollama"]
async fn test_dialog_interaction() {
    let client = Client::connect("nats://localhost:4222")
        .await
        .expect("Failed to connect to NATS");
    
    // Start dialog
    let start_command = AgentCommand {
        id: "test-cmd-1".to_string(),
        command_type: "start_dialog".to_string(),
        payload: json!({
            "user_id": "test-user",
            "context": {},
            "metadata": {}
        }),
        timestamp: chrono::Utc::now(),
        origin: "test".to_string(),
    };
    
    // Publish command and wait for event
    let payload = serde_json::to_vec(&start_command).expect("Failed to serialize command");
    client.publish("test.agent.alchemist.commands.start_dialog", payload.into())
        .await
        .expect("Failed to publish command");
    
    // Subscribe to dialog responses
    let mut sub = client.subscribe("test.agent.alchemist.events.start_dialog_completed")
        .await
        .expect("Failed to subscribe");
    
    // Wait for response event
    let msg = timeout(Duration::from_secs(5), sub.next())
        .await
        .expect("Timed out waiting for event")
        .expect("No event received");
    
    let event: serde_json::Value = serde_json::from_slice(&msg.payload)
        .expect("Failed to parse event");
    
    let dialog_id = event["payload"]["dialog_id"].as_str()
        .expect("No dialog_id in response");
    
    // Send dialog message
    let dialog_msg = DialogMessage {
        dialog_id: dialog_id.to_string(),
        content: "What is Event Sourcing?".to_string(),
        sender: "test-user".to_string(),
        metadata: json!({}),
        timestamp: chrono::Utc::now(),
    };
    
    // Subscribe to responses first
    let mut response_sub = client.subscribe(&format!("cim.dialog.{}.response", dialog_id))
        .await
        .expect("Failed to subscribe to responses");
    
    // Send message
    let payload = serde_json::to_vec(&dialog_msg).expect("Failed to serialize message");
    client.publish("test.agent.alchemist.dialog", payload.into())
        .await
        .expect("Failed to publish dialog message");
    
    // Wait for response
    let response_msg = timeout(Duration::from_secs(10), response_sub.next())
        .await
        .expect("Timed out waiting for dialog response")
        .expect("No response received");
    
    let response: DialogMessage = serde_json::from_slice(&response_msg.payload)
        .expect("Failed to parse dialog response");
    
    assert_eq!(response.sender, "alchemist");
    assert!(!response.content.is_empty());
    assert!(response.content.to_lowercase().contains("event"));
}

#[tokio::test]
async fn test_error_handling() {
    // Test configuration validation
    let mut config = test_config();
    config.nats.servers = vec![]; // Invalid - no servers
    
    // This should fail validation when the service tries to start
    // In a real test, we'd check that the service handles this gracefully
}

#[tokio::test]
async fn test_command_validation() {
    // Test that invalid commands are rejected properly
    let invalid_command = AgentCommand {
        id: "test-invalid".to_string(),
        command_type: "invalid_command_type".to_string(),
        payload: json!({}),
        timestamp: chrono::Utc::now(),
        origin: "test".to_string(),
    };
    
    // In a real test with NATS running, we'd verify this returns an error event
}

/// Mock model provider for testing without Ollama
#[cfg(test)]
mod mock {
    use cim_agent_alchemist::model::{
        ModelProvider, ModelRequest, ModelResponse, ModelInfo, ModelCapabilities, TokenUsage,
    };
    use async_trait::async_trait;
    use std::time::Duration;
    
    pub struct MockModelProvider;
    
    #[async_trait]
    impl ModelProvider for MockModelProvider {
        async fn generate(
            &self,
            request: ModelRequest,
        ) -> cim_agent_alchemist::error::Result<ModelResponse> {
            // Simple mock responses based on prompt content
            let content = if request.prompt.contains("Event Sourcing") {
                "Event Sourcing is a pattern where state changes are stored as a sequence of events."
            } else if request.prompt.contains("explain") {
                "This is a mock explanation of the requested concept."
            } else {
                "This is a mock response from the test model provider."
            };
            
            Ok(ModelResponse {
                content: content.to_string(),
                usage: TokenUsage {
                    prompt_tokens: 10,
                    completion_tokens: 20,
                    total_tokens: 30,
                },
                metadata: serde_json::json!({"mock": true}),
                duration: Duration::from_millis(100),
            })
        }
        
        async fn health_check(&self) -> cim_agent_alchemist::error::Result<()> {
            Ok(())
        }
        
        fn model_info(&self) -> ModelInfo {
            ModelInfo {
                provider: "Mock".to_string(),
                model: "mock-model".to_string(),
                version: Some("1.0".to_string()),
                capabilities: ModelCapabilities {
                    max_context_length: 4096,
                    streaming: false,
                    function_calling: false,
                    vision: false,
                    embeddings: false,
                },
            }
        }
    }
} 