//! Alchemist Agent - CIM Architecture Assistant
//!
//! This is the main entry point for running the Alchemist agent service.

use cim_agent_alchemist::{AgentConfig, service};
use clap::Parser;
use std::path::PathBuf;
use tracing::error;

/// Command-line arguments for the Alchemist agent
#[derive(Parser, Debug)]
#[command(
    name = "alchemist",
    about = "CIM Architecture Assistant Agent",
    version,
    author
)]
struct Args {
    /// Configuration file path
    #[arg(short, long, value_name = "FILE")]
    config: Option<PathBuf>,
    
    /// NATS server URL (overrides config)
    #[arg(long, value_name = "URL")]
    nats_url: Option<String>,
    
    /// AI model to use (overrides config)
    #[arg(long, value_name = "MODEL")]
    model: Option<String>,
    
    /// Log level (trace, debug, info, warn, error)
    #[arg(long, value_name = "LEVEL", default_value = "info")]
    log_level: String,
    
    /// Print default configuration and exit
    #[arg(long)]
    print_config: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    
    // Print default config if requested
    if args.print_config {
        let default_config = AgentConfig::default();
        let yaml = serde_yaml::to_string(&default_config)?;
        println!("{}", yaml);
        return Ok(());
    }
    
    // Load configuration
    let mut config = if let Some(config_path) = args.config {
        load_config_from_file(config_path)?
    } else {
        AgentConfig::default()
    };
    
    // Apply command-line overrides
    if let Some(nats_url) = args.nats_url {
        config.nats.servers = vec![nats_url];
    }
    
    if let Some(model) = args.model {
        if let cim_agent_alchemist::ModelConfig::Ollama { ref mut model as m, .. } = &mut config.model {
            *m = model;
        }
    }
    
    config.service.logging.level = args.log_level;
    
    // Print startup banner
    print_banner();
    
    // Run the service
    match service::run(config).await {
        Ok(()) => {
            println!("Agent service completed successfully");
            Ok(())
        }
        Err(e) => {
            error!("Agent service failed: {}", e);
            Err(e.into())
        }
    }
}

/// Load configuration from file
fn load_config_from_file(path: PathBuf) -> Result<AgentConfig, Box<dyn std::error::Error>> {
    let contents = std::fs::read_to_string(&path)?;
    
    let config = if path.extension().map_or(false, |ext| ext == "yaml" || ext == "yml") {
        serde_yaml::from_str(&contents)?
    } else if path.extension().map_or(false, |ext| ext == "json") {
        serde_json::from_str(&contents)?
    } else if path.extension().map_or(false, |ext| ext == "toml") {
        toml::from_str(&contents)?
    } else {
        // Try to detect format
        if contents.trim_start().starts_with('{') {
            serde_json::from_str(&contents)?
        } else if contents.contains(':') && !contents.contains('=') {
            serde_yaml::from_str(&contents)?
        } else {
            toml::from_str(&contents)?
        }
    };
    
    Ok(config)
}

/// Print startup banner
fn print_banner() {
    println!(r#"
     _    _      _                    _     _   
    / \  | | ___| |__   ___ _ __ ___ (_)___| |_ 
   / _ \ | |/ __| '_ \ / _ \ '_ ` _ \| / __| __|
  / ___ \| | (__| | | |  __/ | | | | | \__ \ |_ 
 /_/   \_\_|\___|_| |_|\___|_| |_| |_|_|___/\__|
                                                 
 CIM Architecture Assistant v{}
 
"#, cim_agent_alchemist::VERSION);
} 