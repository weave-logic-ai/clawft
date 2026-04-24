# Multi-Model Router Configuration Reference

## Overview

The router configuration file (`router.config.json`) controls how the agentic-flow system selects and routes requests to different LLM providers.

## Configuration File Location

- **Default**: `~/.agentic-flow/router.config.json`
- **Custom**: Use `--router-config <path>` CLI option
- **Environment**: Set `AGENTIC_FLOW_ROUTER_CONFIG` variable

## Complete Configuration Schema

```json
{
  "version": "1.0",
  "defaultProvider": "anthropic",
  "fallbackChain": ["anthropic", "openai", "ollama"],
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
      "maxRetries": 3,
      "retryDelay": 1000,
      "rateLimit": {
        "requestsPerMinute": 50,
        "tokensPerMinute": 100000
      }
    },
    "openai": {
      "apiKey": "${OPENAI_API_KEY}",
      "organization": "${OPENAI_ORG_ID}",
      "baseUrl": "https://api.openai.com/v1",
      "models": {
        "default": "gpt-4-turbo-preview",
        "fast": "gpt-3.5-turbo",
        "advanced": "gpt-4"
      },
      "timeout": 120000,
      "maxRetries": 3
    },
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
    },
    "ollama": {
      "baseUrl": "http://localhost:11434",
      "models": {
        "default": "llama3:8b",
        "fast": "phi3:mini",
        "advanced": "llama3:70b"
      },
      "gpuLayers": 35,
      "contextWindow": 8192,
      "numPredict": 2048
    },
    "litellm": {
      "enabled": true,
      "fallbackModels": [
        "gpt-4-turbo-preview",
        "claude-3-opus-20240229",
        "command-r-plus"
      ],
      "timeout": 180000
    }
  },
  "routing": {
    "mode": "cost-optimized",
    "rules": [
      {
        "condition": {
          "agentType": ["researcher", "planner"],
          "complexity": "low"
        },
        "action": {
          "provider": "openai",
          "model": "gpt-3.5-turbo"
        },
        "reason": "Simple tasks use cheaper models"
      },
      {
        "condition": {
          "agentType": ["coder", "reviewer"],
          "requiresTools": true
        },
        "action": {
          "provider": "anthropic",
          "model": "claude-3-5-sonnet-20241022"
        },
        "reason": "Tool calling tasks need Claude"
      },
      {
        "condition": {
          "privacy": "high",
          "localOnly": true
        },
        "action": {
          "provider": "ollama",
          "model": "llama3:70b"
        },
        "reason": "Privacy-sensitive tasks use local models"
      }
    ],
    "costOptimization": {
      "enabled": true,
      "maxCostPerRequest": 0.50,
      "budgetAlerts": {
        "daily": 10.00,
        "monthly": 250.00
      },
      "preferCheaper": true,
      "costThreshold": 0.1
    },
    "performance": {
      "timeout": 120000,
      "concurrentRequests": 5,
      "circuitBreaker": {
        "enabled": true,
        "threshold": 5,
        "timeout": 60000,
        "resetTimeout": 30000
      }
    }
  },
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
  },
  "monitoring": {
    "enabled": true,
    "logLevel": "info",
    "metrics": {
      "trackCost": true,
      "trackLatency": true,
      "trackTokens": true,
      "trackErrors": true
    },
    "alerts": {
      "costThreshold": 5.00,
      "errorRate": 0.1,
      "latencyThreshold": 30000
    }
  },
  "cache": {
    "enabled": true,
    "ttl": 3600,
    "maxSize": 1000,
    "strategy": "lru"
  }
}
```

## Configuration Sections

### 1. Global Settings

#### `version`
- **Type**: String
- **Required**: Yes
- **Description**: Configuration schema version
- **Example**: `"1.0"`

#### `defaultProvider`
- **Type**: String
- **Required**: Yes
- **Description**: Primary provider to use when no routing rules match
- **Options**: `"anthropic"`, `"openai"`, `"openrouter"`, `"ollama"`, `"litellm"`
- **Example**: `"anthropic"`

#### `fallbackChain`
- **Type**: Array<String>
- **Required**: No
- **Description**: Ordered list of providers to try if default fails
- **Example**: `["anthropic", "openai", "ollama"]`

