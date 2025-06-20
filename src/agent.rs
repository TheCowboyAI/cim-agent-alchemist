//! Core Alchemist agent implementation
//!
//! This module implements the main agent logic that composes multiple CIM domains
//! to provide intelligent assistance for understanding CIM architecture.

use crate::error::{AgentError, Result};
use crate::model::{ModelProvider, ModelRequest, ModelResponse, Message as ModelMessage};
use crate::nats_integration::{AgentCommand, AgentQuery, DialogMessage};
use cim_domain_agent::{Agent, AgentStatus, AgentType};
use cim_domain_dialog::{Dialog, DialogStatus, Turn, TurnType, Message, MessageContent};
use cim_domain_graph::{GraphAggregate, NodeId, EdgeId};
use cim_domain_conceptualspaces::{ConceptualSpace, ConceptualPoint};
use cim_domain_workflow::{Workflow, WorkflowStatus};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

/// The Alchemist agent - helps users understand CIM architecture
pub struct AlchemistAgent {
    /// Agent identity from agent domain
    agent: Agent,
    
    /// Active dialogs
    dialogs: Arc<RwLock<HashMap<String, Dialog>>>,
    
    /// Knowledge graph of CIM concepts
    knowledge_graph: Arc<RwLock<GraphAggregate>>,
    
    /// Conceptual space for semantic understanding
    conceptual_space: Arc<RwLock<ConceptualSpace>>,
    
    /// Active workflows
    workflows: Arc<RwLock<HashMap<String, Workflow>>>,
    
    /// AI model provider
    model_provider: Box<dyn ModelProvider>,
    
    /// Agent configuration
    config: crate::config::AgentConfig,
}

/// Capabilities of the Alchemist agent
#[derive(Debug, Clone)]
pub struct AlchemistCapabilities {
    /// Can explain CIM concepts
    pub explain_concepts: bool,
    
    /// Can visualize architecture
    pub visualize_architecture: bool,
    
    /// Can guide through workflows
    pub guide_workflows: bool,
    
    /// Can analyze code patterns
    pub analyze_patterns: bool,
    
    /// Can suggest improvements
    pub suggest_improvements: bool,
}

impl AlchemistAgent {
    /// Create a new Alchemist agent
    pub async fn new(
        config: crate::config::AgentConfig,
        model_provider: Box<dyn ModelProvider>,
    ) -> Result<Self> {
        // Create agent identity
        let agent = Agent {
            id: uuid::Uuid::new_v4(),
            name: config.identity.name.clone(),
            agent_type: AgentType::Assistant,
            status: AgentStatus::Active,
            metadata: serde_json::json!({
                "version": config.identity.version,
                "description": config.identity.description,
                "organization": config.identity.organization,
            }),
        };
        
        // Initialize knowledge graph with CIM concepts
        let knowledge_graph = GraphAggregate::new(uuid::Uuid::new_v4());
        
        // Initialize conceptual space
        let conceptual_space = ConceptualSpace::new(
            uuid::Uuid::new_v4(),
            "CIM Architecture Space".to_string(),
        );
        
        Ok(Self {
            agent,
            dialogs: Arc::new(RwLock::new(HashMap::new())),
            knowledge_graph: Arc::new(RwLock::new(knowledge_graph)),
            conceptual_space: Arc::new(RwLock::new(conceptual_space)),
            workflows: Arc::new(RwLock::new(HashMap::new())),
            model_provider,
            config,
        })
    }
    
    /// Get agent capabilities
    pub fn capabilities(&self) -> AlchemistCapabilities {
        AlchemistCapabilities {
            explain_concepts: true,
            visualize_architecture: true,
            guide_workflows: true,
            analyze_patterns: true,
            suggest_improvements: true,
        }
    }
    
