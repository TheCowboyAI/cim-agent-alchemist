//! Bevy plugin for CIM Alchemist Agent
//! 
//! This plugin integrates the AI assistant into the Bevy ECS system,
//! allowing it to interact with the graph editor and workflow components.

use bevy::prelude::*;
use crate::{agent::AlchemistAgent, config::Config, error::Result};
use crate::model::ModelProvider;
use crate::nats_integration::NatsClient;
use std::sync::Arc;
use tokio::runtime::Runtime;
use crossbeam_channel::{bounded, Receiver, Sender};

/// Events for agent communication
#[derive(Event, Debug, Clone)]
pub struct AgentQuestionEvent {
    pub id: String,
    pub question: String,
}

#[derive(Event, Debug, Clone)]
pub struct AgentResponseEvent {
    pub id: String,
    pub response: String,
    pub question_id: String,
}

#[derive(Event, Debug, Clone)]
pub struct AgentErrorEvent {
    pub error: String,
}

/// Resource for agent configuration
#[derive(Resource)]
pub struct AgentConfig {
    pub nats_url: String,
    pub ollama_url: String,
    pub model_name: String,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            nats_url: "nats://localhost:4222".to_string(),
            ollama_url: "http://localhost:11434".to_string(),
            model_name: "vicuna:latest".to_string(),
        }
    }
}

/// Resource for the async runtime
#[derive(Resource)]
struct AgentRuntime {
    runtime: Arc<Runtime>,
}

/// Channel for communication between Bevy and async agent
#[derive(Resource)]
struct AgentChannels {
    question_sender: Sender<AgentQuestionEvent>,
    response_receiver: Receiver<AgentResponseEvent>,
    error_receiver: Receiver<AgentErrorEvent>,
}

/// Component for agent UI elements
#[derive(Component)]
pub struct AgentChatUI;

#[derive(Component)]
pub struct AgentInputField;

#[derive(Component)]
pub struct AgentResponseDisplay;

/// Plugin for the CIM Alchemist Agent
pub struct AlchemistAgentPlugin;

impl Plugin for AlchemistAgentPlugin {
    fn build(&self, app: &mut App) {
        // Create async runtime
        let runtime = Arc::new(
            tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .expect("Failed to create Tokio runtime")
        );

        // Create channels
        let (question_tx, question_rx) = bounded::<AgentQuestionEvent>(100);
        let (response_tx, response_rx) = bounded::<AgentResponseEvent>(100);
        let (error_tx, error_rx) = bounded::<AgentErrorEvent>(100);

        app
            // Resources
            .insert_resource(AgentConfig::default())
            .insert_resource(AgentRuntime { runtime: runtime.clone() })
            .insert_resource(AgentChannels {
                question_sender: question_tx,
                response_receiver: response_rx,
                error_receiver: error_rx,
            })
            // Events
            .add_event::<AgentQuestionEvent>()
            .add_event::<AgentResponseEvent>()
            .add_event::<AgentErrorEvent>()
            // Systems
            .add_systems(Startup, setup_agent_service)
            .add_systems(Update, (
                handle_question_events,
                poll_agent_responses,
                poll_agent_errors,
                update_agent_ui,
            ).chain());

        // Start the agent service in the background
        let runtime_clone = runtime.clone();
        let response_sender = response_tx;
        let error_sender = error_tx;
        let question_receiver = question_rx;

        runtime.spawn(async move {
            if let Err(e) = run_agent_service(
                question_receiver,
                response_sender,
                error_sender,
            ).await {
                error!("Agent service failed: {}", e);
            }
        });
    }
}

/// Setup the agent service
fn setup_agent_service(
    mut commands: Commands,
    config: Res<AgentConfig>,
) {
    info!("Setting up CIM Alchemist Agent service");
    
    // Spawn UI entities (placeholder - customize based on your UI needs)
    commands.spawn((
        AgentChatUI,
        Name::new("Agent Chat UI"),
    ));
}

