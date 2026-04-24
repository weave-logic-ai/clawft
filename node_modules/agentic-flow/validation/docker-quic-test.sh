#!/bin/bash
#
# Docker-based QUIC transport validation
# Tests QUIC integration in a clean containerized environment
#

set -e

echo "🐳 QUIC Transport Docker Validation"
echo "===================================="
echo ""

# Build Docker image
echo "📦 Building Docker test image..."
docker build -f Dockerfile.quic-test -t agentic-flow-quic-test . || {
    echo "❌ Docker build failed"
    exit 1
}

echo ""
echo "✅ Docker image built successfully"
echo ""

# Run tests in container
echo "🧪 Running QUIC tests in Docker container..."
echo ""

docker run --rm agentic-flow-quic-test || {
    echo ""
    echo "❌ Docker tests failed"
    exit 1
}

echo ""
echo "✅ All Docker-based QUIC tests passed!"
echo ""
echo "📊 Validation Summary:"
echo "   ✓ WASM bindings load in containerized environment"
echo "   ✓ Package exports are correctly configured"
echo "   ✓ Dependencies resolve properly"
echo "   ✓ Integration works in production-like setup"
echo ""