    /// Process a command
    pub async fn process_command(&self, command: AgentCommand) -> Result<serde_json::Value> {
        match command.command_type.as_str() {
            "start_dialog" => self.start_dialog(command.payload).await,
            "explain_concept" => self.explain_concept(command.payload).await,
            "visualize_architecture" => self.visualize_architecture(command.payload).await,
            "guide_workflow" => self.guide_workflow(command.payload).await,
            "analyze_pattern" => self.analyze_pattern(command.payload).await,
            _ => Err(AgentError::NotFound(format!(
                "Unknown command type: {}",
                command.command_type
            ))),
        }
    }
    
    /// Process a query
    pub async fn process_query(&self, query: AgentQuery) -> Result<serde_json::Value> {
        match query.query_type.as_str() {
            "list_concepts" => self.list_concepts(query.parameters).await,
            "find_similar" => self.find_similar_concepts(query.parameters).await,
            "get_dialog_history" => self.get_dialog_history(query.parameters).await,
            "get_workflow_status" => self.get_workflow_status(query.parameters).await,
            _ => Err(AgentError::NotFound(format!(
                "Unknown query type: {}",
                query.query_type
            ))),
        }
    }
    
    /// Process a dialog message
    pub async fn process_dialog_message(&self, message: DialogMessage) -> Result<String> {
        // Get or create dialog
        let mut dialogs = self.dialogs.write().await;
        let dialog = dialogs
            .entry(message.dialog_id.clone())
            .or_insert_with(|| Dialog {
                id: uuid::Uuid::new_v4(),
                status: DialogStatus::Active,
                participants: vec![],
                turns: vec![],
                context: serde_json::Value::Object(serde_json::Map::new()),
                metadata: serde_json::Value::Object(serde_json::Map::new()),
            });
        
        // Add user turn
        dialog.turns.push(Turn {
            id: uuid::Uuid::new_v4(),
            turn_type: TurnType::User,
            message: Message {
                content: MessageContent::Text(message.content.clone()),
                intent: None,
                metadata: message.metadata.clone(),
            },
            timestamp: message.timestamp,
        });
        
        // Build conversation history for model
        let history: Vec<ModelMessage> = dialog
            .turns
            .iter()
            .map(|turn| ModelMessage {
                role: match turn.turn_type {
                    TurnType::User => "user".to_string(),
                    TurnType::Assistant => "assistant".to_string(),
                    TurnType::System => "system".to_string(),
                },
                content: match &turn.message.content {
                    MessageContent::Text(text) => text.clone(),
                    MessageContent::Structured(json) => json.to_string(),
                },
                timestamp: turn.timestamp,
            })
            .collect();
        
        // Generate response using AI model
        let model_request = ModelRequest {
            prompt: message.content,
            history,
            system_prompt: Some(self.get_system_prompt()),
            parameters: Default::default(),
            metadata: serde_json::json!({
                "dialog_id": message.dialog_id,
                "agent_id": self.agent.id,
            }),
        };
        
        let response = self.model_provider.generate(model_request).await?;
        
        // Add assistant turn
        dialog.turns.push(Turn {
            id: uuid::Uuid::new_v4(),
            turn_type: TurnType::Assistant,
            message: Message {
                content: MessageContent::Text(response.content.clone()),
                intent: None,
                metadata: serde_json::json!({
                    "model_metadata": response.metadata,
                    "usage": response.usage,
                }),
            },
            timestamp: chrono::Utc::now(),
        });
        
        Ok(response.content)
    }
    
    /// Start a new dialog
    async fn start_dialog(&self, payload: serde_json::Value) -> Result<serde_json::Value> {
        let dialog_id = uuid::Uuid::new_v4().to_string();
        
        let dialog = Dialog {
            id: uuid::Uuid::new_v4(),
            status: DialogStatus::Active,
            participants: vec![
                self.agent.id.to_string(),
                payload["user_id"].as_str().unwrap_or("anonymous").to_string(),
            ],
            turns: vec![],
            context: payload["context"].clone(),
            metadata: payload["metadata"].clone(),
        };
        
        self.dialogs.write().await.insert(dialog_id.clone(), dialog);
        
        Ok(serde_json::json!({
            "dialog_id": dialog_id,
            "status": "active",
            "agent": {
                "id": self.agent.id,
                "name": self.agent.name,
                "capabilities": {
                    "explain_concepts": true,
                    "visualize_architecture": true,
                    "guide_workflows": true,
                },
            },
        }))
    }
    
