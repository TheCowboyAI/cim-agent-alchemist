//! Core Alchemist agent implementation
//!
//! This module implements the main agent logic that composes multiple CIM domains
//! to provide intelligent assistance for understanding CIM architecture.

use crate::error::{AgentError, Result};
use crate::model::{ModelProvider, Message as ModelMessage};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

// Domain imports
use cim_domain_agent::aggregate::Agent;
use cim_domain_agent::commands::AgentCommand;
use cim_domain_agent::queries::AgentQuery;
use cim_domain_dialog::aggregate::{Dialog, DialogStatus};
use cim_domain_dialog::value_objects::{Message, MessageContent, Turn, TurnType};
use cim_domain_graph::aggregate::GraphAggregate;
use cim_domain_conceptualspaces::aggregate::ConceptualSpace;
use cim_domain_workflow::aggregate::{Workflow, WorkflowStatus};

/// The Alchemist agent - helps users understand and work with CIM
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
#[derive(Debug, Clone, serde::Serialize)]
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
            name: config.name.clone(),
            description: config.description.clone(),
            capabilities: vec![
                "explain_concepts".to_string(),
                "visualize_architecture".to_string(),
                "guide_workflows".to_string(),
                "analyze_patterns".to_string(),
                "suggest_improvements".to_string(),
            ],
            metadata: serde_json::json!({
                "version": env!("CARGO_PKG_VERSION"),
                "type": "alchemist",
            }),
        };
        
        Ok(Self {
            agent,
            dialogs: Arc::new(RwLock::new(HashMap::new())),
            knowledge_graph: Arc::new(RwLock::new(GraphAggregate::new(
                uuid::Uuid::new_v4(),
                "CIM Knowledge Graph".to_string(),
            ))),
            conceptual_space: Arc::new(RwLock::new(ConceptualSpace::new(
                uuid::Uuid::new_v4(),
                "CIM Conceptual Space".to_string(),
            ))),
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
    
    /// Process an agent command
    pub async fn process_command(&self, command: AgentCommand) -> Result<serde_json::Value> {
        match command {
            AgentCommand::ExplainConcept { payload } => self.explain_concept(payload).await,
            AgentCommand::VisualizeArchitecture { payload } => self.visualize_architecture(payload).await,
            AgentCommand::GuideWorkflow { payload } => self.guide_workflow(payload).await,
            AgentCommand::AnalyzePattern { payload } => self.analyze_pattern(payload).await,
            _ => Err(AgentError::InvalidRequest("Unknown command".to_string())),
        }
    }
    
    /// Process an agent query
    pub async fn process_query(&self, query: AgentQuery) -> Result<serde_json::Value> {
        match query {
            AgentQuery::ListConcepts { parameters } => self.list_concepts(parameters).await,
            AgentQuery::FindSimilarConcepts { parameters } => self.find_similar_concepts(parameters).await,
            AgentQuery::GetDialogHistory { parameters } => self.get_dialog_history(parameters).await,
            AgentQuery::GetWorkflowStatus { parameters } => self.get_workflow_status(parameters).await,
            _ => Err(AgentError::InvalidRequest("Unknown query".to_string())),
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
            })
            .collect();
        
        // Add system prompt as first message if history is empty
        let mut context = vec![ModelMessage {
            role: "system".to_string(),
            content: self.get_system_prompt(),
        }];
        context.extend(history);
        
        // Generate response using AI model
        let response = self.model_provider
            .generate_with_context(&message.content, &context)
            .await?;
        
        // Add assistant turn
        dialog.turns.push(Turn {
            id: uuid::Uuid::new_v4(),
            turn_type: TurnType::Assistant,
            message: Message {
                content: MessageContent::Text(response.clone()),
                intent: None,
                metadata: serde_json::json!({
                    "model": self.config.model_provider.model_name(),
                    "agent_id": self.agent.id,
                }),
            },
            timestamp: chrono::Utc::now(),
        });
        
        Ok(response)
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
        let _graph = self.knowledge_graph.read().await;
        
        // Generate explanation using model
        let prompt = format!(
            "Explain the CIM concept '{}' in detail, including its purpose, \
             how it fits into the overall architecture, and provide examples.",
            concept
        );
        
        let response = self.model_provider.generate(&prompt).await?;
        
        Ok(serde_json::json!({
            "concept": concept,
            "explanation": response,
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
            _ => return Err(AgentError::Domain(format!("Unknown workflow type: {}", workflow_type))),
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
        
        let response = self.model_provider.generate(&prompt).await?;
        
        Ok(serde_json::json!({
            "pattern_type": pattern_type,
            "analysis": response,
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
        let _space = self.conceptual_space.read().await;
        
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
            .ok_or_else(|| AgentError::Domain(format!("Dialog {} not found", dialog_id)))?;
        
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
            .ok_or_else(|| AgentError::Domain(format!("Workflow {} not found", workflow_id)))?;
        
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
        
        let response = self.model_provider.generate(&prompt).await?;
        Ok(response)
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
                ("handler".to_string(), serde_json::json!({"step": "Create event handler"})),
                ("test".to_string(), serde_json::json!({"step": "Write event tests"})),
                ("integrate".to_string(), serde_json::json!({"step": "Integrate with aggregate"})),
            ]
            .into_iter()
            .collect(),
            edges: vec![
                (("define".to_string(), "handler".to_string()), serde_json::json!({"label": "next"})),
                (("handler".to_string(), "test".to_string()), serde_json::json!({"label": "next"})),
                (("test".to_string(), "integrate".to_string()), serde_json::json!({"label": "next"})),
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
                "description": "Create a new cim-agent-* directory with the standard structure",
                "actions": [
                    "Create Cargo.toml with dependencies",
                    "Set up src/ directory structure",
                    "Create configuration templates",
                    "Initialize git repository",
                ],
            }),
            "implement_domain" => serde_json::json!({
                "step": "design",
                "title": "Design Domain Model",
                "description": "Define the domain boundaries and core concepts",
                "actions": [
                    "Identify aggregates and entities",
                    "Define value objects",
                    "Map relationships",
                    "Document ubiquitous language",
                ],
            }),
            "add_event" => serde_json::json!({
                "step": "define",
                "title": "Define Event Structure",
                "description": "Create the event type and its properties",
                "actions": [
                    "Choose event name (past tense)",
                    "Define event payload",
                    "Add serialization derives",
                    "Document event purpose",
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
            "Based on this {} pattern:\n\n{}\n\n\
             Provide 3-5 specific recommendations for improvement in the context of CIM architecture.",
            pattern_type, code
        );
        
        let response = self.model_provider.generate(&prompt).await?;
        
        // Parse recommendations from response
        let recommendations: Vec<String> = response
            .lines()
            .filter(|line| line.trim().starts_with("- ") || line.trim().starts_with("* "))
            .map(|line| line.trim_start_matches("- ").trim_start_matches("* ").to_string())
            .collect();
        
        if recommendations.is_empty() {
            Ok(vec![
                "Consider using event sourcing for state changes".to_string(),
                "Ensure proper separation between commands and queries".to_string(),
                "Add appropriate error handling".to_string(),
            ])
        } else {
            Ok(recommendations)
        }
    }
}

// Dialog message for conversations
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DialogMessage {
    pub dialog_id: String,
    pub content: String,
    pub metadata: serde_json::Value,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl Workflow {
    fn progress_percentage(&self) -> f32 {
        if self.nodes.is_empty() {
            return 0.0;
        }
        
        // Simple progress calculation based on current node position
        if let Some(current) = &self.current_node {
            let node_keys: Vec<_> = self.nodes.keys().collect();
            if let Some(pos) = node_keys.iter().position(|k| k == &current) {
                return ((pos + 1) as f32 / node_keys.len() as f32) * 100.0;
            }
        }
        
        0.0
    }
} 