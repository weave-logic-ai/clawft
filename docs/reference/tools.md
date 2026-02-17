# Tools Reference

clawft provides a set of built-in tools that the AI agent can invoke during its
execution loop. Each tool implements the `Tool` trait and is registered in a
central `ToolRegistry`. External tools can be added via MCP (Model Context
Protocol) servers.

## Tool Trait

All tools implement the following interface:

```rust
#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters(&self) -> serde_json::Value; // JSON Schema
    async fn execute(
        &self,
        args: serde_json::Value,
    ) -> Result<serde_json::Value, ToolError>;
}
```

- **name** -- Unique identifier used in LLM function-calling requests.
- **description** -- Human-readable summary shown to the model.
- **parameters** -- JSON Schema object describing accepted arguments.
- **execute** -- Runs the tool and returns a JSON result or a `ToolError`.

### ToolError Variants

| Variant            | Description                                           |
|--------------------|-------------------------------------------------------|
| `NotFound`         | The requested tool name is not in the registry.       |
| `InvalidArgs`      | The arguments do not satisfy the tool's schema.       |
| `ExecutionFailed`  | A runtime error occurred during execution.            |
| `PermissionDenied` | The operation was blocked by a safety check.          |
| `FileNotFound`     | A required file or resource does not exist.           |
| `InvalidPath`      | The path is invalid or escapes the allowed workspace. |
| `Timeout`          | Execution exceeded the allowed time limit.            |

---

## Tool Call Lifecycle

Tools are invoked as part of the agent's execution loop. When the LLM responds
with one or more `tool_calls` in its message, the agent extracts each call,
looks up the tool in the registry, executes it, and feeds the result back to the
LLM as a tool-result message. This loop repeats until the LLM responds without
any tool calls or the iteration limit is reached.

1. **LLM response** -- The model returns a message containing `tool_calls`.
2. **Dispatch** -- Each tool call is matched by name in the `ToolRegistry`.
3. **Execute** -- The tool runs with the provided arguments.
4. **Result** -- The JSON result (or error) is sent back to the LLM.
5. **Repeat** -- The LLM decides whether to issue more tool calls or produce a
   final answer.

**Iteration limit** -- The maximum number of tool-call rounds defaults to **20**
(`max_tool_iterations`). This can be overridden in the agent configuration.

**Result truncation** -- Tool results are truncated to **64 KB**
(`MAX_TOOL_RESULT_BYTES = 65,536`) before being passed back to the LLM. This
protects against unbounded context growth from large tool outputs.

For a detailed walkthrough of the pipeline, error handling, and parallel tool
execution, see the [Tool Calls guide](../guides/tool-calls.md).

---

## Built-in Tools

### read_file

Read the contents of a file within the workspace.

**Parameters**

| Name   | Type   | Required | Description                              |
|--------|--------|----------|------------------------------------------|
| `path` | string | yes      | File path to read (relative to workspace)|

**Return value**

```json
{ "content": "file contents as a string" }
```

**Example**

```json
{
  "path": "src/main.rs"
}
```

**Security notes**

- All paths are resolved relative to the workspace directory.
- Paths are canonicalized and verified to remain within the workspace boundary.
  Traversal attempts such as `../../../etc/passwd` or absolute paths outside the
  workspace are rejected with an `InvalidPath` or `FileNotFound` error.

---

### write_file

Write content to a file, creating it (and any missing parent directories) if it
does not exist. Overwrites the file if it already exists.

**Parameters**

| Name      | Type   | Required | Description                                |
|-----------|--------|----------|--------------------------------------------|
| `path`    | string | yes      | File path to write (relative to workspace) |
| `content` | string | yes      | Content to write to the file               |

**Return value**

```json
{ "message": "Successfully wrote 42 bytes to src/lib.rs" }
```

**Example**

```json
{
  "path": "config/settings.toml",
  "content": "[server]\nport = 8080\n"
}
```

**Security notes**

- Workspace containment is enforced. The deepest existing ancestor of the target
  path is canonicalized and checked against the workspace root.
- Parent directories are created automatically when they do not exist.

---

### edit_file

Edit an existing file by replacing the first occurrence of a text string with a
new value. The file must contain exactly one occurrence of `old_text`; zero
matches or multiple matches both produce an error.

**Parameters**

| Name       | Type   | Required | Description                                |
|------------|--------|----------|--------------------------------------------|
| `path`     | string | yes      | File path to edit (relative to workspace)  |
| `old_text` | string | yes      | Exact text to find and replace             |
| `new_text` | string | yes      | Replacement text                           |

