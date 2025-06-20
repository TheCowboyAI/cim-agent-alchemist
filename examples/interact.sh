#!/bin/bash
# Example interactions with the Alchemist agent

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}Alchemist Agent Interaction Examples${NC}"
echo "====================================="

# Check health
echo -e "\n${GREEN}1. Health Check:${NC}"
nats request cim.agent.alchemist.health ""

# List available concepts
echo -e "\n${GREEN}2. List CIM Concepts:${NC}"
nats request cim.agent.alchemist.queries.list_concepts '{
  "id": "query-1",
  "query_type": "list_concepts",
  "parameters": {},
  "timestamp": "'$(date -u +%Y-%m-%dT%H:%M:%SZ)'",
  "origin": "demo"
}'

# Start a dialog
echo -e "\n${GREEN}3. Start Dialog:${NC}"
nats publish cim.agent.alchemist.commands.start_dialog '{
  "id": "cmd-1",
  "command_type": "start_dialog",
  "payload": {
    "user_id": "demo-user",
    "context": {},
    "metadata": {}
  },
  "timestamp": "'$(date -u +%Y-%m-%dT%H:%M:%SZ)'",
  "origin": "demo"
}'

# Explain a concept
echo -e "\n${GREEN}4. Explain Event Sourcing:${NC}"
nats publish cim.agent.alchemist.commands.explain_concept '{
  "id": "cmd-2",
  "command_type": "explain_concept",
  "payload": {
    "concept": "Event Sourcing"
  },
  "timestamp": "'$(date -u +%Y-%m-%dT%H:%M:%SZ)'",
  "origin": "demo"
}'

# Visualize architecture
echo -e "\n${GREEN}5. Visualize CIM Architecture:${NC}"
nats publish cim.agent.alchemist.commands.visualize_architecture '{
  "id": "cmd-3",
  "command_type": "visualize_architecture",
  "payload": {
    "scope": "domains"
  },
  "timestamp": "'$(date -u +%Y-%m-%dT%H:%M:%SZ)'",
  "origin": "demo"
}'

# Guide through workflow
echo -e "\n${GREEN}6. Guide Through Creating an Agent:${NC}"
nats publish cim.agent.alchemist.commands.guide_workflow '{
  "id": "cmd-4",
  "command_type": "guide_workflow",
  "payload": {
    "workflow_type": "create_agent"
  },
  "timestamp": "'$(date -u +%Y-%m-%dT%H:%M:%SZ)'",
  "origin": "demo"
}'

# Find similar concepts
echo -e "\n${GREEN}7. Find Concepts Similar to Domain-Driven Design:${NC}"
nats request cim.agent.alchemist.queries.find_similar '{
  "id": "query-2",
  "query_type": "find_similar",
  "parameters": {
    "concept": "Domain-Driven Design"
  },
  "timestamp": "'$(date -u +%Y-%m-%dT%H:%M:%SZ)'",
  "origin": "demo"
}'

# Interactive dialog example
echo -e "\n${GREEN}8. Interactive Dialog (requires dialog_id from step 3):${NC}"
echo "To send a dialog message, use:"
echo 'nats publish cim.dialog.alchemist.{dialog_id} '\''{
  "dialog_id": "{dialog_id}",
  "content": "What is the difference between Event Sourcing and CQRS?",
  "sender": "demo-user",
  "metadata": {},
  "timestamp": "'$(date -u +%Y-%m-%dT%H:%M:%SZ)'"
}'\'''

echo -e "\n${BLUE}Subscribe to events to see responses:${NC}"
echo "nats subscribe 'cim.agent.alchemist.events.>'" 