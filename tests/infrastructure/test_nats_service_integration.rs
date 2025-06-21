//! Infrastructure Layer 1.2: NATS Service Integration Tests for cim-agent-alchemist
//! 
//! User Story: As a system architect, I need the agent to provide services via NATS
//!
//! Test Requirements:
//! - Verify NATS service endpoint registration
//! - Verify request/response handling
//! - Verify service discovery
//! - Verify error handling
//!
//! Event Sequence:
//! 1. ServiceStarted { service_id, endpoints }
//! 2. RequestReceived { request_id, subject, payload }
//! 3. ResponseSent { request_id, response, latency }
//! 4. ServiceHealthChecked { service_id, status }
//!
//! ```mermaid
//! graph LR
//!     A[Test Start] --> B[Start Service]
//!     B --> C[ServiceStarted]
//!     C --> D[Receive Request]
//!     D --> E[RequestReceived]
//!     E --> F[Process Request]
//!     F --> G[Send Response]
//!     G --> H[ResponseSent]
//!     H --> I[Health Check]
//!     I --> J[ServiceHealthChecked]
//!     J --> K[Test Success]
//! ```

use std::collections::HashMap;
use std::time::{Duration, Instant};
use serde::{Serialize, Deserialize};
use serde_json::Value as JsonValue;

/// NATS service events
#[derive(Debug, Clone, PartialEq)]
pub enum NATSServiceEvent {
    ServiceStarted { service_id: String, endpoints: Vec<ServiceEndpoint> },
    RequestReceived { request_id: String, subject: String, payload: JsonValue },
    ResponseSent { request_id: String, response: ServiceResponse, latency_ms: u64 },
    ServiceHealthChecked { service_id: String, status: HealthStatus },
    ServiceError { service_id: String, error: String },
}

/// Service endpoint definition
#[derive(Debug, Clone, PartialEq)]
pub struct ServiceEndpoint {
    pub subject: String,
    pub handler: String,
    pub description: String,
    pub request_schema: Option<String>,
    pub response_schema: Option<String>,
}

/// Service response
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ServiceResponse {
    pub success: bool,
    pub data: Option<JsonValue>,
    pub error: Option<String>,
    pub metadata: HashMap<String, String>,
}

/// Health status
#[derive(Debug, Clone, PartialEq)]
pub enum HealthStatus {
    Healthy,
    Degraded { reason: String },
    Unhealthy { error: String },
}

/// Mock NATS service
pub struct MockNATSService {
    service_id: String,
    endpoints: Vec<ServiceEndpoint>,
    handlers: HashMap<String, Box<dyn Fn(&JsonValue) -> ServiceResponse + Send + Sync>>,
    request_count: u64,
    error_count: u64,
    started: bool,
}

impl MockNATSService {
    pub fn new(service_id: String) -> Self {
        Self {
            service_id,
            endpoints: Vec::new(),
            handlers: HashMap::new(),
            request_count: 0,
            error_count: 0,
            started: false,
        }
    }

    pub fn register_endpoint(&mut self, endpoint: ServiceEndpoint) -> Result<(), String> {
        if self.started {
            return Err("Cannot register endpoints after service started".to_string());
        }

        if self.endpoints.iter().any(|e| e.subject == endpoint.subject) {
            return Err(format!("Endpoint {} already registered", endpoint.subject));
        }

        self.endpoints.push(endpoint);
        Ok(())
    }

    pub fn register_handler<F>(&mut self, subject: String, handler: F) -> Result<(), String>
    where
        F: Fn(&JsonValue) -> ServiceResponse + Send + Sync + 'static,
    {
        if !self.endpoints.iter().any(|e| e.subject == subject) {
            return Err(format!("No endpoint registered for subject {}", subject));
        }

        self.handlers.insert(subject, Box::new(handler));
        Ok(())
    }

    pub async fn start(&mut self) -> Result<(), String> {
        // Simulate service startup delay
        tokio::time::sleep(Duration::from_millis(50)).await;

        if self.endpoints.is_empty() {
            return Err("No endpoints registered".to_string());
        }

        self.started = true;
        Ok(())
    }