### 2. Provider Configuration

Each provider has specific configuration options:

#### Anthropic Provider

```json
{
  "apiKey": "${ANTHROPIC_API_KEY}",
  "baseUrl": "https://api.anthropic.com",
  "models": {
    "default": "claude-3-5-sonnet-20241022",
    "fast": "claude-3-5-haiku-20241022",
    "advanced": "claude-3-opus-20240229"
  },
  "timeout": 120000,
  "maxRetries": 3,
  "retryDelay": 1000
}
```

**Options**:
- `apiKey`: API key (supports env var substitution)
- `baseUrl`: API endpoint URL
- `models.default`: Default model for this provider
- `models.fast`: Fast/cheap model variant
- `models.advanced`: Advanced/expensive model variant
- `timeout`: Request timeout in milliseconds
- `maxRetries`: Number of retry attempts
- `retryDelay`: Delay between retries in milliseconds

#### OpenAI Provider

```json
{
  "apiKey": "${OPENAI_API_KEY}",
  "organization": "${OPENAI_ORG_ID}",
  "baseUrl": "https://api.openai.com/v1",
  "models": {
    "default": "gpt-4-turbo-preview",
    "fast": "gpt-3.5-turbo",
    "advanced": "gpt-4"
  }
}
```

**Additional Options**:
- `organization`: OpenAI organization ID

#### OpenRouter Provider

```json
{
  "apiKey": "${OPENROUTER_API_KEY}",
  "baseUrl": "https://openrouter.ai/api/v1",
  "preferences": {
    "requireParameters": true,
    "dataCollection": "deny",
    "order": ["anthropic", "openai", "google"]
  }
}
```

**Additional Options**:
- `preferences.requireParameters`: Require model parameters in requests
- `preferences.dataCollection`: Data collection preference (`"allow"` or `"deny"`)
- `preferences.order`: Provider preference order

#### Ollama Provider

```json
{
  "baseUrl": "http://localhost:11434",
  "models": {
    "default": "llama3:8b",
    "fast": "phi3:mini",
    "advanced": "llama3:70b"
  },
  "gpuLayers": 35,
  "contextWindow": 8192,
  "numPredict": 2048
}
```

**Additional Options**:
- `gpuLayers`: Number of layers to offload to GPU
- `contextWindow`: Maximum context window size
- `numPredict`: Maximum tokens to predict

#### LiteLLM Provider

```json
{
  "enabled": true,
  "fallbackModels": [
    "gpt-4-turbo-preview",
    "claude-3-opus-20240229",
    "command-r-plus"
  ],
  "timeout": 180000
}
```

### 3. Routing Rules

#### Routing Modes

- **`manual`**: User explicitly selects provider/model via CLI
- **`cost-optimized`**: Automatically select cheapest suitable provider
- **`performance-optimized`**: Prioritize fastest provider
- **`quality-optimized`**: Prioritize most capable model
- **`rule-based`**: Use custom routing rules

#### Rule Syntax

```json
{
  "condition": {
    "agentType": ["coder", "reviewer"],
    "requiresTools": true,
    "complexity": "high",
    "privacy": "high",
    "localOnly": false
  },
  "action": {
    "provider": "anthropic",
    "model": "claude-3-5-sonnet-20241022",
    "temperature": 0.7,
    "maxTokens": 4000
  },
  "reason": "Description of why this rule exists"
}
```

**Condition Fields**:
- `agentType`: Array of agent types this rule applies to
- `requiresTools`: Boolean, whether task requires tool calling
- `complexity`: String, task complexity level (`"low"`, `"medium"`, `"high"`)
- `privacy`: String, privacy requirement level
- `localOnly`: Boolean, whether to restrict to local models

**Action Fields**:
- `provider`: Provider to use
- `model`: Specific model name
- `temperature`: Model temperature (0.0-1.0)
- `maxTokens`: Maximum tokens to generate

### 4. Cost Optimization

```json
{
  "costOptimization": {
    "enabled": true,
    "maxCostPerRequest": 0.50,
    "budgetAlerts": {
      "daily": 10.00,
      "monthly": 250.00
    },
    "preferCheaper": true,
    "costThreshold": 0.1
  }
}
```