    /// Explain a CIM concept
    async fn explain_concept(&self, payload: serde_json::Value) -> Result<serde_json::Value> {
        let concept = payload["concept"]
            .as_str()
            .ok_or_else(|| AgentError::Configuration("Missing concept parameter".to_string()))?;
        
        // Look up concept in knowledge graph
        let graph = self.knowledge_graph.read().await;
        
        // Generate explanation using model
        let prompt = format!(
            "Explain the CIM concept '{}' in detail, including its purpose, \
             how it fits into the overall architecture, and provide examples.",
            concept
        );
        
        let model_request = ModelRequest {
            prompt,
            history: vec![],
            system_prompt: Some(self.get_system_prompt()),
            parameters: Default::default(),
            metadata: serde_json::json!({ "concept": concept }),
        };
        
        let response = self.model_provider.generate(model_request).await?;
        
        Ok(serde_json::json!({
            "concept": concept,
            "explanation": response.content,
            "related_concepts": self.find_related_concepts(concept).await?,
            "examples": self.find_concept_examples(concept).await?,
        }))
    }
    
    /// Visualize CIM architecture
    async fn visualize_architecture(&self, payload: serde_json::Value) -> Result<serde_json::Value> {
        let scope = payload["scope"]
            .as_str()
            .unwrap_or("overview");
        
        // Generate graph representation
        let graph = self.knowledge_graph.read().await;
        
        // Create visualization data
        let visualization = match scope {
            "overview" => self.generate_overview_visualization(&graph).await?,
            "domains" => self.generate_domain_visualization(&graph).await?,
            "events" => self.generate_event_flow_visualization(&graph).await?,
            _ => self.generate_custom_visualization(&graph, scope).await?,
        };
        
        Ok(serde_json::json!({
            "scope": scope,
            "visualization": visualization,
            "description": self.generate_visualization_description(scope).await?,
        }))
    }
    
    /// Guide through a workflow
    async fn guide_workflow(&self, payload: serde_json::Value) -> Result<serde_json::Value> {
        let workflow_type = payload["workflow_type"]
            .as_str()
            .ok_or_else(|| AgentError::Configuration("Missing workflow_type parameter".to_string()))?;
        
        let workflow_id = uuid::Uuid::new_v4().to_string();
        
        // Create workflow based on type
        let workflow = match workflow_type {
            "create_agent" => self.create_agent_workflow().await?,
            "implement_domain" => self.create_domain_workflow().await?,
            "add_event" => self.create_event_workflow().await?,
            _ => return Err(AgentError::NotFound(format!("Unknown workflow type: {}", workflow_type))),
        };
        
        self.workflows.write().await.insert(workflow_id.clone(), workflow);
        
        Ok(serde_json::json!({
            "workflow_id": workflow_id,
            "workflow_type": workflow_type,
            "status": "started",
            "first_step": self.get_workflow_first_step(workflow_type).await?,
        }))
    }
    