    pub async fn handle_request(
        &mut self,
        subject: &str,
        payload: &JsonValue,
    ) -> Result<ServiceResponse, String> {
        let start = Instant::now();

        if !self.started {
            return Err("Service not started".to_string());
        }

        self.request_count += 1;

        // Find handler
        let handler = self.handlers.get(subject)
            .ok_or_else(|| format!("No handler for subject {}", subject))?;

        // Process request
        let response = handler(payload);

        // Track errors
        if !response.success {
            self.error_count += 1;
        }

        // Simulate processing delay
        tokio::time::sleep(Duration::from_millis(10)).await;

        Ok(response)
    }

    pub fn get_health_status(&self) -> HealthStatus {
        if !self.started {
            return HealthStatus::Unhealthy {
                error: "Service not started".to_string(),
            };
        }

        let error_rate = if self.request_count > 0 {
            (self.error_count as f64) / (self.request_count as f64)
        } else {
            0.0
        };

        if error_rate > 0.5 {
            HealthStatus::Unhealthy {
                error: format!("High error rate: {:.2}%", error_rate * 100.0),
            }
        } else if error_rate > 0.1 {
            HealthStatus::Degraded {
                reason: format!("Elevated error rate: {:.2}%", error_rate * 100.0),
            }
        } else {
            HealthStatus::Healthy
        }
    }

    pub fn get_metrics(&self) -> ServiceMetrics {
        ServiceMetrics {
            request_count: self.request_count,
            error_count: self.error_count,
            endpoint_count: self.endpoints.len(),
            uptime_seconds: 0, // Simplified for testing
        }
    }
}

#[derive(Debug, Clone)]
pub struct ServiceMetrics {
    pub request_count: u64,
    pub error_count: u64,
    pub endpoint_count: usize,
    pub uptime_seconds: u64,
}

/// Service discovery manager
pub struct ServiceDiscoveryManager {
    services: HashMap<String, ServiceInfo>,
}

#[derive(Debug, Clone)]
struct ServiceInfo {
    service_id: String,
    endpoints: Vec<ServiceEndpoint>,
    health_status: HealthStatus,
    last_seen: Instant,
}

impl ServiceDiscoveryManager {
    pub fn new() -> Self {
        Self {
            services: HashMap::new(),
        }
    }

    pub fn register_service(
        &mut self,
        service_id: String,
        endpoints: Vec<ServiceEndpoint>,
    ) -> Result<(), String> {
        if self.services.contains_key(&service_id) {
            return Err("Service already registered".to_string());
        }

        self.services.insert(service_id.clone(), ServiceInfo {
            service_id,
            endpoints,
            health_status: HealthStatus::Healthy,
            last_seen: Instant::now(),
        });

        Ok(())
    }

    pub fn update_health_status(
        &mut self,
        service_id: &str,
        status: HealthStatus,
    ) -> Result<(), String> {
        let service = self.services.get_mut(service_id)
            .ok_or_else(|| "Service not found".to_string())?;

        service.health_status = status;
        service.last_seen = Instant::now();

        Ok(())
    }

    pub fn find_service_by_subject(&self, subject: &str) -> Option<String> {
        for (service_id, info) in &self.services {
            if info.endpoints.iter().any(|e| e.subject == subject) {
                return Some(service_id.clone());
            }
        }
        None
    }

    pub fn get_healthy_services(&self) -> Vec<String> {
        self.services
            .iter()
            .filter(|(_, info)| matches!(info.health_status, HealthStatus::Healthy))
            .map(|(id, _)| id.clone())
            .collect()
    }
}

/// Request router
pub struct RequestRouter {
    routes: HashMap<String, String>, // subject -> service_id
    discovery: ServiceDiscoveryManager,
}

impl RequestRouter {
    pub fn new(discovery: ServiceDiscoveryManager) -> Self {
        Self {
            routes: HashMap::new(),
            discovery,
        }
    }

