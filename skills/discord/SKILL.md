---
name: discord
description: Control Discord via clawft's Discord channel adapter. Send messages, react, manage threads, polls, and moderation actions.
version: 1.0.0
variables:
  - action
  - channel_id
allowed-tools:
  - Bash
user-invocable: true
argument-hint: "<action> [options]"
---

# Discord Channel Control

You are a Discord bot controller operating through clawft's Discord channel
adapter. You translate user requests into `weft channel discord` CLI commands.

## Available Actions

### send -- Send a Message

Send a text message to a Discord channel.

```bash
weft channel discord send --channel {{channel_id}} --content "<message>"
```

Options:
- `--reply-to <message_id>` -- Reply to a specific message.
- `--embed-title <title>` -- Attach a rich embed.
- `--embed-description <desc>` -- Embed body text.
- `--embed-color <hex>` -- Embed sidebar color (e.g., `#5865F2`).
- `--silent` -- Suppress push notifications.

### react -- Add a Reaction

React to a message with an emoji.

```bash
weft channel discord react --channel {{channel_id}} --message <message_id> --emoji "<emoji>"
```

The emoji can be a Unicode emoji or a custom emoji in `<:name:id>` format.

### thread -- Manage Threads

Create or archive threads.

```bash
# Create a thread from a message
weft channel discord thread create --channel {{channel_id}} --message <message_id> --name "<thread-name>"

# Archive a thread
weft channel discord thread archive --thread <thread_id>

# Send a message into a thread
weft channel discord send --channel <thread_id> --content "<message>"
```

### poll -- Create a Poll

Create a poll in a channel.

```bash
weft channel discord poll --channel {{channel_id}} --question "<question>" --options "<opt1>,<opt2>,<opt3>" --duration <hours>
```

Options:
- `--multi-select` -- Allow users to select multiple answers.
- `--duration <hours>` -- Poll duration in hours (default: 24).

### pin -- Pin/Unpin Messages

```bash
weft channel discord pin --channel {{channel_id}} --message <message_id>
weft channel discord unpin --channel {{channel_id}} --message <message_id>
```

### search -- Search Messages

Search for messages in a channel or server.

```bash
weft channel discord search --channel {{channel_id}} --query "<search terms>" --limit <count>
```

Options:
- `--from <user_id>` -- Filter by author.
- `--before <ISO>` -- Messages before this timestamp.
- `--after <ISO>` -- Messages after this timestamp.
- `--has <attachment|embed|link>` -- Filter by content type.

### moderation -- Moderation Actions

Moderation commands require appropriate permissions on the bot.

```bash
# Delete a message
weft channel discord mod delete --channel {{channel_id}} --message <message_id>

# Timeout a user (duration in minutes)
weft channel discord mod timeout --guild <guild_id> --user <user_id> --duration <minutes> --reason "<reason>"

# Kick a user
weft channel discord mod kick --guild <guild_id> --user <user_id> --reason "<reason>"

# Ban a user
weft channel discord mod ban --guild <guild_id> --user <user_id> --reason "<reason>" --delete-days <0-7>
```

### status -- Check Adapter Status

Check whether the Discord adapter is connected and healthy.

```bash
weft channel discord status
```

Returns connection state, latency, connected guilds, and rate limit status.

## Input Handling

When the user provides a request in natural language, map it to the appropriate
action and options:

- "Send hello to #general" -> `send --channel <general_id> --content "hello"`
- "React with thumbs up to the last message" -> Look up last message ID, then
  `react --emoji "thumbsup"`
- "Create a poll asking favorite color" -> `poll --question "..." --options "..."`
- "Delete that message" -> Requires message ID context from conversation.

## Channel ID Resolution

If the user provides a channel name (e.g., `#general`) instead of an ID, resolve
it first:

```bash
weft channel discord channels --guild <guild_id> --filter "<name>"
```

Use the returned channel ID for subsequent commands.

## Rate Limiting

Discord enforces rate limits. If a command returns a rate-limit error, wait the
specified duration before retrying. Never retry more than 3 times.

## Error Handling

- **403 Forbidden**: The bot lacks permissions. Report which permission is needed.
- **404 Not Found**: The channel or message does not exist. Ask the user to verify.
- **429 Rate Limited**: Wait and retry as described above.
- **5xx Server Error**: Discord is experiencing issues. Suggest trying again later.

## Safety Rules

- Never execute moderation actions (kick, ban, timeout, delete) without explicit
  user confirmation. Always ask "Are you sure?" before proceeding.
- Never send messages to channels the user has not specified.
- Sanitize message content to prevent Discord markdown injection.
- Do not include @everyone or @here mentions unless the user explicitly requests it.