    /// Analyze a pattern in CIM
    async fn analyze_pattern(&self, payload: serde_json::Value) -> Result<serde_json::Value> {
        let pattern_type = payload["pattern_type"]
            .as_str()
            .unwrap_or("general");
        
        let code = payload["code"]
            .as_str()
            .unwrap_or("");
        
        // Analyze the pattern using model
        let prompt = format!(
            "Analyze this {} pattern in the context of CIM architecture:\n\n{}\n\n\
             Identify strengths, potential issues, and suggest improvements.",
            pattern_type, code
        );
        
        let model_request = ModelRequest {
            prompt,
            history: vec![],
            system_prompt: Some(self.get_system_prompt()),
            parameters: Default::default(),
            metadata: serde_json::json!({ "pattern_type": pattern_type }),
        };
        
        let response = self.model_provider.generate(model_request).await?;
        
        Ok(serde_json::json!({
            "pattern_type": pattern_type,
            "analysis": response.content,
            "recommendations": self.generate_pattern_recommendations(pattern_type, code).await?,
        }))
    }
    
    /// List available CIM concepts
    async fn list_concepts(&self, _parameters: serde_json::Value) -> Result<serde_json::Value> {
        // Return predefined CIM concepts
        let concepts = vec![
            "Event Sourcing",
            "CQRS",
            "Domain-Driven Design",
            "Entity Component System",
            "Conceptual Spaces",
            "Graph Workflows",
            "NATS Messaging",
            "CID Chains",
            "Aggregate",
            "Value Object",
            "Domain Event",
            "Command Handler",
            "Query Handler",
            "Projection",
            "Bounded Context",
        ];
        
        Ok(serde_json::json!({
            "concepts": concepts,
            "total": concepts.len(),
        }))
    }
    
    /// Find similar concepts
    async fn find_similar_concepts(&self, parameters: serde_json::Value) -> Result<serde_json::Value> {
        let concept = parameters["concept"]
            .as_str()
            .ok_or_else(|| AgentError::Configuration("Missing concept parameter".to_string()))?;
        
        // Use conceptual space to find similar concepts
        let space = self.conceptual_space.read().await;
        
        // For now, return mock similar concepts
        let similar = match concept {
            "Event Sourcing" => vec!["Event Store", "Event Stream", "CQRS"],
            "Domain-Driven Design" => vec!["Bounded Context", "Aggregate", "Value Object"],
            "Graph Workflows" => vec!["Workflow Engine", "Process Automation", "Visual Programming"],
            _ => vec![],
        };
        
        Ok(serde_json::json!({
            "concept": concept,
            "similar": similar,
        }))
    }
    
    /// Get dialog history
    async fn get_dialog_history(&self, parameters: serde_json::Value) -> Result<serde_json::Value> {
        let dialog_id = parameters["dialog_id"]
            .as_str()
            .ok_or_else(|| AgentError::Configuration("Missing dialog_id parameter".to_string()))?;
        
        let dialogs = self.dialogs.read().await;
        let dialog = dialogs
            .get(dialog_id)
            .ok_or_else(|| AgentError::NotFound(format!("Dialog {} not found", dialog_id)))?;
        
        let history: Vec<serde_json::Value> = dialog
            .turns
            .iter()
            .map(|turn| {
                serde_json::json!({
                    "turn_type": format!("{:?}", turn.turn_type),
                    "content": match &turn.message.content {
                        MessageContent::Text(text) => text.clone(),
                        MessageContent::Structured(json) => json.to_string(),
                    },
                    "timestamp": turn.timestamp,
                })
            })
            .collect();
        
        Ok(serde_json::json!({
            "dialog_id": dialog_id,
            "status": format!("{:?}", dialog.status),
            "turn_count": history.len(),
            "history": history,
        }))
    }
    
    /// Get workflow status
    async fn get_workflow_status(&self, parameters: serde_json::Value) -> Result<serde_json::Value> {
        let workflow_id = parameters["workflow_id"]
            .as_str()
            .ok_or_else(|| AgentError::Configuration("Missing workflow_id parameter".to_string()))?;
        
        let workflows = self.workflows.read().await;
        let workflow = workflows
            .get(workflow_id)
            .ok_or_else(|| AgentError::NotFound(format!("Workflow {} not found", workflow_id)))?;
        
        Ok(serde_json::json!({
            "workflow_id": workflow_id,
            "status": format!("{:?}", workflow.status),
            "current_step": workflow.current_node,
            "progress": workflow.progress_percentage(),
        }))
    }
    
