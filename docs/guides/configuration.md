# Configuration Guide

clawft uses a single JSON file for all configuration. Every field is optional
and falls back to a sensible default when omitted.

## Config File Location

The runtime resolves its configuration through a three-step discovery chain,
stopping at the first match:

| Priority | Source | Notes |
|----------|--------|-------|
| 1 | `CLAWFT_CONFIG` environment variable | Absolute path to a JSON file. |
| 2 | `~/.clawft/config.json` | Recommended location. |
| 3 | `~/.nanobot/config.json` | Legacy fallback for migration. |

If no file is found at any location, all values fall back to their compiled-in
defaults (equivalent to an empty `{}` document).

You can inspect the fully resolved configuration at any time:

```sh
weft config show              # full config as JSON
weft config section agents    # single section
weft config section gateway
```

## Config File Format

The configuration file accepts both `snake_case` and `camelCase` keys. Keys
are normalized to `snake_case` internally. Unknown fields are silently ignored
for forward compatibility.

Below is a fully annotated example showing every section. Remove or omit any
section you do not need.

```json
{
  "agents": {
    "defaults": {
      "model": "anthropic/claude-opus-4-5",
      "workspace": "~/.clawft/workspace",
      "max_tokens": 8192,
      "temperature": 0.7,
      "max_tool_iterations": 20,
      "memory_window": 50
    }
  },

  "providers": {
    "anthropic": {
      "api_key": "sk-ant-...",
      "api_base": null,
      "extra_headers": {}
    },
    "openai": {
      "api_key": "sk-..."
    },
    "openrouter": {
      "api_key": "sk-or-...",
      "api_base": "https://openrouter.ai/api/v1"
    }
  },

  "channels": {
    "telegram": {
      "enabled": true,
      "token": "123456:ABC-DEF...",
      "allow_from": ["user1"],
      "proxy": null
    },
    "slack": {
      "enabled": true,
      "mode": "socket",
      "bot_token": "xoxb-...",
      "app_token": "xapp-...",
      "webhook_path": "/slack/events",
      "user_token_read_only": true,
      "group_policy": "mention",
      "group_allow_from": [],
      "dm": {
        "enabled": true,
        "policy": "open",
        "allow_from": []
      }
    },
    "discord": {
      "enabled": true,
      "token": "MTIz...",
      "allow_from": [],
      "gateway_url": "wss://gateway.discord.gg/?v=10&encoding=json",
      "intents": 37377
    }
  },

  "gateway": {
    "host": "0.0.0.0",
    "port": 18790,
    "heartbeat_interval_minutes": 0,
    "heartbeat_prompt": "heartbeat"
  },

  "tools": {
    "web": {
      "search": {
        "api_key": "",
        "max_results": 5
      }
    },
    "exec": {
      "timeout": 60
    },
    "restrict_to_workspace": false,
    "mcp_servers": {
      "example-server": {
        "command": "npx",
        "args": ["-y", "@example/mcp-server"],
        "env": {
          "API_KEY": "secret"
        }
      }
    }
  }
}
```

## Section Reference

### agents.defaults

Default settings applied to every agent instance.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `model` | string | `"anthropic/claude-opus-4-5"` | LLM model identifier using `provider/model` syntax. |
| `workspace` | string | `"~/.nanobot/workspace"` | Working directory for file tool operations. Tilde is expanded at runtime. |
| `max_tokens` | integer | `8192` | Maximum tokens in a single LLM response. |
| `temperature` | float | `0.7` | Sampling temperature for LLM calls. |
| `max_tool_iterations` | integer | `20` | Maximum tool-use rounds per message turn. |
| `memory_window` | integer | `50` | Number of recent messages included in context. |

### providers

