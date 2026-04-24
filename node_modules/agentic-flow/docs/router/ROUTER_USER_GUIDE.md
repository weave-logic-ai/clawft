# Multi-Model Router User Guide

## Introduction

The Multi-Model Router enables agentic-flow to work with multiple LLM providers beyond Anthropic's Claude, including OpenAI, OpenRouter, local models via Ollama, and any provider supported by LiteLLM.

## Quick Start

### 1. Installation

The router is included with agentic-flow v2.0+:

```bash
npm install -g agentic-flow@latest
```

### 2. Basic Configuration

Create a router configuration file:

```bash
# Initialize with default config
npx agentic-flow router init

# Or create custom config
npx agentic-flow router init --provider openai
```

This creates `~/.agentic-flow/router.config.json`:

```json
{
  "version": "1.0",
  "defaultProvider": "anthropic",
  "fallbackChain": ["anthropic", "openai"],
  "providers": {
    "anthropic": {
      "apiKey": "${ANTHROPIC_API_KEY}",
      "models": {
        "default": "claude-3-5-sonnet-20241022"
      }
    },
    "openai": {
      "apiKey": "${OPENAI_API_KEY}",
      "models": {
        "default": "gpt-4-turbo-preview"
      }
    }
  }
}
```

### 3. Set Environment Variables

Add API keys to your environment:

```bash
# Add to ~/.bashrc or ~/.zshrc
export ANTHROPIC_API_KEY="sk-ant-..."
export OPENAI_API_KEY="sk-..."
export OPENROUTER_API_KEY="sk-or-..."
```

### 4. Use the Router

```bash
# Use default provider (Anthropic)
npx agentic-flow --agent coder --task "Build a REST API"

# Use specific provider
npx agentic-flow --provider openai --agent coder --task "Build a REST API"

# Use specific model
npx agentic-flow --model gpt-4 --agent coder --task "Build a REST API"

# Use local model via Ollama
npx agentic-flow --provider ollama --model llama3:70b --agent coder --task "Build a REST API"
```

## Provider Setup Guides

### Anthropic (Claude)

**Prerequisites:**
- Anthropic API key from https://console.anthropic.com/

**Configuration:**

```json
{
  "providers": {
    "anthropic": {
      "apiKey": "${ANTHROPIC_API_KEY}",
      "baseUrl": "https://api.anthropic.com",
      "models": {
        "default": "claude-3-5-sonnet-20241022",
        "fast": "claude-3-5-haiku-20241022",
        "advanced": "claude-3-opus-20240229"
      },
      "timeout": 120000,
      "maxRetries": 3
    }
  }
}
```

**Usage:**

```bash
# Default Claude model
npx agentic-flow --task "Analyze code"

# Specific Claude model
npx agentic-flow --model claude-3-opus-20240229 --task "Complex reasoning task"

# Fast Claude model
npx agentic-flow --model claude-3-5-haiku-20241022 --task "Quick task"
```

### OpenAI

**Prerequisites:**
- OpenAI API key from https://platform.openai.com/api-keys
- Optional: Organization ID

**Configuration:**

```json
{
  "providers": {
    "openai": {
      "apiKey": "${OPENAI_API_KEY}",
      "organization": "${OPENAI_ORG_ID}",
      "models": {
        "default": "gpt-4-turbo-preview",
        "fast": "gpt-3.5-turbo",
        "advanced": "gpt-4"
      }
    }
  }
}
```

**Usage:**

```bash
# Use OpenAI
npx agentic-flow --provider openai --task "Write code"

# Specific GPT model
npx agentic-flow --model gpt-4 --task "Complex reasoning"

# Cheaper model for simple tasks
npx agentic-flow --model gpt-3.5-turbo --task "Simple question"
```

**Cost Optimization:**

```bash
# Enable cost tracking
npx agentic-flow router config set routing.costOptimization.enabled true

# Set budget alerts
npx agentic-flow router config set routing.costOptimization.budgetAlerts.daily 10.00
```

### OpenRouter

**Prerequisites:**
- OpenRouter API key from https://openrouter.ai/keys
- Credits or payment method configured

**Configuration:**

```json
{
  "providers": {
    "openrouter": {
      "apiKey": "${OPENROUTER_API_KEY}",
      "baseUrl": "https://openrouter.ai/api/v1",
      "models": {
        "default": "anthropic/claude-3.5-sonnet",
        "fast": "anthropic/claude-3-haiku",
        "advanced": "anthropic/claude-3-opus"
      },
      "preferences": {
        "requireParameters": true,
        "dataCollection": "deny",
        "order": ["anthropic", "openai", "google"]
      }
    }
  }
}
```

**Available Models via OpenRouter:**