**Options**:
- `enabled`: Enable cost optimization
- `maxCostPerRequest`: Maximum cost per request in USD
- `budgetAlerts.daily`: Daily budget alert threshold
- `budgetAlerts.monthly`: Monthly budget alert threshold
- `preferCheaper`: Prefer cheaper models when quality difference is minimal
- `costThreshold`: Cost difference threshold for preferCheaper (USD)

### 5. Tool Calling Configuration

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

**Options**:
- `translationEnabled`: Enable automatic tool format translation
- `defaultFormat`: Default tool format to use
- `formatMapping`: Map providers to tool formats
- `fallbackStrategy`: What to do when tools not supported (`"disable-tools"`, `"use-text"`, `"fail"`)

### 6. Monitoring & Logging

```json
{
  "monitoring": {
    "enabled": true,
    "logLevel": "info",
    "metrics": {
      "trackCost": true,
      "trackLatency": true,
      "trackTokens": true,
      "trackErrors": true
    },
    "alerts": {
      "costThreshold": 5.00,
      "errorRate": 0.1,
      "latencyThreshold": 30000
    }
  }
}
```

**Log Levels**: `"debug"`, `"info"`, `"warn"`, `"error"`

### 7. Caching

```json
{
  "cache": {
    "enabled": true,
    "ttl": 3600,
    "maxSize": 1000,
    "strategy": "lru"
  }
}
```

**Options**:
- `ttl`: Time to live in seconds
- `maxSize`: Maximum cache entries
- `strategy`: Cache eviction strategy (`"lru"`, `"fifo"`, `"lfu"`)

## Environment Variable Substitution

Use `${VAR_NAME}` syntax to reference environment variables:

```json
{
  "apiKey": "${ANTHROPIC_API_KEY}",
  "organization": "${OPENAI_ORG_ID}"
}
```

## CLI Override Options

Configuration can be overridden via CLI:

```bash
# Override provider
npx agentic-flow --provider openai --task "..."

# Override model
npx agentic-flow --model gpt-4 --task "..."

# Override routing mode
npx agentic-flow --router-mode cost-optimized --task "..."

# Use custom config file
npx agentic-flow --router-config ./custom-router.json --task "..."
```

## Example Configurations

### Development Configuration

```json
{
  "version": "1.0",
  "defaultProvider": "ollama",
  "fallbackChain": ["ollama", "anthropic"],
  "providers": {
    "ollama": {
      "baseUrl": "http://localhost:11434",
      "models": {
        "default": "llama3:8b"
      }
    },
    "anthropic": {
      "apiKey": "${ANTHROPIC_API_KEY}",
      "models": {
        "default": "claude-3-5-haiku-20241022"
      }
    }
  },
  "routing": {
    "mode": "manual"
  }
}
```

### Production Configuration

```json
{
  "version": "1.0",
  "defaultProvider": "anthropic",
  "fallbackChain": ["anthropic", "openai", "openrouter"],
  "routing": {
    "mode": "cost-optimized",
    "costOptimization": {
      "enabled": true,
      "maxCostPerRequest": 1.00,
      "budgetAlerts": {
        "daily": 50.00,
        "monthly": 1000.00
      }
    }
  },
  "monitoring": {
    "enabled": true,
    "logLevel": "warn"
  }
}
```

### Privacy-Focused Configuration

```json
{
  "version": "1.0",
  "defaultProvider": "ollama",
  "providers": {
    "ollama": {
      "baseUrl": "http://localhost:11434",
      "models": {
        "default": "llama3:70b",
        "fast": "phi3:mini"
      }
    }
  },
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

## Validation

Validate configuration file:

```bash
npx agentic-flow router validate ./router.config.json
```

## Migration

Migrate from old configuration format:

```bash
npx agentic-flow router migrate ./old-config.json ./new-config.json
```

## Best Practices

1. **Use Environment Variables**: Never hardcode API keys
2. **Set Fallback Chain**: Always configure fallbacks for reliability
3. **Enable Cost Tracking**: Monitor spending with budget alerts
4. **Use Routing Rules**: Optimize cost and performance with smart routing
5. **Enable Caching**: Reduce API calls and costs
6. **Configure Timeouts**: Set appropriate timeouts for your use case
7. **Test Configuration**: Validate before deploying to production