Credentials and endpoint overrides for LLM providers. Each provider section
has the same structure. Named providers: `anthropic`, `openai`, `openrouter`,
`deepseek`, `groq`, `zhipu`, `dashscope`, `vllm`, `gemini`, `moonshot`,
`minimax`, `aihubmix`, `openai_codex`, `custom`.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `api_key` | string | `""` | API key for authentication. Prefer environment variables over inline keys. |
| `api_base` | string or null | `null` | Base URL override (e.g., for proxies or self-hosted endpoints). |
| `extra_headers` | object | `{}` | Additional HTTP headers sent with every request to this provider. |

The `model` field in `agents.defaults` uses a `provider/model` prefix to route
requests. For example, `"anthropic/claude-opus-4-5"` routes to the `anthropic`
provider and strips the prefix before calling the API.

Built-in provider routing:

| Provider | Prefix | API Key Env Var | Base URL |
|----------|--------|-----------------|----------|
| OpenAI | `openai/` | `OPENAI_API_KEY` | `https://api.openai.com/v1` |
| Anthropic | `anthropic/` | `ANTHROPIC_API_KEY` | `https://api.anthropic.com/v1` |
| Groq | `groq/` | `GROQ_API_KEY` | `https://api.groq.com/openai/v1` |
| DeepSeek | `deepseek/` | `DEEPSEEK_API_KEY` | `https://api.deepseek.com/v1` |
| Mistral | `mistral/` | `MISTRAL_API_KEY` | `https://api.mistral.ai/v1` |
| Together | `together/` | `TOGETHER_API_KEY` | `https://api.together.xyz/v1` |
| OpenRouter | `openrouter/` | `OPENROUTER_API_KEY` | `https://openrouter.ai/api/v1` |

### gateway

HTTP server and heartbeat settings.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `host` | string | `"0.0.0.0"` | Bind address for the HTTP server. |
| `port` | integer | `18790` | Listen port. |
| `heartbeat_interval_minutes` | integer | `0` | Minutes between heartbeat messages. `0` disables heartbeats. |
| `heartbeat_prompt` | string | `"heartbeat"` | Prompt text sent on each heartbeat tick. |

### channels

Each channel section follows its own schema. All channels share an `enabled`
boolean that defaults to `false`.

### tools

Top-level tool configuration.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `restrict_to_workspace` | boolean | `false` | When `true`, all file tools are sandboxed to the workspace directory. |

#### tools.web.search

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `api_key` | string | `""` | Search provider API key (e.g., Brave Search). |
| `max_results` | integer | `5` | Maximum number of search results returned. |

#### tools.exec

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `timeout` | integer | `60` | Command execution timeout in seconds. |

## Channel Setup

### Telegram