/// Handle question events from the UI
fn handle_question_events(
    mut events: EventReader<AgentQuestionEvent>,
    channels: Res<AgentChannels>,
) {
    for event in events.read() {
        if let Err(e) = channels.question_sender.try_send(event.clone()) {
            error!("Failed to send question to agent: {}", e);
        }
    }
}

/// Poll for responses from the agent
fn poll_agent_responses(
    channels: Res<AgentChannels>,
    mut response_events: EventWriter<AgentResponseEvent>,
) {
    while let Ok(response) = channels.response_receiver.try_recv() {
        response_events.send(response);
    }
}

/// Poll for errors from the agent
fn poll_agent_errors(
    channels: Res<AgentChannels>,
    mut error_events: EventWriter<AgentErrorEvent>,
) {
    while let Ok(error) = channels.error_receiver.try_recv() {
        error_events.send(error);
    }
}

/// Update the agent UI based on events
fn update_agent_ui(
    mut response_events: EventReader<AgentResponseEvent>,
    mut error_events: EventReader<AgentErrorEvent>,
) {
    for response in response_events.read() {
        info!("Agent response: {}", response.response);
        // TODO: Update UI components with response
    }

    for error in error_events.read() {
        error!("Agent error: {}", error.error);
        // TODO: Show error in UI
    }
}

/// Run the agent service in the background
async fn run_agent_service(
    question_receiver: Receiver<AgentQuestionEvent>,
    response_sender: Sender<AgentResponseEvent>,
    error_sender: Sender<AgentErrorEvent>,
) -> Result<()> {
    use crate::model::OllamaProvider;
    
    // Initialize the model provider
    let model_provider = Arc::new(OllamaProvider::new(
        "http://localhost:11434".to_string(),
        "vicuna:latest".to_string(),
    ));

    // Initialize NATS client (optional - can be disabled for pure Bevy usage)
    // let nats_client = NatsClient::connect("nats://localhost:4222").await?;

    // Create the agent
    let agent = AlchemistAgent::new(
        cim_domain_agent::aggregate::Agent::default(),
        model_provider,
    );

    // Main service loop
    loop {
        // Check for questions from Bevy
        if let Ok(question) = question_receiver.try_recv() {
            match agent.process_question(&question.question).await {
                Ok(response) => {
                    let response_event = AgentResponseEvent {
                        id: uuid::Uuid::new_v4().to_string(),
                        response,
                        question_id: question.id,
                    };
                    
                    if let Err(e) = response_sender.send(response_event) {
                        error!("Failed to send response: {}", e);
                    }
                }
                Err(e) => {
                    let error_event = AgentErrorEvent {
                        error: format!("Failed to process question: {}", e),
                    };
                    
                    if let Err(e) = error_sender.send(error_event) {
                        error!("Failed to send error: {}", e);
                    }
                }
            }
        }

        // Small delay to prevent busy waiting
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    }
}

/// Helper function to send a question to the agent
pub fn ask_agent(
    question: String,
    mut events: EventWriter<AgentQuestionEvent>,
) {
    events.send(AgentQuestionEvent {
        id: uuid::Uuid::new_v4().to_string(),
        question,
    });
}

/// System to handle keyboard input for agent questions
pub fn handle_agent_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut events: EventWriter<AgentQuestionEvent>,
) {
    // Example: Press F1 to ask about CIM
    if keyboard.just_pressed(KeyCode::F1) {
        ask_agent("What is CIM?".to_string(), events);
    }
    
    // Example: Press F2 to ask about current graph
    if keyboard.just_pressed(KeyCode::F2) {
        ask_agent("Can you explain the current graph structure?".to_string(), events);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_config_default() {
        let config = AgentConfig::default();
        assert_eq!(config.nats_url, "nats://localhost:4222");
        assert_eq!(config.ollama_url, "http://localhost:11434");
        assert_eq!(config.model_name, "vicuna:latest");
    }
} 