**Return value**

```json
{ "message": "Successfully edited src/main.rs" }
```

**Example**

```json
{
  "path": "src/main.rs",
  "old_text": "fn main() {}",
  "new_text": "fn main() {\n    println!(\"hello\");\n}"
}
```

**Error conditions**

- `old_text` not found in file -- returns `InvalidArgs` error.
- `old_text` appears more than once -- returns `InvalidArgs` error with the
  match count. Provide more surrounding context to make the match unique.

**Security notes**

- Workspace path containment is enforced identically to `read_file`.

---

### list_directory

List the contents of a directory within the workspace. Returns metadata for each
entry, sorted alphabetically by name.

**Parameters**

| Name   | Type   | Required | Description                                   |
|--------|--------|----------|-----------------------------------------------|
| `path` | string | yes      | Directory path to list (relative to workspace) |

**Return value**

```json
{
  "entries": [
    { "name": "main.rs", "is_dir": false, "size": 1234 },
    { "name": "tests",   "is_dir": true,  "size": 4096 }
  ]
}
```

Each entry contains:

| Field    | Type    | Description                                 |
|----------|---------|---------------------------------------------|
| `name`   | string  | File or directory name (not full path)       |
| `is_dir` | boolean | `true` if the entry is a directory           |
| `size`   | integer | Size in bytes (0 if metadata is unavailable) |

**Example**

```json
{
  "path": "src"
}
```

**Security notes**

- Workspace path containment is enforced.
- Only the immediate children of the directory are listed (non-recursive).

---

### exec_shell

Execute a shell command and capture its output. Commands run via `sh -c` with
the working directory set to the workspace.

**Parameters**

| Name      | Type   | Required | Description                                  |
|-----------|--------|----------|----------------------------------------------|
| `command` | string | yes      | Shell command to execute                     |
| `timeout` | number | no       | Timeout in seconds (default: 30, max: 300)   |

**Return value**

```json
{
  "exit_code": 0,
  "stdout": "command output",
  "stderr": "",
  "duration_ms": 42
}
```

| Field         | Type    | Description                                       |
|---------------|---------|---------------------------------------------------|
| `exit_code`   | integer | Process exit code (`-1` if the exit code is unknown) |
| `stdout`      | string  | Standard output captured from the process         |
| `stderr`      | string  | Standard error captured from the process          |
| `duration_ms` | integer | Wall-clock execution time in milliseconds         |

**Example**

```json
{
  "command": "cargo build --release",
  "timeout": 120
}
```

**Security notes**

- A command denylist blocks dangerous operations before execution. The following
  patterns are rejected (case-insensitive match):

  | Pattern            | Reason                     |
  |--------------------|----------------------------|
  | `rm -rf /`         | Filesystem destruction     |
  | `sudo `            | Privilege escalation       |
  | `mkfs`             | Filesystem formatting      |
  | `dd if=`           | Raw disk write             |
  | `:(){ :\|:& };:`   | Fork bomb                  |
  | `chmod 777 /`      | Dangerous permission change|
  | `> /dev/sd`        | Raw device write           |
  | `shutdown`         | System shutdown            |
  | `reboot`           | System reboot              |
  | `poweroff`         | System power off           |
  | `format c:`        | Disk formatting (Windows)  |

- Commands that match a denylist pattern return a `PermissionDenied` error.
- The requested timeout is clamped to the maximum (300 seconds). If the process
  does not exit within the timeout, it is killed and a `Timeout` error is
  returned.

---

### memory_read

Read from the workspace memory file (`MEMORY.md`). Supports optional
paragraph-level search.

The memory file is resolved at `~/.clawft/workspace/memory/MEMORY.md`, with a
fallback to `~/.nanobot/workspace/memory/MEMORY.md` for backward compatibility.

**Parameters**

| Name    | Type   | Required | Description                                      |
|---------|--------|----------|--------------------------------------------------|
| `query` | string | no       | Search query to filter paragraphs (case-insensitive) |

**Return value (no query)**

```json
{ "content": "full memory file contents" }
```

**Return value (with query)**

```json
{
  "query": "authentication",
  "matches": [
    "Decided to use JWT tokens for authentication..."
  ],
  "count": 1
}
```

When no query is provided, the full file content is returned. When a query is
provided, the content is split into paragraphs (delimited by blank lines) and
only paragraphs containing the query string are returned.