1. Create a bot via [@BotFather](https://t.me/BotFather) on Telegram.
2. Copy the bot token.
3. Add the following to your config file:

```json
{
  "channels": {
    "telegram": {
      "enabled": true,
      "token": "123456789:ABCdef..."
    }
  }
}
```

**Optional fields:**

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enabled` | boolean | `false` | Enable or disable the Telegram channel. |
| `token` | string | `""` | Bot API token from BotFather. |
| `allow_from` | string[] | `[]` | Restrict to these user IDs or usernames. Empty allows all users. |
| `proxy` | string or null | `null` | HTTP or SOCKS5 proxy URL for regions where Telegram is restricted. |

### Slack

Slack uses Socket Mode, which requires both a Bot Token and an App-Level Token.

1. Create a Slack app at [api.slack.com/apps](https://api.slack.com/apps).
2. Enable **Socket Mode** under Settings and generate an App-Level Token
   (`xapp-...`) with the `connections:write` scope.
3. Under **OAuth & Permissions**, add the required Bot Token Scopes
   (`chat:write`, `channels:history`, `groups:history`, `im:history`,
   `mpim:history`, `app_mentions:read`).
4. Install the app to your workspace and copy the Bot User OAuth Token
   (`xoxb-...`).
5. Under **Event Subscriptions**, subscribe to `message.channels`,
   `message.groups`, `message.im`, `message.mpim`, and `app_mention`.

```json
{
  "channels": {
    "slack": {
      "enabled": true,
      "bot_token": "xoxb-...",
      "app_token": "xapp-..."
    }
  }
}
```

**All fields:**

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enabled` | boolean | `false` | Enable or disable the Slack channel. |
| `mode` | string | `"socket"` | Connection mode. Only `"socket"` is currently supported. |
| `bot_token` | string | `""` | Bot User OAuth Token (`xoxb-...`). |
| `app_token` | string | `""` | App-Level Token (`xapp-...`) for Socket Mode. |
| `webhook_path` | string | `"/slack/events"` | Webhook path for event subscriptions (future use). |
| `user_token_read_only` | boolean | `true` | Whether the user token is treated as read-only. |
| `group_policy` | string | `"mention"` | Group message policy: `"mention"` (respond when @-mentioned), `"open"` (respond to all), or `"allowlist"` (respond only in listed channels). |
| `group_allow_from` | string[] | `[]` | Channel IDs permitted when `group_policy` is `"allowlist"`. |

**DM sub-section (`channels.slack.dm`):**

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enabled` | boolean | `true` | Whether direct messages are accepted. |
| `policy` | string | `"open"` | DM access policy: `"open"` or `"allowlist"`. |
| `allow_from` | string[] | `[]` | Slack user IDs permitted when policy is `"allowlist"`. |

### Discord

1. Create an application at the
   [Discord Developer Portal](https://discord.com/developers/applications).
2. Under **Bot**, create a bot and copy its token.
3. Enable the **Message Content** privileged intent.
4. Generate an invite URL under **OAuth2 > URL Generator** with the `bot`
   scope and `Send Messages` + `Read Message History` permissions.
5. Invite the bot to your server.

```json
{
  "channels": {
    "discord": {
      "enabled": true,
      "token": "MTIz..."
    }
  }
}
```

**All fields:**

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enabled` | boolean | `false` | Enable or disable the Discord channel. |
| `token` | string | `""` | Bot token from the Developer Portal. |
| `allow_from` | string[] | `[]` | Restrict to these user IDs. Empty allows all users. |
| `gateway_url` | string | `"wss://gateway.discord.gg/?v=10&encoding=json"` | WebSocket gateway URL. Override only for testing. |
| `intents` | integer | `37377` | Gateway intents bitmask. Default enables GUILDS, GUILD_MESSAGES, DIRECT_MESSAGES, and MESSAGE_CONTENT. |

### Additional Channels

clawft also supports WhatsApp (via WebSocket bridge), Feishu/Lark, DingTalk,
Mochat, Email (IMAP + SMTP), and QQ. Each follows the same pattern of
`enabled` plus channel-specific credentials. Refer to the source type
definitions in `clawft-types` for the full field list.

Unknown channel names are captured as extension data and silently ignored,
allowing forward compatibility with future channel plugins.

## MCP Server Configuration

MCP (Model Context Protocol) servers extend the tool system with external
capabilities. Servers are defined as named entries under `tools.mcp_servers`.

Each entry supports two transport modes:

**Stdio transport** (most common) -- the runtime spawns the server as a child
process and communicates over stdin/stdout:

```json
{
  "tools": {
    "mcp_servers": {
      "filesystem": {
        "command": "npx",
        "args": ["-y", "@modelcontextprotocol/server-filesystem", "/path/to/dir"],
        "env": {}
      }
    }
  }
}
```

**HTTP transport** -- the runtime connects to an already-running server over
Streamable HTTP:

```json
{
  "tools": {
    "mcp_servers": {
      "remote-tools": {
        "url": "http://localhost:8080/mcp"
      }
    }
  }
}
```

**MCP server fields:**

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `command` | string | `""` | Executable to run (stdio transport). |
| `args` | string[] | `[]` | Arguments passed to the command. |
| `env` | object | `{}` | Environment variables set for the child process. |
| `url` | string | `""` | Streamable HTTP endpoint URL (HTTP transport). |

The server name (the JSON key) is used for tool namespacing. A server named
`"filesystem"` exposes tools prefixed with `filesystem__`.

## Environment Variables

| Variable | Description |
|----------|-------------|
| `CLAWFT_CONFIG` | Override the config file path. Takes priority over all file-based discovery. |
| `RUST_LOG` | Controls log verbosity via `tracing`'s `EnvFilter` syntax. Examples: `info`, `debug`, `clawft_core=trace`, `clawft_llm=debug,info`. |
| `OPENAI_API_KEY` | API key for OpenAI models. |
| `ANTHROPIC_API_KEY` | API key for Anthropic models. |
| `GROQ_API_KEY` | API key for Groq models. |
| `DEEPSEEK_API_KEY` | API key for DeepSeek models. |
| `MISTRAL_API_KEY` | API key for Mistral models. |
| `TOGETHER_API_KEY` | API key for Together AI models. |
| `OPENROUTER_API_KEY` | API key for OpenRouter gateway. |

Provider API keys can be set either in the config file (`providers.<name>.api_key`)
or as environment variables. Environment variables are generally preferred to
avoid storing secrets in files.

## Security Policy

clawft includes configurable security policies for command execution and URL
access. These policies protect against prompt injection attacks where malicious
instructions in user content could trick the agent into executing dangerous
operations.

### Command Execution Policy

The `tools.commandPolicy` section controls which commands the `exec_shell` and
`spawn` tools can execute.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `mode` | string | `"allowlist"` | `"allowlist"` or `"denylist"` |
| `allowlist` | string[] | `[]` | Permitted commands (overrides defaults) |
| `denylist` | string[] | `[]` | Blocked patterns (overrides defaults) |

**Default allowlist** (used when `allowlist` is empty):
`echo`, `cat`, `ls`, `pwd`, `head`, `tail`, `wc`, `grep`, `find`, `sort`,
`uniq`, `diff`, `date`, `env`, `true`, `false`, `test`

Example -- expand the allowlist:
```json
{
  "tools": {
    "commandPolicy": {
      "mode": "allowlist",
      "allowlist": ["echo", "cat", "ls", "pwd", "python3", "node", "cargo"]
    }
  }
}
```

Example -- use denylist mode (less secure, more permissive):
```json
{
  "tools": {
    "commandPolicy": {
      "mode": "denylist",
      "denylist": ["rm -rf /", "sudo ", "mkfs", "dd if="]
    }
  }
}
```

### URL Safety Policy (SSRF Protection)

The `tools.urlPolicy` section controls which URLs the `web_fetch` tool can
access. By default, requests to private networks, loopback addresses, and
cloud metadata endpoints are blocked.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enabled` | bool | `true` | Enable URL safety validation |
| `allowPrivate` | bool | `false` | Allow private/internal IPs |
| `allowedDomains` | string[] | `[]` | Domains that bypass checks |
| `blockedDomains` | string[] | `[]` | Additional blocked domains |

Blocked by default:
- Private networks: `10.0.0.0/8`, `172.16.0.0/12`, `192.168.0.0/16`
- Loopback: `127.0.0.0/8`, `::1`
- Link-local: `169.254.0.0/16`, `fe80::/10`
- Cloud metadata: `169.254.169.254`, `metadata.google.internal`

Example -- allow specific internal services:
```json
{
  "tools": {
    "urlPolicy": {
      "enabled": true,
      "allowedDomains": ["api.internal.corp", "vault.internal.corp"]
    }
  }
}
```

## Feature Flags

Compile-time feature flags enable optional capabilities that are not included
in the default build.

### vector-memory

Enables the vector memory subsystem, which provides `IntelligentRouter`,
`VectorStore`, and `SessionIndexer` for semantic search over session history.

```sh
cargo install clawft-cli --features vector-memory
```

Or when building from source:

```sh
cargo build --release --features vector-memory
```

This flag is propagated from `clawft-cli` to `clawft-core`. It is off by
default to keep the baseline binary small and avoid unnecessary dependencies.
