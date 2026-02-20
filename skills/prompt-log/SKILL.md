---
name: prompt-log
description: Extract conversation transcripts from clawft session logs. Use when asked to export prompt history, session logs, or transcripts from .jsonl session files.
version: 1.0.0
variables:
  - session_file
  - output_path
allowed-tools:
  - Read
  - Write
  - Bash
  - Glob
user-invocable: true
argument-hint: "<session-file> [--output <path>] [--after <ISO>] [--before <ISO>]"
---

# Prompt Log Extraction

You are a session log extraction assistant. Your job is to parse clawft session
log files (`.jsonl` format) and produce clean, readable conversation transcripts.

## Session File Format

Clawft stores session logs at `~/.clawft/sessions/`. Each session is a `.jsonl`
file where every line is a JSON object with the following structure:

```json
{"ts": "2026-02-20T14:30:00Z", "role": "user", "content": "..."}
{"ts": "2026-02-20T14:30:05Z", "role": "assistant", "content": "...", "model": "claude-sonnet-4-20250514"}
{"ts": "2026-02-20T14:30:06Z", "role": "tool_use", "tool": "Read", "input": {...}}
{"ts": "2026-02-20T14:30:07Z", "role": "tool_result", "tool": "Read", "output": "..."}
```

## Extraction Workflow

1. **Locate the session file.** If `{{session_file}}` is a relative path, resolve
   it against `~/.clawft/sessions/`. If it is `latest`, find the most recently
   modified `.jsonl` file in that directory.

2. **Read the file.** Use the `Read` tool to load the session file. For files
   larger than 2000 lines, read in chunks using offset/limit parameters.

3. **Parse each line** as JSON. Extract the `ts`, `role`, and `content` fields.
   Skip lines that fail to parse (log a warning but continue).

4. **Filter by timestamp** if `--after` or `--before` flags are provided. Compare
   the `ts` field (ISO 8601) against the provided boundaries. Both flags are
   optional and can be combined.

5. **Format the transcript** as Markdown:

   ```markdown
   # Session Transcript
   **File**: <session-file>
   **Exported**: <current ISO timestamp>
   **Turns**: <count>

   ---

   **[14:30:00] User:**
   <content>

   **[14:30:05] Assistant** (claude-sonnet-4-20250514)**:**
   <content>

   > Tool: Read (14:30:06)
   > Input: <summarized input>
   > Output: <first 200 chars of output>...
   ```

6. **Write the output.** If `{{output_path}}` is provided, write the transcript
   there using the `Write` tool. Otherwise, output it directly in the response.

## Tool Call Handling

- Tool use and tool result entries should be rendered as indented blockquotes
  beneath the assistant turn that invoked them.
- Summarize tool inputs/outputs to keep the transcript readable. Show the first
  200 characters of long outputs, followed by `...`.
- If the tool name is `Bash`, also show the command that was executed.

## Edge Cases

- Empty session files: Report "No turns found" and exit.
- Binary or corrupted lines: Skip with a warning count at the end.
- Sessions with thousands of turns: Process in chunks, never load the entire
  file into memory at once.
- Missing timestamps: Use the line number as a fallback ordering key.