If no memory file exists, returns:

```json
{ "content": "", "message": "No memory file found" }
```

**Example**

```json
{
  "query": "database schema"
}
```

---

### memory_write

Write to the workspace memory file. Supports `append` (default) and `overwrite`
modes. Creates the file and parent directories if they do not exist.

**Parameters**

| Name      | Type   | Required | Description                                        |
|-----------|--------|----------|----------------------------------------------------|
| `content` | string | yes      | Content to write to memory                         |
| `mode`    | string | no       | Write mode: `"append"` (default) or `"overwrite"` |

**Return value**

```json
{
  "message": "Successfully wrote 42 bytes to memory (mode: append)",
  "path": "/home/user/.clawft/workspace/memory/MEMORY.md"
}
```

**Example**

```json
{
  "content": "## Decision\n\nUse PostgreSQL for the primary datastore.",
  "mode": "append"
}
```

**Behavior**

- In `append` mode, content is appended with a blank-line separator if the file
  already has content.
- In `overwrite` mode, the entire file is replaced.

---

### web_search

Search the web using a configured search API endpoint. Returns structured
results including titles, URLs, and snippets.

**Parameters**

| Name          | Type    | Required | Description                                   |
|---------------|---------|----------|-----------------------------------------------|
| `query`       | string  | yes      | Search query string                           |
| `num_results` | integer | no       | Maximum number of results to return (default: 5) |

**Return value (configured)**

```json
{
  "query": "rust async patterns",
  "results": [ ... ]
}
```

The shape of each result depends on the search API backend.

**Return value (not configured)**

```json
{
  "error": "web search not configured",
  "message": "No search API endpoint is configured. Set 'tools.web_search.endpoint' in your config.",
  "query": "rust async patterns"
}
```

**Example**

```json
{
  "query": "tokio runtime tutorial",
  "num_results": 3
}
```

**Configuration**

Requires `tools.web_search.endpoint` to be set in the agent configuration. When
the endpoint is not configured, the tool returns an informational response
rather than an error.

---

### web_fetch

Fetch content from a URL. Returns the response status, content type, and body
as text.

**Parameters**

| Name      | Type   | Required | Description                                    |
|-----------|--------|----------|------------------------------------------------|
| `url`     | string | yes      | URL to fetch (must start with `http://` or `https://`) |
| `method`  | string | no       | HTTP method (default: `"GET"`)                |
| `headers` | object | no       | HTTP headers as key-value string pairs         |

**Return value**

```json
{
  "status": 200,
  "content_type": "text/html; charset=utf-8",
  "body": "<!DOCTYPE html>...",
  "url": "https://example.com",
  "bytes": 1256
}
```

| Field          | Type    | Description                            |
|----------------|---------|----------------------------------------|
| `status`       | integer | HTTP response status code              |
| `content_type` | string  | Content-Type header value              |
| `body`         | string  | Response body as text                  |
| `url`          | string  | The requested URL                      |
| `bytes`        | integer | Total response body size in bytes      |

**Example**

```json
{
  "url": "https://api.example.com/data",
  "method": "GET",
  "headers": {
    "Accept": "application/json"
  }
}
```

**Limits**

- Response bodies larger than 1 MB (1,048,576 bytes) are truncated. A suffix
  indicating truncation is appended.
- Only `http://` and `https://` URL schemes are accepted. Other schemes produce
  an `InvalidArgs` error.

---

### message

Send a message to a specific channel and chat via the internal MessageBus.
Enables cross-channel communication, notifications, and broadcasting.

**Parameters**

| Name      | Type   | Required | Description                                          |
|-----------|--------|----------|------------------------------------------------------|
| `channel` | string | yes      | Target channel (e.g., `"telegram"`, `"slack"`, `"discord"`) |
| `chat_id` | string | yes      | Target chat or conversation ID                       |
| `content` | string | yes      | Message content to send                              |

**Return value**

```json
{
  "status": "sent",
  "channel": "telegram",
  "chat_id": "12345",
  "content_length": 18
}
```

**Example**

```json
{
  "channel": "slack",
  "chat_id": "C024BE91L",
  "content": "Build completed successfully."
}
```

**Notes**

- The `message` tool is registered separately from other built-in tools because
  it requires a reference to the MessageBus.
- Messages are dispatched asynchronously through the bus. The return value
  confirms dispatch, not delivery.

---

### spawn

Spawn a subprocess to run a command. Processes execute in the workspace
directory with a concurrency limit.