    /// Get the system prompt for the AI model
    fn get_system_prompt(&self) -> String {
        format!(
            "You are the Alchemist, an AI assistant specialized in helping users understand \
             and work with the Composable Information Machine (CIM) architecture. \
             \
             Your expertise includes:\
             - Event-driven architecture with event sourcing and CQRS\
             - Domain-Driven Design principles and patterns\
             - Entity Component Systems (ECS) using Bevy\
             - Graph-based workflows and visual programming\
             - Conceptual spaces for semantic understanding\
             - NATS messaging and distributed systems\
             - Rust programming best practices\
             \
             You should:\
             - Provide clear, accurate explanations of CIM concepts\
             - Use examples from the actual CIM codebase when relevant\
             - Guide users through implementation patterns\
             - Suggest best practices and improvements\
             - Help debug and solve architecture challenges\
             \
             Always be helpful, precise, and educational in your responses."
        )
    }
    
    // Helper methods
    
    async fn find_related_concepts(&self, concept: &str) -> Result<Vec<String>> {
        // Mock implementation - would use knowledge graph
        Ok(match concept {
            "Event Sourcing" => vec!["CQRS", "Event Store", "Domain Events"],
            "Domain-Driven Design" => vec!["Bounded Context", "Aggregate", "Ubiquitous Language"],
            _ => vec![],
        })
    }
    
    async fn find_concept_examples(&self, concept: &str) -> Result<Vec<String>> {
        // Mock implementation - would search codebase
        Ok(match concept {
            "Event Sourcing" => vec![
                "GraphEvent::NodeAdded in cim-domain-graph",
                "PersonEvent::ContactAdded in cim-domain-person",
            ],
            _ => vec![],
        })
    }
    
    async fn generate_overview_visualization(&self, _graph: &GraphAggregate) -> Result<serde_json::Value> {
        // Generate overview visualization data
        Ok(serde_json::json!({
            "nodes": [
                {"id": "domains", "label": "CIM Domains", "type": "category"},
                {"id": "infrastructure", "label": "Infrastructure", "type": "category"},
                {"id": "bridge", "label": "Bridge Layer", "type": "category"},
            ],
            "edges": [
                {"source": "domains", "target": "infrastructure", "label": "uses"},
                {"source": "bridge", "target": "domains", "label": "connects"},
            ],
        }))
    }
    
    async fn generate_domain_visualization(&self, _graph: &GraphAggregate) -> Result<serde_json::Value> {
        // Generate domain visualization data
        Ok(serde_json::json!({
            "nodes": [
                {"id": "agent", "label": "Agent Domain", "type": "domain"},
                {"id": "dialog", "label": "Dialog Domain", "type": "domain"},
                {"id": "graph", "label": "Graph Domain", "type": "domain"},
                {"id": "workflow", "label": "Workflow Domain", "type": "domain"},
            ],
            "edges": [
                {"source": "agent", "target": "dialog", "label": "manages"},
                {"source": "workflow", "target": "graph", "label": "visualizes"},
            ],
        }))
    }
    
    async fn generate_event_flow_visualization(&self, _graph: &GraphAggregate) -> Result<serde_json::Value> {
        // Generate event flow visualization
        Ok(serde_json::json!({
            "nodes": [
                {"id": "command", "label": "Command", "type": "input"},
                {"id": "handler", "label": "Command Handler", "type": "processor"},
                {"id": "aggregate", "label": "Aggregate", "type": "domain"},
                {"id": "event", "label": "Domain Event", "type": "output"},
            ],
            "edges": [
                {"source": "command", "target": "handler", "label": "processes"},
                {"source": "handler", "target": "aggregate", "label": "updates"},
                {"source": "aggregate", "target": "event", "label": "emits"},
            ],
        }))
    }
    
