#!/bin/bash
# Build script for Claude Agent SDK Docker container
# This ensures the correct build context is used

set -e

# Get the script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"

echo "Building Claude Agent SDK Docker image..."
echo "Root directory: $ROOT_DIR"
echo "Docker context: $ROOT_DIR"
echo "Dockerfile: docker/claude-agent-sdk/Dockerfile"

cd "$ROOT_DIR"

docker build \
  -f docker/claude-agent-sdk/Dockerfile \
  -t claude-agents:latest \
  -t claude-agents:cli \
  .

echo ""
echo "✅ Build complete!"
echo ""
echo "Usage examples:"
echo "  docker run claude-agents:cli --list"
echo "  docker run --env-file .env claude-agents:cli --agent goal-planner --task \"Plan feature\""
echo "  docker-compose -f docker/claude-agent-sdk/docker-compose.agent.yml --profile goal-planner up"
