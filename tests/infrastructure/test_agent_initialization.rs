//! Infrastructure Layer 1.1: Agent Initialization Tests for cim-agent-alchemist
//! 
//! User Story: As a developer, I need to initialize the Alchemist agent with proper configuration
//!
//! Test Requirements:
//! - Verify agent configuration loading
//! - Verify identity establishment
//! - Verify capability registration
//! - Verify NATS connection setup
//!
//! Event Sequence:
//! 1. AgentConfigLoaded { agent_id, config }
//! 2. IdentityEstablished { agent_id, identity }
//! 3. CapabilitiesRegistered { agent_id, capabilities }
//! 4. NATSServiceRegistered { agent_id, service_name }
//!
//! ```mermaid
//! graph LR
//!     A[Test Start] --> B[Load Config]
//!     B --> C[AgentConfigLoaded]
//!     C --> D[Establish Identity]
//!     D --> E[IdentityEstablished]
//!     E --> F[Register Capabilities]
//!     F --> G[CapabilitiesRegistered]
//!     G --> H[Register NATS Service]
//!     H --> I[NATSServiceRegistered]
//!     I --> J[Test Success]
//! ```

use std::collections::HashMap;
use std::time::Duration;
use serde::{Serialize, Deserialize};
use uuid::Uuid;

/// Agent infrastructure events
#[derive(Debug, Clone, PartialEq)]
pub enum AgentInfrastructureEvent {
    AgentConfigLoaded { agent_id: String, config: AgentConfig },
    IdentityEstablished { agent_id: String, identity: AgentIdentity },
    CapabilitiesRegistered { agent_id: String, capabilities: Vec<AgentCapability> },
    NATSServiceRegistered { agent_id: String, service_name: String },
    InitializationFailed { agent_id: String, error: String },
}

/// Agent configuration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentConfig {
    pub agent_id: String,
    pub name: String,
    pub description: String,
    pub nats_url: String,
    pub capabilities: Vec<String>,
    pub metadata: HashMap<String, String>,
}

/// Agent identity
#[derive(Debug, Clone, PartialEq)]
pub struct AgentIdentity {
    pub id: String,
    pub name: String,
    pub role: AgentRole,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Agent roles
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AgentRole {
    Assistant,
    Architect,
    Developer,
    Analyst,
    Custom(String),
}

/// Agent capabilities
#[derive(Debug, Clone, PartialEq)]
pub struct AgentCapability {
    pub name: String,
    pub description: String,
    pub enabled: bool,
    pub parameters: HashMap<String, String>,
}

/// Event stream validator for agent testing
pub struct AgentEventStreamValidator {
    expected_events: Vec<AgentInfrastructureEvent>,
    captured_events: Vec<AgentInfrastructureEvent>,
}

impl AgentEventStreamValidator {
    pub fn new() -> Self {
        Self {
            expected_events: Vec::new(),
            captured_events: Vec::new(),
        }
    }

    pub fn expect_sequence(mut self, events: Vec<AgentInfrastructureEvent>) -> Self {
        self.expected_events = events;
        self
    }

    pub fn capture_event(&mut self, event: AgentInfrastructureEvent) {
        self.captured_events.push(event);
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.captured_events.len() != self.expected_events.len() {
            return Err(format!(
                "Event count mismatch: expected {}, got {}",
                self.expected_events.len(),
                self.captured_events.len()
            ));
        }

        for (i, (expected, actual)) in self.expected_events.iter()
            .zip(self.captured_events.iter())
            .enumerate()
        {
            if expected != actual {
                return Err(format!(
                    "Event mismatch at position {}: expected {:?}, got {:?}",
                    i, expected, actual
                ));
            }
        }

        Ok(())
    }
}

/// Mock agent manager
pub struct MockAgentManager {
    config: Option<AgentConfig>,
    identity: Option<AgentIdentity>,
    capabilities: Vec<AgentCapability>,
    service_registered: bool,
}

impl MockAgentManager {
    pub fn new() -> Self {
        Self {
            config: None,
            identity: None,
            capabilities: Vec::new(),
            service_registered: false,
        }
    }

    pub async fn load_config(&mut self, config_path: &str) -> Result<AgentConfig, String> {
        // Simulate config loading delay
        tokio::time::sleep(Duration::from_millis(10)).await;

        if config_path.is_empty() {
            return Err("Config path not provided".to_string());
        }

        // Create mock config
        let config = AgentConfig {
            agent_id: Uuid::new_v4().to_string(),
            name: "Alchemist Agent".to_string(),
            description: "AI assistant for CIM architecture".to_string(),
            nats_url: "nats://localhost:4222".to_string(),
            capabilities: vec![
                "code_analysis".to_string(),
                "architecture_guidance".to_string(),
                "documentation".to_string(),
            ],
            metadata: {
                let mut meta = HashMap::new();
                meta.insert("version".to_string(), "0.3.0".to_string());
                meta.insert("environment".to_string(), "test".to_string());
                meta
            },
        };

        self.config = Some(config.clone());
        Ok(config)
    }