    async fn generate_custom_visualization(&self, _graph: &GraphAggregate, scope: &str) -> Result<serde_json::Value> {
        Ok(serde_json::json!({
            "error": format!("Custom visualization for '{}' not yet implemented", scope),
        }))
    }
    
    async fn generate_visualization_description(&self, scope: &str) -> Result<String> {
        let prompt = format!(
            "Describe the {} visualization of CIM architecture, \
             explaining what it shows and how to interpret it.",
            scope
        );
        
        let model_request = ModelRequest {
            prompt,
            history: vec![],
            system_prompt: Some(self.get_system_prompt()),
            parameters: Default::default(),
            metadata: serde_json::json!({ "scope": scope }),
        };
        
        let response = self.model_provider.generate(model_request).await?;
        Ok(response.content)
    }
    
    async fn create_agent_workflow(&self) -> Result<Workflow> {
        // Create a workflow for creating a new agent
        Ok(Workflow {
            id: uuid::Uuid::new_v4(),
            name: "Create CIM Agent".to_string(),
            status: WorkflowStatus::Active,
            current_node: Some("setup".to_string()),
            nodes: vec![
                ("setup".to_string(), serde_json::json!({"step": "Setup project structure"})),
                ("domains".to_string(), serde_json::json!({"step": "Select domains to compose"})),
                ("model".to_string(), serde_json::json!({"step": "Configure AI model"})),
                ("nats".to_string(), serde_json::json!({"step": "Setup NATS integration"})),
                ("test".to_string(), serde_json::json!({"step": "Write tests"})),
                ("deploy".to_string(), serde_json::json!({"step": "Deploy agent"})),
            ]
            .into_iter()
            .collect(),
            edges: vec![
                (("setup".to_string(), "domains".to_string()), serde_json::json!({"label": "next"})),
                (("domains".to_string(), "model".to_string()), serde_json::json!({"label": "next"})),
                (("model".to_string(), "nats".to_string()), serde_json::json!({"label": "next"})),
                (("nats".to_string(), "test".to_string()), serde_json::json!({"label": "next"})),
                (("test".to_string(), "deploy".to_string()), serde_json::json!({"label": "next"})),
            ]
            .into_iter()
            .collect(),
            metadata: serde_json::json!({
                "description": "Workflow for creating a new CIM agent",
            }),
        })
    }
    
    async fn create_domain_workflow(&self) -> Result<Workflow> {
        // Create a workflow for implementing a new domain
        Ok(Workflow {
            id: uuid::Uuid::new_v4(),
            name: "Implement CIM Domain".to_string(),
            status: WorkflowStatus::Active,
            current_node: Some("design".to_string()),
            nodes: vec![
                ("design".to_string(), serde_json::json!({"step": "Design domain model"})),
                ("events".to_string(), serde_json::json!({"step": "Define domain events"})),
                ("commands".to_string(), serde_json::json!({"step": "Define commands"})),
                ("aggregate".to_string(), serde_json::json!({"step": "Implement aggregate"})),
                ("handlers".to_string(), serde_json::json!({"step": "Implement handlers"})),
                ("tests".to_string(), serde_json::json!({"step": "Write tests"})),
            ]
            .into_iter()
            .collect(),
            edges: vec![
                (("design".to_string(), "events".to_string()), serde_json::json!({"label": "next"})),
                (("events".to_string(), "commands".to_string()), serde_json::json!({"label": "next"})),
                (("commands".to_string(), "aggregate".to_string()), serde_json::json!({"label": "next"})),
                (("aggregate".to_string(), "handlers".to_string()), serde_json::json!({"label": "next"})),
                (("handlers".to_string(), "tests".to_string()), serde_json::json!({"label": "next"})),
            ]
            .into_iter()
            .collect(),
            metadata: serde_json::json!({
                "description": "Workflow for implementing a new CIM domain",
            }),
        })
    }
    