    pub fn update_routes(&mut self) {
        self.routes.clear();

        for service_id in self.discovery.get_healthy_services() {
            if let Some(info) = self.discovery.services.get(&service_id) {
                for endpoint in &info.endpoints {
                    self.routes.insert(endpoint.subject.clone(), service_id.clone());
                }
            }
        }
    }

    pub fn route_request(&self, subject: &str) -> Option<String> {
        self.routes.get(subject).cloned()
    }

    pub fn get_route_count(&self) -> usize {
        self.routes.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_service_registration() {
        // Arrange
        let mut service = MockNATSService::new("agent-service-001".to_string());
        
        let endpoint = ServiceEndpoint {
            subject: "agent.query".to_string(),
            handler: "handle_query".to_string(),
            description: "Handle agent queries".to_string(),
            request_schema: Some("QueryRequest".to_string()),
            response_schema: Some("QueryResponse".to_string()),
        };

        // Act
        let result = service.register_endpoint(endpoint);

        // Assert
        assert!(result.is_ok());
        assert_eq!(service.endpoints.len(), 1);
    }

    #[tokio::test]
    async fn test_service_startup() {
        // Arrange
        let mut service = MockNATSService::new("agent-service-002".to_string());
        
        service.register_endpoint(ServiceEndpoint {
            subject: "agent.command".to_string(),
            handler: "handle_command".to_string(),
            description: "Handle agent commands".to_string(),
            request_schema: None,
            response_schema: None,
        }).unwrap();

        // Act
        let result = service.start().await;

        // Assert
        assert!(result.is_ok());
        assert!(service.started);
    }

    #[tokio::test]
    async fn test_request_handling() {
        // Arrange
        let mut service = MockNATSService::new("agent-service-003".to_string());
        
        service.register_endpoint(ServiceEndpoint {
            subject: "agent.process".to_string(),
            handler: "handle_process".to_string(),
            description: "Process requests".to_string(),
            request_schema: None,
            response_schema: None,
        }).unwrap();

        service.register_handler("agent.process".to_string(), |payload| {
            ServiceResponse {
                success: true,
                data: Some(json!({
                    "processed": true,
                    "input": payload
                })),
                error: None,
                metadata: HashMap::new(),
            }
        }).unwrap();

        service.start().await.unwrap();

        let request = json!({
            "action": "analyze",
            "target": "code"
        });

        // Act
        let response = service.handle_request("agent.process", &request).await.unwrap();

        // Assert
        assert!(response.success);
        assert!(response.data.is_some());
        assert_eq!(service.get_metrics().request_count, 1);
    }

    #[tokio::test]
    async fn test_health_status() {
        // Arrange
        let mut service = MockNATSService::new("agent-service-004".to_string());
        
        // Before start
        assert!(matches!(service.get_health_status(), HealthStatus::Unhealthy { .. }));

        // Setup and start
        service.register_endpoint(ServiceEndpoint {
            subject: "agent.health".to_string(),
            handler: "handle_health".to_string(),
            description: "Health check".to_string(),
            request_schema: None,
            response_schema: None,
        }).unwrap();

        service.start().await.unwrap();

        // Act & Assert - Healthy state
        assert!(matches!(service.get_health_status(), HealthStatus::Healthy));
    }

    #[tokio::test]
    async fn test_service_discovery() {
        // Arrange
        let mut discovery = ServiceDiscoveryManager::new();
        
        let endpoints = vec![
            ServiceEndpoint {
                subject: "agent.query".to_string(),
                handler: "handle_query".to_string(),
                description: "Query handler".to_string(),
                request_schema: None,
                response_schema: None,
            },
            ServiceEndpoint {
                subject: "agent.command".to_string(),
                handler: "handle_command".to_string(),
                description: "Command handler".to_string(),
                request_schema: None,
                response_schema: None,
            },
        ];

        // Act
        discovery.register_service("service-001".to_string(), endpoints).unwrap();

        // Assert
        assert_eq!(discovery.find_service_by_subject("agent.query"), Some("service-001".to_string()));
        assert_eq!(discovery.get_healthy_services().len(), 1);
    }

    #[tokio::test]
    async fn test_request_routing() {
        // Arrange
        let mut discovery = ServiceDiscoveryManager::new();
        
        discovery.register_service("service-001".to_string(), vec![
            ServiceEndpoint {
                subject: "agent.v1.query".to_string(),
                handler: "handle_v1_query".to_string(),
                description: "V1 Query handler".to_string(),
                request_schema: None,
                response_schema: None,
            },
        ]).unwrap();

        discovery.register_service("service-002".to_string(), vec![
            ServiceEndpoint {
                subject: "agent.v2.query".to_string(),
                handler: "handle_v2_query".to_string(),
                description: "V2 Query handler".to_string(),
                request_schema: None,
                response_schema: None,
            },
        ]).unwrap();

        let mut router = RequestRouter::new(discovery);

        // Act
        router.update_routes();

        // Assert
        assert_eq!(router.get_route_count(), 2);
        assert_eq!(router.route_request("agent.v1.query"), Some("service-001".to_string()));
        assert_eq!(router.route_request("agent.v2.query"), Some("service-002".to_string()));
    }

    #[tokio::test]
    async fn test_error_handling() {
        // Arrange
        let mut service = MockNATSService::new("agent-service-005".to_string());
        
        service.register_endpoint(ServiceEndpoint {
            subject: "agent.error".to_string(),
            handler: "handle_error".to_string(),
            description: "Error test".to_string(),
            request_schema: None,
            response_schema: None,
        }).unwrap();

        service.register_handler("agent.error".to_string(), |_| {
            ServiceResponse {
                success: false,
                data: None,
                error: Some("Simulated error".to_string()),
                metadata: HashMap::new(),
            }
        }).unwrap();

        service.start().await.unwrap();

        // Act
        let response = service.handle_request("agent.error", &json!({})).await.unwrap();

        // Assert
        assert!(!response.success);
        assert!(response.error.is_some());
        assert_eq!(service.get_metrics().error_count, 1);
    }

    #[tokio::test]
    async fn test_full_service_flow() {
        // Arrange
        let mut service = MockNATSService::new("agent-service-006".to_string());
        let mut discovery = ServiceDiscoveryManager::new();

        // Register endpoints
        let endpoints = vec![
            ServiceEndpoint {
                subject: "agent.analyze".to_string(),
                handler: "handle_analyze".to_string(),
                description: "Code analysis".to_string(),
                request_schema: Some("AnalyzeRequest".to_string()),
                response_schema: Some("AnalyzeResponse".to_string()),
            },
        ];

        for endpoint in &endpoints {
            service.register_endpoint(endpoint.clone()).unwrap();
        }

        // Register handler
        service.register_handler("agent.analyze".to_string(), |payload| {
            ServiceResponse {
                success: true,
                data: Some(json!({
                    "analysis": "complete",
                    "issues": 0,
                    "suggestions": ["Use more descriptive names"]
                })),
                error: None,
                metadata: {
                    let mut meta = HashMap::new();
                    meta.insert("version".to_string(), "1.0".to_string());
                    meta
                },
            }
        }).unwrap();

        // Start service
        service.start().await.unwrap();

        // Register with discovery
        discovery.register_service(service.service_id.clone(), endpoints).unwrap();

        // Create router
        let mut router = RequestRouter::new(discovery);
        router.update_routes();

        // Act
        let request = json!({
            "code": "fn main() { println!(\"Hello\"); }",
            "language": "rust"
        });

        let service_id = router.route_request("agent.analyze").unwrap();
        let response = service.handle_request("agent.analyze", &request).await.unwrap();

        // Assert
        assert_eq!(service_id, service.service_id);
        assert!(response.success);
        assert!(response.data.is_some());
        assert_eq!(service.get_metrics().request_count, 1);
        assert!(matches!(service.get_health_status(), HealthStatus::Healthy));
    }
} 