    pub fn establish_identity(&mut self, config: &AgentConfig) -> Result<AgentIdentity, String> {
        if self.config.is_none() {
            return Err("Config not loaded".to_string());
        }

        let identity = AgentIdentity {
            id: config.agent_id.clone(),
            name: config.name.clone(),
            role: AgentRole::Assistant,
            created_at: chrono::Utc::now(),
        };

        self.identity = Some(identity.clone());
        Ok(identity)
    }

    pub fn register_capabilities(&mut self, config: &AgentConfig) -> Result<Vec<AgentCapability>, String> {
        if self.identity.is_none() {
            return Err("Identity not established".to_string());
        }

        let capabilities: Vec<AgentCapability> = config.capabilities
            .iter()
            .map(|cap| AgentCapability {
                name: cap.clone(),
                description: format!("Capability for {}", cap),
                enabled: true,
                parameters: HashMap::new(),
            })
            .collect();

        self.capabilities = capabilities.clone();
        Ok(capabilities)
    }

    pub async fn register_nats_service(&mut self, service_name: &str) -> Result<(), String> {
        // Simulate service registration delay
        tokio::time::sleep(Duration::from_millis(20)).await;

        if self.capabilities.is_empty() {
            return Err("No capabilities registered".to_string());
        }

        if service_name.is_empty() {
            return Err("Service name not provided".to_string());
        }

        self.service_registered = true;
        Ok(())
    }

    pub fn is_initialized(&self) -> bool {
        self.config.is_some() 
            && self.identity.is_some() 
            && !self.capabilities.is_empty() 
            && self.service_registered
    }
}

/// Capability manager
pub struct CapabilityManager {
    capabilities: HashMap<String, CapabilityDefinition>,
}

#[derive(Debug, Clone)]
struct CapabilityDefinition {
    name: String,
    handler: String,
    required_permissions: Vec<String>,
    rate_limit: Option<u32>,
}

impl CapabilityManager {
    pub fn new() -> Self {
        Self {
            capabilities: HashMap::new(),
        }
    }

    pub fn register_capability(&mut self, capability: &AgentCapability) -> Result<(), String> {
        if capability.name.is_empty() {
            return Err("Capability name cannot be empty".to_string());
        }

        let definition = CapabilityDefinition {
            name: capability.name.clone(),
            handler: format!("handle_{}", capability.name),
            required_permissions: vec!["execute".to_string()],
            rate_limit: Some(100), // 100 requests per minute
        };

        self.capabilities.insert(capability.name.clone(), definition);
        Ok(())
    }

    pub fn get_capability(&self, name: &str) -> Option<&CapabilityDefinition> {
        self.capabilities.get(name)
    }

    pub fn list_capabilities(&self) -> Vec<String> {
        self.capabilities.keys().cloned().collect()
    }

    pub fn validate_capability(&self, name: &str) -> Result<(), String> {
        if !self.capabilities.contains_key(name) {
            return Err(format!("Capability '{}' not found", name));
        }
        Ok(())
    }
}

/// Identity manager
pub struct IdentityManager {
    identities: HashMap<String, AgentIdentity>,
    active_identity: Option<String>,
}

impl IdentityManager {
    pub fn new() -> Self {
        Self {
            identities: HashMap::new(),
            active_identity: None,
        }
    }

    pub fn create_identity(&mut self, identity: AgentIdentity) -> Result<(), String> {
        if self.identities.contains_key(&identity.id) {
            return Err("Identity already exists".to_string());
        }

        let id = identity.id.clone();
        self.identities.insert(id.clone(), identity);
        
        // Set as active if first identity
        if self.active_identity.is_none() {
            self.active_identity = Some(id);
        }

        Ok(())
    }

    pub fn get_active_identity(&self) -> Option<&AgentIdentity> {
        self.active_identity
            .as_ref()
            .and_then(|id| self.identities.get(id))
    }

    pub fn switch_identity(&mut self, identity_id: &str) -> Result<(), String> {
        if !self.identities.contains_key(identity_id) {
            return Err("Identity not found".to_string());
        }

        self.active_identity = Some(identity_id.to_string());
        Ok(())
    }

