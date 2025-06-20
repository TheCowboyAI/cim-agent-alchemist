//! Simple test of the Alchemist agent with NATS and Ollama

use async_nats;
use futures::StreamExt;
use reqwest;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::time::Duration;
use tokio;
use uuid;

#[derive(Serialize)]
struct OllamaGenerateRequest {
    model: String,
    prompt: String,
    stream: bool,
}

#[derive(Deserialize)]
struct OllamaGenerateResponse {
    response: String,
    done: bool,
}

#[derive(Serialize, Deserialize, Debug)]
struct AgentMessage {
    id: String,
    content: String,
    timestamp: chrono::DateTime<chrono::Utc>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    println!("CIM Alchemist Agent - Simple Test");
    println!("==================================\n");

    // Test 1: Check Ollama connection
    println!("1. Testing Ollama connection...");
    let client = reqwest::Client::new();
    
    match client.get("http://localhost:11434/api/tags").send().await {
        Ok(response) => {
            if response.status().is_success() {
                println!("   ✓ Ollama is running");
                let body: serde_json::Value = response.json().await?;
                if let Some(models) = body["models"].as_array() {
                    println!("   ✓ Available models:");
                    for model in models {
                        if let Some(name) = model["name"].as_str() {
                            println!("     - {}", name);
                        }
                    }
                }
            } else {
                println!("   ✗ Ollama returned error: {}", response.status());
            }
        }
        Err(e) => {
            println!("   ✗ Failed to connect to Ollama: {}", e);
            println!("   Make sure Ollama is running on http://localhost:11434");
        }
    }

    // Test 2: Check NATS connection
    println!("\n2. Testing NATS connection...");
    match async_nats::connect("nats://localhost:4222").await {
        Ok(nats_client) => {
            println!("   ✓ Connected to NATS");

            // Test 3: Test Ollama generation
            println!("\n3. Testing AI generation with Ollama...");
            let request = OllamaGenerateRequest {
                model: "vicuna:latest".to_string(),
                prompt: "What is the Composable Information Machine (CIM)? Give a brief answer.".to_string(),
                stream: false,
            };

            match client
                .post("http://localhost:11434/api/generate")
                .json(&request)
                .send()
                .await
            {
                Ok(response) => {
                    if response.status().is_success() {
                        let ollama_response: OllamaGenerateResponse = response.json().await?;
                        println!("   ✓ AI Response:");
                        println!("   {}", ollama_response.response);
                    } else {
                        println!("   ✗ Ollama generation failed: {}", response.status());
                    }
                }
                Err(e) => {
                    println!("   ✗ Failed to generate response: {}", e);
                }
            }

            // Test 4: Test NATS messaging
            println!("\n4. Testing NATS pub/sub...");
            
            // Subscribe to a test subject
            let mut sub = nats_client.subscribe("test.alchemist").await?;
            println!("   ✓ Subscribed to test.alchemist");

            // Publish a test message
            let message = AgentMessage {
                id: uuid::Uuid::new_v4().to_string(),
                content: "Hello from Alchemist agent!".to_string(),
                timestamp: chrono::Utc::now(),
            };

            let payload = serde_json::to_vec(&message)?;
            nats_client.publish("test.alchemist", payload.into()).await?;
            println!("   ✓ Published test message");

            // Receive the message
            if let Ok(Some(msg)) = tokio::time::timeout(Duration::from_secs(2), sub.next()).await {
                let received: AgentMessage = serde_json::from_slice(&msg.payload)?;
                println!("   ✓ Received message: {:?}", received);
            } else {
                println!("   ✗ Timeout waiting for message");
            }

            // Test 5: Simulate agent dialog
            println!("\n5. Simulating agent dialog over NATS...");
            
            // Create dialog subjects
            let dialog_request = "cim.dialog.alchemist.request";
            let dialog_response = "cim.dialog.alchemist.response";
            
            // Subscribe to responses
            let mut response_sub = nats_client.subscribe(dialog_response).await?;
            
            // Simulate dialog processor
            let nats_clone = nats_client.clone();
            let client_clone = client.clone();
            tokio::spawn(async move {
                let mut request_sub = nats_clone.subscribe(dialog_request).await.unwrap();
                
                while let Some(msg) = request_sub.next().await {
                    if let Ok(request) = serde_json::from_slice::<AgentMessage>(&msg.payload) {
                        // Generate AI response
                        let ollama_request = OllamaGenerateRequest {
                            model: "vicuna:latest".to_string(),
                            prompt: request.content,
                            stream: false,
                        };
                        
                        if let Ok(response) = client_clone
                            .post("http://localhost:11434/api/generate")
                            .json(&ollama_request)
                            .send()
                            .await
                        {
                            if let Ok(ollama_response) = response.json::<OllamaGenerateResponse>().await {
                                let reply = AgentMessage {
                                    id: uuid::Uuid::new_v4().to_string(),
                                    content: ollama_response.response,
                                    timestamp: chrono::Utc::now(),
                                };
                                
                                let payload = serde_json::to_vec(&reply).unwrap();
                                let _ = nats_clone.publish(dialog_response, payload.into()).await;
                            }
                        }
                    }
                }
            });
            
            // Send a dialog request
            let dialog_message = AgentMessage {
                id: uuid::Uuid::new_v4().to_string(),
                content: "What is event sourcing in one sentence?".to_string(),
                timestamp: chrono::Utc::now(),
            };
            
            let payload = serde_json::to_vec(&dialog_message)?;
            nats_client.publish(dialog_request, payload.into()).await?;
            println!("   ✓ Sent dialog request: {}", dialog_message.content);
            
            // Wait for response
            if let Ok(Some(msg)) = tokio::time::timeout(Duration::from_secs(10), response_sub.next()).await {
                let response: AgentMessage = serde_json::from_slice(&msg.payload)?;
                println!("   ✓ Received AI response: {}", response.content);
            } else {
                println!("   ✗ Timeout waiting for AI response");
            }

            println!("\n✅ All tests completed!");
        }
        Err(e) => {
            println!("   ✗ Failed to connect to NATS: {}", e);
            println!("   Make sure NATS is running on nats://localhost:4222");
        }
    }

    Ok(())
} 