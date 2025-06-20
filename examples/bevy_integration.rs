//! Example of integrating the Alchemist Agent into a Bevy application

use bevy::prelude::*;
use cim_agent_alchemist::{
    AlchemistAgentPlugin, 
    AgentQuestionEvent, 
    AgentResponseEvent,
    AgentErrorEvent,
    ask_agent,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(AlchemistAgentPlugin)
        .add_systems(Startup, setup)
        .add_systems(Update, (
            handle_keyboard_input,
            display_agent_responses,
        ))
        .run();
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    // Camera
    commands.spawn(Camera2d);

    // UI for agent interaction
    commands
        .spawn(Node {
            style: Style {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            ..default()
        })
        .with_children(|parent| {
            // Title
            parent.spawn((
                Text::new("CIM Alchemist Agent Demo"),
                TextFont {
                    font_size: 40.0,
                    ..default()
                },
                TextColor(Color::WHITE),
            ));

            // Instructions
            parent.spawn((
                Text::new("\nPress keys to ask questions:\n\n\
                          F1 - What is CIM?\n\
                          F2 - Explain event sourcing\n\
                          F3 - What are the 8 domains?\n\
                          F4 - How does NATS work with CIM?\n"),
                TextFont {
                    font_size: 20.0,
                    ..default()
                },
                TextColor(Color::srgb(0.8, 0.8, 0.8)),
            ));

            // Response area
            parent.spawn((
                Text::new("\nAgent Response:\n"),
                TextFont {
                    font_size: 24.0,
                    ..default()
                },
                TextColor(Color::srgb(0.5, 0.8, 0.5)),
                AgentResponseText,
            ));
        });
}

#[derive(Component)]
struct AgentResponseText;

fn handle_keyboard_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut events: EventWriter<AgentQuestionEvent>,
) {
    if keyboard.just_pressed(KeyCode::F1) {
        ask_agent("What is CIM?".to_string(), events);
    }
    
    if keyboard.just_pressed(KeyCode::F2) {
        ask_agent("Can you explain event sourcing in CIM?".to_string(), events);
    }
    
    if keyboard.just_pressed(KeyCode::F3) {
        ask_agent("What are the 8 production-ready domains in CIM?".to_string(), events);
    }
    
    if keyboard.just_pressed(KeyCode::F4) {
        ask_agent("How does NATS integrate with CIM architecture?".to_string(), events);
    }
}

fn display_agent_responses(
    mut response_events: EventReader<AgentResponseEvent>,
    mut error_events: EventReader<AgentErrorEvent>,
    mut query: Query<&mut Text, With<AgentResponseText>>,
) {
    for response in response_events.read() {
        info!("Got agent response: {}", response.response);
        
        for mut text in query.iter_mut() {
            text.0 = format!("\nAgent Response:\n\n{}", response.response);
        }
    }
    
    for error in error_events.read() {
        error!("Agent error: {}", error.error);
        
        for mut text in query.iter_mut() {
            text.0 = format!("\nAgent Error:\n\n{}", error.error);
        }
    }
} 