    pub fn list_identities(&self) -> Vec<&AgentIdentity> {
        self.identities.values().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_agent_config_loading() {
        // Arrange
        let mut manager = MockAgentManager::new();
        let config_path = "/etc/alchemist/config.toml";

        // Act
        let config = manager.load_config(config_path).await.unwrap();

        // Assert
        assert!(!config.agent_id.is_empty());
        assert_eq!(config.name, "Alchemist Agent");
        assert_eq!(config.capabilities.len(), 3);
        assert!(config.capabilities.contains(&"code_analysis".to_string()));
    }

    #[tokio::test]
    async fn test_identity_establishment() {
        // Arrange
        let mut manager = MockAgentManager::new();
        let config = manager.load_config("/etc/config.toml").await.unwrap();

        // Act
        let identity = manager.establish_identity(&config).unwrap();

        // Assert
        assert_eq!(identity.id, config.agent_id);
        assert_eq!(identity.name, config.name);
        assert_eq!(identity.role, AgentRole::Assistant);
    }

    #[tokio::test]
    async fn test_capability_registration() {
        // Arrange
        let mut manager = MockAgentManager::new();
        let config = manager.load_config("/etc/config.toml").await.unwrap();
        manager.establish_identity(&config).unwrap();

        // Act
        let capabilities = manager.register_capabilities(&config).unwrap();

        // Assert
        assert_eq!(capabilities.len(), 3);
        assert!(capabilities.iter().all(|c| c.enabled));
        assert!(capabilities.iter().any(|c| c.name == "code_analysis"));
    }

    #[tokio::test]
    async fn test_nats_service_registration() {
        // Arrange
        let mut manager = MockAgentManager::new();
        let config = manager.load_config("/etc/config.toml").await.unwrap();
        manager.establish_identity(&config).unwrap();
        manager.register_capabilities(&config).unwrap();

        // Act
        let result = manager.register_nats_service("alchemist.agent").await;

        // Assert
        assert!(result.is_ok());
        assert!(manager.is_initialized());
    }

    #[tokio::test]
    async fn test_empty_config_path_failure() {
        // Arrange
        let mut manager = MockAgentManager::new();

        // Act
        let result = manager.load_config("").await;

        // Assert
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Config path not provided"));
    }

    #[tokio::test]
    async fn test_capability_manager() {
        // Arrange
        let mut cap_manager = CapabilityManager::new();
        let capability = AgentCapability {
            name: "test_capability".to_string(),
            description: "Test capability".to_string(),
            enabled: true,
            parameters: HashMap::new(),
        };

        // Act
        cap_manager.register_capability(&capability).unwrap();

        // Assert
        assert!(cap_manager.get_capability("test_capability").is_some());
        assert_eq!(cap_manager.list_capabilities().len(), 1);
        assert!(cap_manager.validate_capability("test_capability").is_ok());
    }

    #[tokio::test]
    async fn test_identity_manager() {
        // Arrange
        let mut id_manager = IdentityManager::new();
        let identity = AgentIdentity {
            id: "agent-001".to_string(),
            name: "Test Agent".to_string(),
            role: AgentRole::Developer,
            created_at: chrono::Utc::now(),
        };

        // Act
        id_manager.create_identity(identity.clone()).unwrap();

        // Assert
        assert!(id_manager.get_active_identity().is_some());
        assert_eq!(id_manager.get_active_identity().unwrap().id, "agent-001");
        assert_eq!(id_manager.list_identities().len(), 1);
    }

    #[tokio::test]
    async fn test_full_initialization_flow() {
        // Arrange
        let mut validator = AgentEventStreamValidator::new();
        let mut manager = MockAgentManager::new();

        // Act
        // 1. Load config
        let config = manager.load_config("/etc/config.toml").await.unwrap();
        validator.capture_event(AgentInfrastructureEvent::AgentConfigLoaded {
            agent_id: config.agent_id.clone(),
            config: config.clone(),
        });

        // 2. Establish identity
        let identity = manager.establish_identity(&config).unwrap();
        validator.capture_event(AgentInfrastructureEvent::IdentityEstablished {
            agent_id: config.agent_id.clone(),
            identity,
        });

        // 3. Register capabilities
        let capabilities = manager.register_capabilities(&config).unwrap();
        validator.capture_event(AgentInfrastructureEvent::CapabilitiesRegistered {
            agent_id: config.agent_id.clone(),
            capabilities,
        });

        // 4. Register NATS service
        manager.register_nats_service("alchemist.agent").await.unwrap();
        validator.capture_event(AgentInfrastructureEvent::NATSServiceRegistered {
            agent_id: config.agent_id.clone(),
            service_name: "alchemist.agent".to_string(),
        });

        // Assert
        assert!(manager.is_initialized());
        assert_eq!(validator.captured_events.len(), 4);
    }
} 