**Parameters**

| Name          | Type     | Required | Description                                  |
|---------------|----------|----------|----------------------------------------------|
| `command`     | string   | yes      | Command to execute                           |
| `args`        | string[] | no       | Command arguments                            |
| `description` | string   | no       | Human-readable description of the task       |
| `timeout`     | number   | no       | Timeout in seconds (default: 60)             |

**Return value**

```json
{
  "exit_code": 0,
  "stdout": "hello world\n",
  "stderr": "",
  "command": "echo",
  "description": "test echo"
}
```

| Field         | Type    | Description                               |
|---------------|---------|-------------------------------------------|
| `exit_code`   | integer | Process exit code                         |
| `stdout`      | string  | Captured standard output                  |
| `stderr`      | string  | Captured standard error                   |
| `command`     | string  | The command that was executed              |
| `description` | string  | The description provided (or `"(no description)"`) |

**Example**

```json
{
  "command": "cargo",
  "args": ["test", "--release"],
  "description": "Run tests in release mode",
  "timeout": 120
}
```

**Limits**

- Maximum 5 concurrent spawned processes. Attempts to exceed this limit return
  an `ExecutionFailed` error with a message indicating the concurrency limit.
- If the platform does not support process spawning, an `ExecutionFailed` error
  is returned.

---

## MCP Tools

External tools can be integrated through MCP (Model Context Protocol) servers.
MCP tools are discovered at startup, wrapped, and registered in the same
`ToolRegistry` used by built-in tools.

### Configuration

MCP servers are defined in the `tools.mcp_servers` section of the agent
configuration. Each server entry specifies either a command (stdio transport) or
a URL (HTTP transport):

```toml
[tools.mcp_servers.my_server]
command = "npx"
args = ["-y", "my-mcp-server"]

[tools.mcp_servers.remote_server]
url = "http://localhost:3000/mcp"
```

### Naming Convention

Each MCP tool is registered with a namespaced name to avoid collisions:

```
{server_name}__{tool_name}
```

For example, a tool named `search` from a server named `web` is registered as
`web__search`.

### Transport

- **Stdio** -- If `command` is set, a child process is spawned and
  communication happens over stdin/stdout.
- **HTTP** -- If `url` is set (and `command` is empty), HTTP requests are used.
- If neither is set, the server is skipped with a warning.

### Registration

MCP tools are registered via `register_mcp_tools()` after built-in tools. They
appear alongside built-in tools in the registry and can be invoked in the same
way by the agent.

---

## Tool Registration

Built-in tools are registered by calling `clawft_tools::register_all()`, which
takes a `ToolRegistry`, a `Platform` instance, and the workspace directory path.
File tools and the shell tool are sandboxed to the workspace. The `message` tool
is registered separately because it requires a `MessageBus` reference.

Registration order:

1. `register_all()` -- registers `read_file`, `write_file`, `edit_file`,
   `list_directory`, `exec_shell`, `memory_read`, `memory_write`, `web_search`,
   `web_fetch`, and `spawn`.
2. `register_mcp_tools()` -- discovers and registers tools from configured MCP
   servers.
3. `MessageTool` -- registered with a reference to the MessageBus.

If a tool is registered with a name that already exists, the new registration
replaces the previous one.

---

## Security

### Workspace Containment

All file tools (`read_file`, `write_file`, `edit_file`, `list_directory`)
enforce workspace path containment:

1. The provided path is joined to the workspace directory.
2. The result is canonicalized (resolving symlinks and `.`/`..` components).
3. The canonical path must start with the canonical workspace directory.
4. If the check fails, an `InvalidPath` error is returned.

For write operations on paths that do not yet exist, the deepest existing
ancestor is canonicalized and checked instead.

### Shell Command Denylist

The `exec_shell` tool rejects commands matching a set of dangerous patterns
before execution. Pattern matching is case-insensitive and checks for substring
containment. Blocked commands receive a `PermissionDenied` error.

### Output Truncation

Tool output is truncated to 64 KB (65,536 bytes) before being passed back to
the LLM. This limit is enforced by the agent loop, not by individual tools.
The `web_fetch` tool additionally enforces its own 1 MB limit on response
bodies.

### Concurrency Limits

The `spawn` tool enforces a maximum of 5 concurrent subprocesses via an atomic
counter. The `exec_shell` tool enforces a configurable timeout (default 30
seconds, maximum 300 seconds) and kills processes that exceed it.
