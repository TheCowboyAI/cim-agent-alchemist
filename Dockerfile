# Build stage
FROM rust:1.75 as builder

WORKDIR /app

# Copy workspace files first
COPY Cargo.toml /workspace/Cargo.toml
COPY Cargo.lock /workspace/Cargo.lock

# Copy all domain dependencies
COPY cim-domain-agent /workspace/cim-domain-agent
COPY cim-domain-dialog /workspace/cim-domain-dialog
COPY cim-domain-identity /workspace/cim-domain-identity
COPY cim-domain-graph /workspace/cim-domain-graph
COPY cim-domain-conceptualspaces /workspace/cim-domain-conceptualspaces
COPY cim-domain-workflow /workspace/cim-domain-workflow
COPY cim-infrastructure /workspace/cim-infrastructure
COPY cim-bridge /workspace/cim-bridge

# Copy the agent source
COPY cim-agent-alchemist /workspace/cim-agent-alchemist

# Set working directory to agent
WORKDIR /workspace/cim-agent-alchemist

# Build the agent
RUN cargo build --release --bin alchemist

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Copy the binary from builder
COPY --from=builder /workspace/cim-agent-alchemist/target/release/alchemist /usr/local/bin/alchemist

# Create non-root user
RUN useradd -m -u 1000 alchemist

# Switch to non-root user
USER alchemist

# Set working directory
WORKDIR /home/alchemist

# Expose health check port (if needed)
EXPOSE 8080

# Set default environment variables
ENV RUST_LOG=info
ENV RUST_BACKTRACE=1

# Run the agent
ENTRYPOINT ["alchemist"]

# Default command shows help
CMD ["--help"] 