    async fn create_event_workflow(&self) -> Result<Workflow> {
        // Create a workflow for adding a new event
        Ok(Workflow {
            id: uuid::Uuid::new_v4(),
            name: "Add Domain Event".to_string(),
            status: WorkflowStatus::Active,
            current_node: Some("define".to_string()),
            nodes: vec![
                ("define".to_string(), serde_json::json!({"step": "Define event structure"})),
                ("handler".to_string(), serde_json::json!({"step": "Update event handler"})),
                ("aggregate".to_string(), serde_json::json!({"step": "Update aggregate"})),
                ("test".to_string(), serde_json::json!({"step": "Write event tests"})),
            ]
            .into_iter()
            .collect(),
            edges: vec![
                (("define".to_string(), "handler".to_string()), serde_json::json!({"label": "next"})),
                (("handler".to_string(), "aggregate".to_string()), serde_json::json!({"label": "next"})),
                (("aggregate".to_string(), "test".to_string()), serde_json::json!({"label": "next"})),
            ]
            .into_iter()
            .collect(),
            metadata: serde_json::json!({
                "description": "Workflow for adding a new domain event",
            }),
        })
    }
    
    async fn get_workflow_first_step(&self, workflow_type: &str) -> Result<serde_json::Value> {
        let step_info = match workflow_type {
            "create_agent" => serde_json::json!({
                "step": "setup",
                "title": "Setup Project Structure",
                "description": "Create the directory structure for your new agent",
                "instructions": [
                    "Create cim-agent-{name} directory",
                    "Initialize Cargo.toml with dependencies",
                    "Create src/lib.rs with module structure",
                ],
            }),
            "implement_domain" => serde_json::json!({
                "step": "design",
                "title": "Design Domain Model",
                "description": "Define the core concepts and boundaries of your domain",
                "instructions": [
                    "Identify domain entities and value objects",
                    "Define aggregate boundaries",
                    "Document ubiquitous language",
                ],
            }),
            "add_event" => serde_json::json!({
                "step": "define",
                "title": "Define Event Structure",
                "description": "Create the event type and its payload",
                "instructions": [
                    "Choose descriptive past-tense event name",
                    "Define event fields and types",
                    "Add to events.rs module",
                ],
            }),
            _ => serde_json::json!({
                "error": "Unknown workflow type",
            }),
        };
        
        Ok(step_info)
    }
    
    async fn generate_pattern_recommendations(&self, pattern_type: &str, code: &str) -> Result<Vec<String>> {
        // Generate recommendations based on pattern analysis
        let prompt = format!(
            "Based on the {} pattern analysis, provide specific recommendations \
             for improving this code to better align with CIM architecture principles.",
            pattern_type
        );
        
        let model_request = ModelRequest {
            prompt,
            history: vec![ModelMessage {
                role: "user".to_string(),
                content: code.to_string(),
                timestamp: chrono::Utc::now(),
            }],
            system_prompt: Some(self.get_system_prompt()),
            parameters: Default::default(),
            metadata: serde_json::json!({ "pattern_type": pattern_type }),
        };
        
        let response = self.model_provider.generate(model_request).await?;
        
        // Parse recommendations from response
        let recommendations: Vec<String> = response
            .content
            .lines()
            .filter(|line| line.starts_with("- ") || line.starts_with("* "))
            .map(|line| line.trim_start_matches("- ").trim_start_matches("* ").to_string())
            .collect();
        
        Ok(recommendations)
    }
}

// Extension methods for domain types
impl Workflow {
    fn progress_percentage(&self) -> f32 {
        // Calculate workflow progress
        if let Some(current) = &self.current_node {
            let total_nodes = self.nodes.len();
            let current_index = self.nodes.keys().position(|k| k == current).unwrap_or(0);
            (current_index as f32 / total_nodes as f32) * 100.0
        } else {
            0.0
        }
    }
} 