```bash
# List available models
npx agentic-flow router models --provider openrouter

# Use specific model
npx agentic-flow --provider openrouter --model anthropic/claude-3.5-sonnet --task "..."

# Try different providers through OpenRouter
npx agentic-flow --provider openrouter --model google/gemini-pro --task "..."
npx agentic-flow --provider openrouter --model meta-llama/llama-3-70b-instruct --task "..."
```

**Cost Optimization with OpenRouter:**

OpenRouter automatically routes to cheapest available provider when model supports it:

```json
{
  "routing": {
    "mode": "cost-optimized",
    "rules": [
      {
        "condition": {
          "complexity": "low"
        },
        "action": {
          "provider": "openrouter",
          "model": "anthropic/claude-3-haiku"
        }
      }
    ]
  }
}
```

### Ollama (Local Models)

**Prerequisites:**
- Ollama installed: https://ollama.ai/download
- Models downloaded locally

**Installation:**

```bash
# Install Ollama
curl -fsSL https://ollama.ai/install.sh | sh

# Download models
ollama pull llama3:8b
ollama pull llama3:70b
ollama pull phi3:mini
ollama pull codellama:13b

# Verify models
ollama list
```

**Configuration:**

```json
{
  "providers": {
    "ollama": {
      "baseUrl": "http://localhost:11434",
      "models": {
        "default": "llama3:8b",
        "fast": "phi3:mini",
        "advanced": "llama3:70b",
        "code": "codellama:13b"
      },
      "gpuLayers": 35,
      "contextWindow": 8192,
      "numPredict": 2048
    }
  }
}
```

**Usage:**

```bash
# Use local model
npx agentic-flow --provider ollama --task "Write code"

# Specific local model
npx agentic-flow --provider ollama --model codellama:13b --task "Write Python code"

# Privacy-sensitive tasks
npx agentic-flow --provider ollama --model llama3:70b --task "Analyze confidential data"
```

**Performance Tuning:**

```bash
# Increase GPU layers for faster inference
npx agentic-flow router config set providers.ollama.gpuLayers 40

# Increase context window
npx agentic-flow router config set providers.ollama.contextWindow 16384

# Configure concurrent requests
npx agentic-flow router config set providers.ollama.concurrent 2
```

### LiteLLM (Universal Gateway)

**Prerequisites:**
- LiteLLM proxy running or configured
- API keys for target providers

**Configuration:**

```json
{
  "providers": {
    "litellm": {
      "enabled": true,
      "proxyUrl": "http://localhost:8000",
      "fallbackModels": [
        "gpt-4-turbo-preview",
        "claude-3-opus-20240229",
        "command-r-plus",
        "gemini-pro"
      ],
      "timeout": 180000,
      "loadBalancing": true
    }
  }
}
```

**Setup LiteLLM Proxy:**

```bash
# Install LiteLLM
pip install litellm[proxy]

# Create config
cat > litellm_config.yaml << EOF
model_list:
  - model_name: gpt-4
    litellm_params:
      model: gpt-4
      api_key: ${OPENAI_API_KEY}
  - model_name: claude-3-opus
    litellm_params:
      model: claude-3-opus-20240229
      api_key: ${ANTHROPIC_API_KEY}
  - model_name: gemini-pro
    litellm_params:
      model: gemini/gemini-pro
      api_key: ${GOOGLE_API_KEY}
EOF

# Start proxy
litellm --config litellm_config.yaml --port 8000
```

**Usage:**

```bash
# Use LiteLLM with automatic routing
npx agentic-flow --provider litellm --task "Write code"

# LiteLLM with specific model
npx agentic-flow --provider litellm --model gpt-4 --task "Complex task"

# Enable load balancing across providers
npx agentic-flow router config set providers.litellm.loadBalancing true
```

## Routing Strategies

### Manual Routing

Explicitly select provider and model:

```bash
npx agentic-flow --provider openai --model gpt-4 --task "..."
```

### Cost-Optimized Routing

Automatically select cheapest suitable provider:

```json
{
  "routing": {
    "mode": "cost-optimized",
    "costOptimization": {
      "enabled": true,
      "maxCostPerRequest": 0.50,
      "preferCheaper": true
    }
  }
}
```

```bash
# Enable cost optimization
npx agentic-flow --router-mode cost-optimized --task "..."

# View cost report
npx agentic-flow router costs --period today
```

### Performance-Optimized Routing

Prioritize fastest provider:

```json
{
  "routing": {
    "mode": "performance-optimized",
    "performance": {
      "targetLatency": 5000,
      "concurrentRequests": 10
    }
  }
}
```

### Rule-Based Routing

Define custom routing rules:

```json
{
  "routing": {
    "mode": "rule-based",
    "rules": [
      {
        "condition": {
          "agentType": ["coder", "reviewer"],
          "requiresTools": true
        },
        "action": {
          "provider": "anthropic",
          "model": "claude-3-5-sonnet-20241022"
        }
      },
      {
        "condition": {
          "agentType": ["researcher"],
          "complexity": "low"
        },
        "action": {
          "provider": "openai",
          "model": "gpt-3.5-turbo"
        }
      },
      {
        "condition": {
          "privacy": "high"
        },
        "action": {
          "provider": "ollama",
          "model": "llama3:70b"
        }
      }
    ]
  }
}
```

## Tool Calling Support

### Provider Compatibility

| Provider | Tool Calling | MCP Support | Format |
|----------|-------------|-------------|--------|
| Anthropic | ✅ Full | ✅ Native | Anthropic Tools |
| OpenAI | ✅ Full | ⚡ Translated | OpenAI Functions |
| OpenRouter | ✅ Varies | ⚡ Translated | Auto-detect |
| Ollama | ⚠️ Limited | ❌ Manual | Text-based |
| LiteLLM | ✅ Full | ⚡ Translated | Auto-detect |

### Enabling Tool Translation

```json
{
  "toolCalling": {
    "translationEnabled": true,
    "defaultFormat": "anthropic",
    "formatMapping": {
      "openai": "openai-functions",
      "anthropic": "anthropic-tools",
      "openrouter": "auto-detect",
      "ollama": "manual"
    },
    "fallbackStrategy": "disable-tools"
  }
}
```

### Using Tools with Different Providers

```bash
# Anthropic (native tool support)
npx agentic-flow --provider anthropic --agent coder --task "Use bash and file tools"

# OpenAI (translated tool support)
npx agentic-flow --provider openai --agent coder --task "Use bash and file tools"

# Ollama (text-based tools)
npx agentic-flow --provider ollama --agent coder --task "Use bash and file tools" --tool-mode text
```

## Cost Management

### Tracking Costs

```bash
# View today's costs
npx agentic-flow router costs --period today

# View monthly costs
npx agentic-flow router costs --period month

# View costs by provider
npx agentic-flow router costs --by-provider

# View costs by agent
npx agentic-flow router costs --by-agent
```

### Setting Budget Alerts

```bash
# Set daily budget
npx agentic-flow router config set routing.costOptimization.budgetAlerts.daily 10.00

# Set monthly budget
npx agentic-flow router config set routing.costOptimization.budgetAlerts.monthly 250.00

# Set per-request limit
npx agentic-flow router config set routing.costOptimization.maxCostPerRequest 0.50
```

### Cost Optimization Tips

1. **Use Cheaper Models for Simple Tasks**:
   ```bash
   npx agentic-flow --model gpt-3.5-turbo --agent researcher --task "Simple research"
   ```

2. **Enable Caching**:
   ```json
   {
     "cache": {
       "enabled": true,
       "ttl": 3600,
       "strategy": "lru"
     }
   }
   ```

3. **Use Local Models When Possible**:
   ```bash
   npx agentic-flow --provider ollama --task "Privacy-sensitive task"
   ```

4. **Configure Cost-Optimized Routing**:
   ```bash
   npx agentic-flow --router-mode cost-optimized --task "..."
   ```

## Performance Tuning

### Latency Optimization

```json
{
  "routing": {
    "performance": {
      "timeout": 60000,
      "concurrentRequests": 10,
      "circuitBreaker": {
        "enabled": true,
        "threshold": 5,
        "timeout": 60000
      }
    }
  }
}
```

### Concurrent Requests

```bash
# Increase concurrent requests
npx agentic-flow router config set routing.performance.concurrentRequests 20

# Enable connection pooling
npx agentic-flow router config set routing.performance.pooling true
```

### Caching Strategies

```json
{
  "cache": {
    "enabled": true,
    "ttl": 3600,
    "maxSize": 1000,
    "strategy": "lru",
    "providers": {
      "anthropic": { "ttl": 7200 },
      "openai": { "ttl": 3600 },
      "ollama": { "ttl": 1800 }
    }
  }
}
```

## Privacy & Security

### Local-Only Mode

Route all requests to local models:

```json
{
  "routing": {
    "mode": "rule-based",
    "rules": [
      {
        "condition": {
          "privacy": "high"
        },
        "action": {
          "provider": "ollama",
          "model": "llama3:70b"
        }
      }
    ]
  }
}
```

```bash
# Force local-only mode
npx agentic-flow --local-only --task "Confidential analysis"
```

### API Key Security

1. **Use Environment Variables**:
   ```bash
   export ANTHROPIC_API_KEY="sk-ant-..."
   export OPENAI_API_KEY="sk-..."
   ```

