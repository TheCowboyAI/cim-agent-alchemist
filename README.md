# CIM Alchemist Agent

The Alchemist is a specialized CIM agent designed to help users understand and navigate the Composable Information Machine architecture. It combines multiple CIM domains to provide intelligent assistance through natural language interaction.

## Features

- **Explain CIM Concepts**: Detailed explanations of Event Sourcing, CQRS, DDD, ECS, and more
- **Visualize Architecture**: Generate graph visualizations of CIM components and relationships
- **Guide Workflows**: Step-by-step guidance for implementing CIM patterns
- **Analyze Patterns**: Review code and suggest improvements based on CIM best practices
- **Interactive Dialog**: Natural conversation flow with context preservation

## Architecture

The Alchemist agent is built using CIM's composition pattern, integrating:

- **Agent Domain**: Core agent lifecycle and capabilities
- **Dialog Domain**: Conversation management and context tracking
- **Identity Domain**: User and agent identity management
- **Graph Domain**: Visual representation of concepts
- **Conceptual Spaces**: Semantic understanding
- **Workflow Domain**: Guided processes

## Quick Start

### Prerequisites

- Rust 1.75+
- NATS Server 2.10+
- Ollama (for AI model serving)

### Installation

1. Start NATS server:
```bash
nats-server -js
```

2. Start Ollama with Vicuna model:
```bash
ollama serve
ollama pull vicuna
```

3. Build and run the agent:
```bash
cargo build --release
cargo run --release
```

### Configuration

Create a `config.yaml` file (see `examples/config.yaml` for reference):

```yaml
identity:
  name: "Alchemist"
  description: "CIM Architecture Assistant"

model:
  provider: "Ollama"
  base_url: "http://localhost:11434"
  model: "vicuna"

nats:
  servers:
    - "nats://localhost:4222"
  subject_prefix: "cim.agent.alchemist"
```

Run with custom config:
```bash
cargo run -- --config config.yaml
```

## Usage

### Command Line Options

```bash
alchemist [OPTIONS]

OPTIONS:
    -c, --config <FILE>         Configuration file path
        --nats-url <URL>        NATS server URL (overrides config)
        --model <MODEL>         AI model to use (overrides config)
        --log-level <LEVEL>     Log level (trace, debug, info, warn, error)
        --print-config          Print default configuration and exit
    -h, --help                  Print help
    -V, --version               Print version
```

### NATS Interaction

The agent listens on several NATS subjects:

#### Commands
Send commands to `cim.agent.alchemist.commands.*`:

```json
{
  "id": "cmd-123",
  "command_type": "explain_concept",
  "payload": {
    "concept": "Event Sourcing"
  },
  "timestamp": "2024-01-15T10:00:00Z",
  "origin": "user-456"
}
```

Available commands:
- `start_dialog`: Start a new conversation
- `explain_concept`: Get detailed explanation of a CIM concept
- `visualize_architecture`: Generate architecture visualization
- `guide_workflow`: Start a guided workflow
- `analyze_pattern`: Analyze code pattern

#### Queries
Send queries to `cim.agent.alchemist.queries.*` (request-reply pattern):

```json
{
  "id": "qry-789",
  "query_type": "list_concepts",
  "parameters": {},
  "timestamp": "2024-01-15T10:00:00Z",
  "origin": "user-456"
}
```

Available queries:
- `list_concepts`: List available CIM concepts
- `find_similar`: Find concepts similar to a given one
- `get_dialog_history`: Retrieve conversation history
- `get_workflow_status`: Check workflow progress

#### Dialog
Send dialog messages to `cim.dialog.alchemist.*`:

```json
{
  "dialog_id": "dlg-abc",
  "content": "What is Event Sourcing?",
  "sender": "user-456",
  "metadata": {},
  "timestamp": "2024-01-15T10:00:00Z"
}
```

### Health Check

Check agent health at `cim.agent.alchemist.health`:

```bash
nats request cim.agent.alchemist.health ""
```

Response:
```json
{
  "status": "Running",
  "version": "0.1.0",
  "uptime_seconds": 3600,
  "model_status": "healthy",
  "active_dialogs": 2,
  "metadata": {
    "agent_name": "alchemist",
    "capabilities": {
      "explain_concepts": true,
      "visualize_architecture": true,
      "guide_workflows": true,
      "analyze_patterns": true,
      "suggest_improvements": true
    }
  }
}
```

## Development

### Project Structure

```
cim-agent-alchemist/
├── src/
│   ├── lib.rs              # Library root
│   ├── main.rs             # Binary entry point
│   ├── agent.rs            # Core agent implementation
│   ├── config.rs           # Configuration types
│   ├── error.rs            # Error handling
│   ├── model.rs            # AI model integration
│   ├── nats_integration.rs # NATS messaging
│   └── service.rs          # Service orchestration
├── examples/
│   └── config.yaml         # Example configuration
├── tests/
│   └── integration.rs      # Integration tests
├── Cargo.toml
└── README.md
```

### Testing

Run unit tests:
```bash
cargo test
```

Run integration tests (requires NATS):
```bash
cargo test --test integration
```

### Adding New Capabilities

1. **Add Command Handler** in `agent.rs`:
```rust
match command.command_type.as_str() {
    "new_command" => self.handle_new_command(command.payload).await,
    // ...
}
```

2. **Implement Handler Method**:
```rust
async fn handle_new_command(&self, payload: serde_json::Value) -> Result<serde_json::Value> {
    // Implementation
}
```

3. **Update Documentation** in README and code comments

## Docker Deployment

Build Docker image:
```dockerfile
FROM rust:1.75 as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates
COPY --from=builder /app/target/release/alchemist /usr/local/bin/
CMD ["alchemist"]
```

Run with Docker Compose:
```yaml
version: '3.8'
services:
  nats:
    image: nats:latest
    command: "-js"
    ports:
      - "4222:4222"
  
  ollama:
    image: ollama/ollama:latest
    ports:
      - "11434:11434"
    volumes:
      - ollama_data:/root/.ollama
  
  alchemist:
    build: .
    depends_on:
      - nats
      - ollama
    environment:
      - RUST_LOG=info
    volumes:
      - ./config.yaml:/config.yaml
    command: ["alchemist", "--config", "/config.yaml"]

volumes:
  ollama_data:
```

## Troubleshooting

### Common Issues

1. **NATS Connection Failed**
   - Ensure NATS server is running
   - Check server URL in configuration
   - Verify network connectivity

2. **Model Provider Error**
   - Ensure Ollama is running
   - Check model is downloaded (`ollama list`)
   - Verify base URL in configuration

3. **High Memory Usage**
   - Adjust `max_history` in dialog configuration
   - Reduce `context_window` size
   - Enable log rotation

### Debug Mode

Enable debug logging:
```bash
cargo run -- --log-level debug
```

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests
5. Submit a pull request

## License

MIT License - see LICENSE file for details 