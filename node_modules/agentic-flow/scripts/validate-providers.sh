#!/bin/bash
# Comprehensive provider validation script

set -e

echo "🧪 Agentic Flow - Provider Validation Suite"
echo "==========================================="
echo ""

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Test 1: Anthropic Provider
if [ -n "$ANTHROPIC_API_KEY" ]; then
    echo -e "${BLUE}1️⃣ Testing Anthropic Provider...${NC}"
    npx agentic-flow@latest --agent coder --task "Write hello world" --provider anthropic 2>&1 | grep -q "Completed" && \
        echo -e "${GREEN}✅ Anthropic provider works${NC}" || \
        echo "❌ Anthropic provider failed"
else
    echo "⏭️  Skipping Anthropic (no API key)"
fi

echo ""

# Test 2: OpenRouter Provider - Meta Llama
if [ -n "$OPENROUTER_API_KEY" ]; then
    echo -e "${BLUE}2️⃣ Testing OpenRouter - Meta Llama 3.1...${NC}"
    npx agentic-flow@latest --agent coder --task "Write hello world" --model "meta-llama/llama-3.1-8b-instruct" 2>&1 | grep -q "Completed" && \
        echo -e "${GREEN}✅ OpenRouter (Meta Llama) works${NC}" || \
        echo "❌ OpenRouter (Meta Llama) failed"

    echo ""
    echo -e "${BLUE}3️⃣ Testing OpenRouter - DeepSeek...${NC}"
    npx agentic-flow@latest --agent coder --task "Write hello world" --model "deepseek/deepseek-chat" 2>&1 | grep -q "Completed" && \
        echo -e "${GREEN}✅ OpenRouter (DeepSeek) works${NC}" || \
        echo "❌ OpenRouter (DeepSeek) failed"

    echo ""
    echo -e "${BLUE}4️⃣ Testing OpenRouter - Gemini Free...${NC}"
    npx agentic-flow@latest --agent coder --task "Write hello world" --model "google/gemini-2.0-flash-exp:free" 2>&1 | grep -q "Completed" && \
        echo -e "${GREEN}✅ OpenRouter (Gemini) works${NC}" || \
        echo "❌ OpenRouter (Gemini) failed"
else
    echo "⏭️  Skipping OpenRouter tests (no API key)"
fi

echo ""
echo -e "${GREEN}✅ Provider validation complete!${NC}"