2. **Use Secret Management**:
   ```bash
   # AWS Secrets Manager
   export ANTHROPIC_API_KEY=$(aws secretsmanager get-secret-value --secret-id anthropic-key --query SecretString --output text)

   # HashiCorp Vault
   export OPENAI_API_KEY=$(vault kv get -field=api_key secret/openai)
   ```

3. **Rotate Keys Regularly**:
   ```bash
   npx agentic-flow router config update-key anthropic NEW_KEY
   ```

## Troubleshooting

### Common Issues

#### 1. Provider Connection Failed

```bash
# Test provider connectivity
npx agentic-flow router test anthropic
npx agentic-flow router test openai
npx agentic-flow router test ollama

# Check provider status
npx agentic-flow router status --provider anthropic
```

#### 2. Tool Calling Not Working

```bash
# Enable tool translation
npx agentic-flow router config set toolCalling.translationEnabled true

# Check tool support
npx agentic-flow router capabilities --provider openai
```

#### 3. High Latency

```bash
# Increase timeout
npx agentic-flow router config set routing.performance.timeout 180000

# Enable circuit breaker
npx agentic-flow router config set routing.performance.circuitBreaker.enabled true
```

#### 4. Cost Overruns

```bash
# Check current costs
npx agentic-flow router costs --period today

# Enable cost limits
npx agentic-flow router config set routing.costOptimization.enabled true
npx agentic-flow router config set routing.costOptimization.maxCostPerRequest 0.50
```

### Debug Mode

```bash
# Enable debug logging
npx agentic-flow --debug --task "..."

# View router logs
npx agentic-flow router logs --level debug

# Test configuration
npx agentic-flow router validate ~/.agentic-flow/router.config.json
```

## Best Practices

### 1. Development vs Production

**Development**:
```json
{
  "defaultProvider": "ollama",
  "fallbackChain": ["ollama", "anthropic"],
  "routing": {
    "mode": "manual"
  }
}
```

**Production**:
```json
{
  "defaultProvider": "anthropic",
  "fallbackChain": ["anthropic", "openai", "openrouter"],
  "routing": {
    "mode": "cost-optimized",
    "costOptimization": {
      "enabled": true,
      "budgetAlerts": { "daily": 50.00 }
    }
  }
}
```

### 2. Agent-Specific Routing

```json
{
  "routing": {
    "mode": "rule-based",
    "rules": [
      {
        "condition": { "agentType": ["coder", "reviewer"] },
        "action": { "provider": "anthropic", "model": "claude-3-5-sonnet-20241022" }
      },
      {
        "condition": { "agentType": ["researcher"] },
        "action": { "provider": "openai", "model": "gpt-4-turbo-preview" }
      },
      {
        "condition": { "agentType": ["tester"] },
        "action": { "provider": "openai", "model": "gpt-3.5-turbo" }
      }
    ]
  }
}
```

### 3. Fallback Strategy

Always configure fallback providers:

```json
{
  "fallbackChain": ["anthropic", "openai", "openrouter", "ollama"]
}
```

### 4. Monitor and Optimize

```bash
# Regular cost review
npx agentic-flow router costs --period month

# Performance analysis
npx agentic-flow router metrics --provider all

# Update routing rules based on metrics
npx agentic-flow router optimize
```

## Advanced Features

### Custom Providers

Add custom LLM providers:

```bash
# Register custom provider
npx agentic-flow router provider add custom-provider \
  --type openai-compatible \
  --base-url https://api.custom.com/v1 \
  --api-key ${CUSTOM_API_KEY}
```

### Parallel Provider Requests

Send requests to multiple providers simultaneously:

```bash
npx agentic-flow --providers "anthropic,openai,openrouter" \
  --strategy consensus \
  --task "Critical decision"
```

### Provider-Specific Features

Enable provider-specific features:

```json
{
  "providers": {
    "anthropic": {
      "features": {
        "prefillAssistant": true,
        "visionEnabled": true,
        "toolChoice": "auto"
      }
    },
    "openai": {
      "features": {
        "jsonMode": true,
        "seedEnabled": true,
        "functionCalling": true
      }
    }
  }
}
```

## Next Steps

1. **Configure Providers**: Set up API keys and models
2. **Test Configuration**: Validate with `router validate`
3. **Run Sample Tasks**: Test with different providers
4. **Monitor Costs**: Track spending and optimize
5. **Adjust Routing**: Refine rules based on usage patterns

## Additional Resources

- [Configuration Reference](./ROUTER_CONFIG_REFERENCE.md)
- [Implementation Plan](./MULTI_MODEL_ROUTER_PLAN.md)
- [Provider Setup Guides](./providers/)
- [Troubleshooting Guide](./TROUBLESHOOTING.md)
- [API Documentation](./API.md)
