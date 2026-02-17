# CLI Reference

The `weft` command-line interface provides tools for managing agents, gateways,
channels, scheduled jobs, sessions, memory, and configuration.

## Global Flags

These flags are available on all subcommands.

| Flag | Description |
|------|-------------|
| `--verbose`, `-v` | Enable debug-level logging |
| `--version` | Show version and exit |
| `--help`, `-h` | Show help text and exit |

---

## weft agent

Start an interactive agent REPL, or send a single message and exit.

### Usage

```
weft agent [FLAGS] [OPTIONS]
```

### Options

| Flag / Option | Description |
|---------------|-------------|
| `--message`, `-m` `<MSG>` | Send a single message to the agent and exit. When omitted, the agent starts in interactive REPL mode. |
| `--model` `<MODEL>` | Override the model specified in config (e.g., `openai/gpt-4o`, `anthropic/claude-sonnet-4-20250514`). |
| `--config`, `-c` `<PATH>` | Path to a config file. Overrides the default config resolution. |
| `--intelligent-routing` | Enable vector-memory routing for context-aware message handling. Requires the `intelligent-routing` feature to be compiled in. |

### Examples

Start an interactive REPL session:

```
weft agent
```

Send a single message and print the response:

```
weft agent -m "Summarize today's open issues"
```

Use a specific model:

```
weft agent --model openai/gpt-4o -m "Draft a status update"
```

Use a custom config file with intelligent routing:

```
weft agent -c ./my-config.toml --intelligent-routing
```

---

## weft gateway

Start the gateway process. This launches all enabled channels, the agent loop,
and outbound message dispatch. The process runs in the foreground until
interrupted.

### Usage

```
weft gateway [FLAGS] [OPTIONS]
```

### Options

| Flag / Option | Description |
|---------------|-------------|
| `--config`, `-c` `<PATH>` | Path to a config file. Overrides the default config resolution. |
| `--intelligent-routing` | Enable vector-memory routing for context-aware message handling. |

### Shutdown

Send `SIGINT` (Ctrl+C) for graceful shutdown. The gateway will drain in-flight
messages before exiting.

### Examples

Start the gateway with default config:

```
weft gateway
```

Start with a custom config and intelligent routing:

```
weft gateway -c /etc/weft/production.toml --intelligent-routing
```

---

## weft status

Display a summary of the current system status, including the gateway, active
channels, and agent health.

### Usage

```
weft status [FLAGS] [OPTIONS]
```

### Options

| Flag / Option | Description |
|---------------|-------------|
| `--detailed` | Show expanded status information for each component. |
| `--config`, `-c` `<PATH>` | Path to a config file. Overrides the default config resolution. |

### Examples

Quick status check:

```
weft status
```

Detailed status with all component info:

```
weft status --detailed
```

---

## weft channels status

Display the status of all configured channels.

### Usage

```
weft channels status [OPTIONS]
```

### Options

| Flag / Option | Description |
|---------------|-------------|
| `--config`, `-c` `<PATH>` | Path to a config file. Overrides the default config resolution. |

### Examples

```
weft channels status
```

```
weft channels status -c ./dev-config.toml
```

---

## weft cron

Manage scheduled cron jobs. Each job runs a prompt on a cron schedule through
the agent.

### Subcommands

### weft cron list

List all configured cron jobs.

```
weft cron list [OPTIONS]
```

| Flag / Option | Description |
|---------------|-------------|
| `--config`, `-c` `<PATH>` | Path to a config file. |

### weft cron add

Add a new cron job.

```
weft cron add --name <NAME> --schedule <CRON_EXPR> --prompt <PROMPT> [OPTIONS]
```

| Flag / Option | Description |
|---------------|-------------|
| `--name` `<NAME>` | Human-readable name for the job. Required. |
| `--schedule` `<CRON_EXPR>` | Cron expression defining the schedule (e.g., `"0 9 * * *"`). Required. |
| `--prompt` `<PROMPT>` | The prompt text to send to the agent on each trigger. Required. |
| `--config`, `-c` `<PATH>` | Path to a config file. |

### weft cron remove

Remove a cron job by ID.

```
weft cron remove <JOB_ID> [OPTIONS]
```

| Argument / Option | Description |
|-------------------|-------------|
| `<JOB_ID>` | The ID of the job to remove. Required. |
| `--config`, `-c` `<PATH>` | Path to a config file. |

### weft cron enable

Enable a previously disabled cron job.

```
weft cron enable <JOB_ID> [OPTIONS]
```

| Argument / Option | Description |
|-------------------|-------------|
| `<JOB_ID>` | The ID of the job to enable. Required. |
| `--config`, `-c` `<PATH>` | Path to a config file. |

### weft cron disable

Disable a cron job without removing it.

```
weft cron disable <JOB_ID> [OPTIONS]
```

| Argument / Option | Description |
|-------------------|-------------|
| `<JOB_ID>` | The ID of the job to disable. Required. |
| `--config`, `-c` `<PATH>` | Path to a config file. |

### weft cron run

Manually trigger a cron job immediately, regardless of its schedule.

```
weft cron run <JOB_ID> [OPTIONS]
```

| Argument / Option | Description |
|-------------------|-------------|
| `<JOB_ID>` | The ID of the job to trigger. Required. |
| `--config`, `-c` `<PATH>` | Path to a config file. |

### Examples

List all cron jobs:

```
weft cron list
```

Add a daily summary job at 9 AM UTC:

```
weft cron add --name "daily-summary" --schedule "0 9 * * *" --prompt "Generate a summary of yesterday's activity"
```

Disable a job temporarily:

```
weft cron disable abc123
```

Re-enable a disabled job:

```
weft cron enable abc123
```

Manually run a job for testing:

```
weft cron run abc123
```

Remove a job:

```
weft cron remove abc123
```

---

## weft sessions

Manage agent conversation sessions. Sessions track message history between the
agent and users or channels.

### Subcommands

### weft sessions list

List all sessions in a table showing session key, message count, and last
updated timestamp.

```
weft sessions list [OPTIONS]
```

| Flag / Option | Description |
|---------------|-------------|
| `--config`, `-c` `<PATH>` | Path to a config file. |

### weft sessions inspect

Display the full message history for a session.

```
weft sessions inspect <SESSION_ID> [OPTIONS]
```

| Argument / Option | Description |
|-------------------|-------------|
| `<SESSION_ID>` | The session identifier to inspect. Required. |
| `--config`, `-c` `<PATH>` | Path to a config file. |

### weft sessions delete

Delete a session and its message history.

```
weft sessions delete <SESSION_ID> [OPTIONS]
```

| Argument / Option | Description |
|-------------------|-------------|
| `<SESSION_ID>` | The session identifier to delete. Required. |
| `--config`, `-c` `<PATH>` | Path to a config file. |

### Examples

List all sessions:

```
weft sessions list
```

View messages in a session:

```
weft sessions inspect slack-C04ABCDEF-U01XYZ
```

Delete a session:

```
weft sessions delete slack-C04ABCDEF-U01XYZ
```

---

## weft memory

View and search the agent's persistent memory and history files.

### Subcommands

### weft memory show

Display the contents of `MEMORY.md`.

```
weft memory show [OPTIONS]
```

| Flag / Option | Description |
|---------------|-------------|
| `--config`, `-c` `<PATH>` | Path to a config file. |

### weft memory history

Display the contents of `HISTORY.md`.

```
weft memory history [OPTIONS]
```

| Flag / Option | Description |
|---------------|-------------|
| `--config`, `-c` `<PATH>` | Path to a config file. |

### weft memory search

Search across memory and history for matching entries.

```
weft memory search <QUERY> [OPTIONS]
```

| Argument / Option | Description |
|-------------------|-------------|
| `<QUERY>` | The search term or phrase. Required. |
| `--limit`, `-n` `<N>` | Maximum number of results to return. |
| `--config`, `-c` `<PATH>` | Path to a config file. |

### Examples

Show the current memory file:

```
weft memory show
```

Show the history file:

```
weft memory history
```

Search memory for a topic:

```
weft memory search "deployment procedure"
```

Search with a result limit:

```
weft memory search "auth" --limit 5
```

---

## weft config

Inspect the resolved configuration.

### Subcommands

### weft config show

Print the full resolved configuration as JSON.

```
weft config show [OPTIONS]
```

| Flag / Option | Description |
|---------------|-------------|
| `--config`, `-c` `<PATH>` | Path to a config file. |

### weft config section

Print a specific configuration section as JSON.

```
weft config section <NAME> [OPTIONS]
```

| Argument / Option | Description |
|-------------------|-------------|
| `<NAME>` | The section name to display. One of: `agents`, `gateway`, `channels`, `tools`. Required. |
| `--config`, `-c` `<PATH>` | Path to a config file. |

### Examples

Show the full resolved config:

```
weft config show
```

Show only the channels section:

```
weft config section channels
```

Show agent configuration from a custom file:

```
weft config section agents -c ./staging.toml
```

---

## weft completions

Generate shell completion scripts for `weft`. Completions are printed to
stdout and can be evaluated or saved to a file.

### Usage

```
weft completions <SHELL>
```

### Arguments

| Argument | Description |
|----------|-------------|
| `<SHELL>` | Target shell. One of: `bash`, `zsh`, `fish`, `powershell`. Required. |

### Examples

Enable completions in the current bash session:

```
eval "$(weft completions bash)"
```

Persist completions for zsh (add to `.zshrc`):

```
weft completions zsh > ~/.zfunc/_weft
```

Generate fish completions:

```
weft completions fish > ~/.config/fish/completions/weft.fish
```

Generate PowerShell completions:

```
weft completions powershell > weft.ps1
```
