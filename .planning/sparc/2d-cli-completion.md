# SPARC Implementation Plan: Stream 2D - CLI Completion

**Timeline**: Week 7-11
**Owned Crates**: `clawft-cli` (additional commands)
**Dependencies**: Phase 1 complete, Stream 2A (channel plugins), Stream 2C (cron, provider login)

---

## 1. Agent Instructions

### Python Source Files to Read
```
repos/nanobot/nanobot/cli/channels.py           # Channel status command
repos/nanobot/nanobot/cli/cron.py               # Cron commands (list, add, remove, etc.)
repos/nanobot/nanobot/cli/provider.py           # Provider login command
repos/nanobot/nanobot/utils/markdown.py         # Markdown conversion utilities
```

### Planning Documents (MUST READ)
```
repos/nanobot/.planning/02-technical-requirements.md    # CLI commands spec
repos/nanobot/.planning/03-development-guide.md         # Stream 2D timeline
repos/nanobot/.planning/01-project-overview.md          # CLI overview
```

### Module Structure
```
clawft-cli/
├── Cargo.toml                          # Dependencies: clap 4.5, comfy-table 7.0, pulldown-cmark
├── src/
│   ├── main.rs                         # CLI entrypoint
│   ├── commands/
│   │   ├── mod.rs                      # Command registry
│   │   ├── channels.rs                 # NEW: weft channels status
│   │   ├── cron.rs                     # NEW: weft cron {list,add,remove,enable,run}
│   │   ├── provider.rs                 # NEW: weft provider login
│   │   └── chat.rs                     # Existing: weft chat
│   ├── markdown/
│   │   ├── mod.rs                      # NEW: MarkdownConverter trait
│   │   ├── telegram.rs                 # NEW: Telegram HTML conversion
│   │   ├── slack.rs                    # NEW: Slack mrkdwn conversion
│   │   └── discord.rs                  # NEW: Discord markdown conversion
│   └── output/
│       ├── mod.rs                      # Output formatting utilities
│       └── table.rs                    # Table display (comfy-table wrapper)
├── tests/
│   ├── channels_tests.rs               # Channel status command tests
│   ├── cron_tests.rs                   # Cron command tests
│   ├── provider_tests.rs               # Provider login tests
│   └── markdown_tests.rs               # Markdown conversion tests
```

---

## 2. Specification

### 2.1 Channel Status Command (Week 7)

#### `weft channels status`
**Purpose**: Display table of all configured channel plugins and their status

**Output Format**:
```
CHANNEL   STATUS     CONNECTED  LAST MESSAGE
telegram  running    yes        2025-01-15 14:32:10
slack     running    yes        2025-01-15 14:30:05
discord   stopped    no         -
cli       running    yes        2025-01-15 14:32:15
```

**Data Source**:
- Read from clawft-core ChannelManager
- Query each Channel::is_running()
- Query each Channel::last_message_timestamp() (if available)

**Table Columns**:
- **CHANNEL**: Channel name (e.g., "telegram", "slack", "discord", "cli")
- **STATUS**: "running" | "stopped" | "error"
- **CONNECTED**: "yes" | "no" (WebSocket/HTTP connection status)
- **LAST MESSAGE**: Timestamp of last received message (format: "YYYY-MM-DD HH:MM:SS")

**API**:
```rust
pub async fn channels_status_command(config: &Config, channel_manager: &ChannelManager) -> Result<()> {
    let mut table = Table::new();
    table.set_header(vec!["CHANNEL", "STATUS", "CONNECTED", "LAST MESSAGE"]);

    for channel_config in &config.channels {
        let channel = channel_manager.get_channel(&channel_config.name)?;
        let status = if channel.is_running() { "running" } else { "stopped" };
        let connected = if channel.is_connected() { "yes" } else { "no" };
        let last_message = channel.last_message_timestamp()
            .map(|ts| format_timestamp(ts))
            .unwrap_or("-".to_string());

        table.add_row(vec![
            channel_config.name.clone(),
            status.to_string(),
            connected.to_string(),
            last_message,
        ]);
    }

    println!("{}", table);
    Ok(())
}
```

### 2.2 Cron Commands (Week 8)

#### `weft cron list`
**Purpose**: Display table of all cron jobs

**Output Format**:
```
ID       NAME            SCHEDULE      ENABLED  LAST RUN              NEXT RUN
abc123   daily-summary   0 0 * * *     true     2025-01-15 00:00:00   2025-01-16 00:00:00
def456   hourly-check    0 * * * *     false    -                     -
ghi789   weekly-report   0 0 * * 0     true     2025-01-14 00:00:00   2025-01-21 00:00:00
```

**API**:
```rust
pub async fn cron_list_command(cron_service: &CronService) -> Result<()> {
    let jobs = cron_service.list_jobs().await?;

    let mut table = Table::new();
    table.set_header(vec!["ID", "NAME", "SCHEDULE", "ENABLED", "LAST RUN", "NEXT RUN"]);

    for job in jobs {
        table.add_row(vec![
            job.id,
            job.name,
            job.schedule,
            job.enabled.to_string(),
            job.last_run.map(format_timestamp).unwrap_or("-".to_string()),
            job.next_run.map(format_timestamp).unwrap_or("-".to_string()),
        ]);
    }

    println!("{}", table);
    Ok(())
}
```

#### `weft cron add`
**Purpose**: Add new cron job

**Usage**:
```bash
weft cron add --name daily-summary --schedule "0 0 * * *" --prompt "Summarize today's work"
```

**Arguments**:
- `--name <name>`: Job name (required)
- `--schedule <cron>`: Cron expression (required)
- `--prompt <prompt>`: LLM prompt to execute (required)

**Validation**:
- Cron expression must be valid (use `cron` crate to validate)
- Name must be unique (check existing jobs)

**API**:
```rust
pub async fn cron_add_command(
    cron_service: &CronService,
    name: String,
    schedule: String,
    prompt: String,
) -> Result<()> {
    // Validate cron expression
    let _ = Schedule::from_str(&schedule)?;

    let job = CronJob {
        id: Uuid::new_v4().to_string(),
        name: name.clone(),
        schedule,
        prompt,
        enabled: true,
        last_run: None,
        next_run: None,
        created_at: SystemTime::now(),
    };

    let job_id = cron_service.add_job(job).await?;

    println!("✓ Cron job '{}' created with ID: {}", name, job_id);
    Ok(())
}
```

#### `weft cron remove <job_id>`
**Purpose**: Remove cron job

**Usage**:
```bash
weft cron remove abc123
```

**API**:
```rust
pub async fn cron_remove_command(cron_service: &CronService, job_id: String) -> Result<()> {
    cron_service.remove_job(&job_id).await?;
    println!("✓ Cron job '{}' removed", job_id);
    Ok(())
}
```

#### `weft cron enable <job_id>` / `weft cron disable <job_id>`
**Purpose**: Enable/disable cron job

**Usage**:
```bash
weft cron enable abc123
weft cron disable abc123
```

**API**:
```rust
pub async fn cron_enable_command(cron_service: &CronService, job_id: String, enabled: bool) -> Result<()> {
    cron_service.enable_job(&job_id, enabled).await?;
    let status = if enabled { "enabled" } else { "disabled" };
    println!("✓ Cron job '{}' {}", job_id, status);
    Ok(())
}
```

#### `weft cron run <job_id>`
**Purpose**: Execute cron job immediately (bypass schedule)

**Usage**:
```bash
weft cron run abc123
```

**API**:
```rust
pub async fn cron_run_command(cron_service: &CronService, job_id: String) -> Result<()> {
    cron_service.run_job_now(&job_id).await?;
    println!("✓ Cron job '{}' executed", job_id);
    Ok(())
}
```

### 2.3 Provider Login Command (Week 9)

#### `weft provider login <provider>`
**Purpose**: Authenticate with provider via OAuth

**Supported Providers**:
- `codex` - OpenAI Codex OAuth

**Usage**:
```bash
weft provider login codex
```

**Flow**:
1. Start local HTTP server on `http://localhost:8765`
2. Open browser to OAuth authorization URL
3. Wait for OAuth callback with code
4. Exchange code for access token
5. Store token in `~/.config/weft/config.yaml`
6. Shutdown HTTP server

**API**:
```rust
pub async fn provider_login_command(provider: String) -> Result<()> {
    match provider.as_str() {
        "codex" => codex_oauth_login().await?,
        _ => return Err(anyhow!("Unknown provider: {}", provider)),
    }

    println!("✓ Successfully authenticated with {}", provider);
    Ok(())
}

async fn codex_oauth_login() -> Result<()> {
    println!("Opening browser for OAuth login...");

    let token = oauth::codex_oauth_flow().await?;

    // Store token in config
    let config_path = config_dir()?.join("config.yaml");
    let mut config = Config::load(&config_path)?;
    config.providers.insert("codex".to_string(), ProviderConfig {
        api_key: Some(token),
        ..Default::default()
    });
    config.save(&config_path)?;

    Ok(())
}
```

### 2.4 Markdown Conversion (Week 10-11)

#### MarkdownConverter Trait
```rust
pub trait MarkdownConverter {
    fn convert(&self, markdown: &str) -> String;
}
```

#### Telegram HTML Conversion (clawft-cli/src/markdown/telegram.rs)
**Purpose**: Convert Markdown → Telegram HTML

**Supported Tags**:
```
**bold** → <b>bold</b>
*italic* → <i>italic</i>
`code` → <code>code</code>
```blocks → <pre>blocks</pre>
[link](url) → <a href="url">link</a>
```

**Implementation**:
```rust
pub struct TelegramMarkdownConverter;

impl MarkdownConverter for TelegramMarkdownConverter {
    fn convert(&self, markdown: &str) -> String {
        let parser = Parser::new(markdown);
        let mut html = String::new();

        for event in parser {
            match event {
                Event::Start(Tag::Strong) => html.push_str("<b>"),
                Event::End(Tag::Strong) => html.push_str("</b>"),
                Event::Start(Tag::Emphasis) => html.push_str("<i>"),
                Event::End(Tag::Emphasis) => html.push_str("</i>"),
                Event::Code(code) => html.push_str(&format!("<code>{}</code>", code)),
                Event::Start(Tag::CodeBlock(_)) => html.push_str("<pre>"),
                Event::End(Tag::CodeBlock(_)) => html.push_str("</pre>"),
                Event::Start(Tag::Link(_, url, _)) => html.push_str(&format!("<a href=\"{}\">", url)),
                Event::End(Tag::Link(..)) => html.push_str("</a>"),
                Event::Text(text) => html.push_str(&text),
                _ => {}
            }
        }

        html
    }
}
```

#### Slack mrkdwn Conversion (clawft-cli/src/markdown/slack.rs)
**Purpose**: Convert Markdown → Slack mrkdwn

**Transformations**:
```
**bold** → *bold*
*italic* → _italic_
`code` → `code`
```blocks → ```blocks
[link](url) → <url|link>
@user → <@U123>
#channel → <#C123>
```

**Implementation**:
```rust
pub struct SlackMarkdownConverter;

impl MarkdownConverter for SlackMarkdownConverter {
    fn convert(&self, markdown: &str) -> String {
        let parser = Parser::new(markdown);
        let mut slack = String::new();

        for event in parser {
            match event {
                Event::Start(Tag::Strong) => slack.push('*'),
                Event::End(Tag::Strong) => slack.push('*'),
                Event::Start(Tag::Emphasis) => slack.push('_'),
                Event::End(Tag::Emphasis) => slack.push('_'),
                Event::Code(code) => slack.push_str(&format!("`{}`", code)),
                Event::Start(Tag::CodeBlock(_)) => slack.push_str("```\n"),
                Event::End(Tag::CodeBlock(_)) => slack.push_str("\n```"),
                Event::Start(Tag::Link(_, url, _)) => {
                    slack.push('<');
                    slack.push_str(&url);
                    slack.push('|');
                }
                Event::End(Tag::Link(..)) => slack.push('>'),
                Event::Text(text) => {
                    // Convert @mentions and #channels
                    let converted = text
                        .replace("@user", "<@U123>")  // Placeholder: lookup user ID
                        .replace("#channel", "<#C123>"); // Placeholder: lookup channel ID
                    slack.push_str(&converted);
                }
                _ => {}
            }
        }

        slack
    }
}
```

#### Discord Markdown Conversion (clawft-cli/src/markdown/discord.rs)
**Purpose**: Convert Markdown → Discord markdown

**Transformations** (most unchanged):
```
**bold** → **bold**
*italic* → *italic*
`code` → `code`
||spoiler|| → ||spoiler||
<t:1234567890:R> → <t:1234567890:R> (timestamp)
> quote → > quote
```

**Implementation**:
```rust
pub struct DiscordMarkdownConverter;

impl MarkdownConverter for DiscordMarkdownConverter {
    fn convert(&self, markdown: &str) -> String {
        // Discord uses standard Markdown, so minimal conversion needed
        // Only special handling for custom Discord features (spoilers, timestamps, etc.)

        let mut result = markdown.to_string();

        // Discord-specific features (pass through unchanged)
        // ||spoiler|| - already valid
        // <t:1234567890:R> - already valid
        // > quote - already valid

        result
    }
}
```

---

## 3. Pseudocode

### 3.1 Channel Status Command
```rust
pub async fn channels_status_command(
    config: &Config,
    channel_manager: &ChannelManager,
) -> Result<()> {
    let mut table = Table::new();
    table.set_header(vec!["CHANNEL", "STATUS", "CONNECTED", "LAST MESSAGE"]);

    for channel_config in &config.channels {
        let channel = match channel_manager.get_channel(&channel_config.name) {
            Ok(ch) => ch,
            Err(_) => {
                // Channel not found, show error status
                table.add_row(vec![
                    channel_config.name.clone(),
                    "error".to_string(),
                    "no".to_string(),
                    "-".to_string(),
                ]);
                continue;
            }
        };

        let status = if channel.is_running() { "running" } else { "stopped" };
        let connected = if channel.is_connected() { "yes" } else { "no" };
        let last_message = channel.last_message_timestamp()
            .map(|ts| {
                let datetime: DateTime<Local> = ts.into();
                datetime.format("%Y-%m-%d %H:%M:%S").to_string()
            })
            .unwrap_or("-".to_string());

        table.add_row(vec![
            channel_config.name.clone(),
            status.to_string(),
            connected.to_string(),
            last_message,
        ]);
    }

    println!("{}", table);
    Ok(())
}
```

### 3.2 Cron List Command
```rust
pub async fn cron_list_command(cron_service: &CronService) -> Result<()> {
    let jobs = cron_service.list_jobs().await?;

    if jobs.is_empty() {
        println!("No cron jobs configured.");
        return Ok(());
    }

    let mut table = Table::new();
    table.set_header(vec!["ID", "NAME", "SCHEDULE", "ENABLED", "LAST RUN", "NEXT RUN"]);

    for job in jobs {
        let last_run = job.last_run
            .map(|ts| {
                let datetime: DateTime<Local> = ts.into();
                datetime.format("%Y-%m-%d %H:%M:%S").to_string()
            })
            .unwrap_or("-".to_string());

        let next_run = job.next_run
            .map(|ts| {
                let datetime: DateTime<Local> = ts.into();
                datetime.format("%Y-%m-%d %H:%M:%S").to_string()
            })
            .unwrap_or("-".to_string());

        table.add_row(vec![
            job.id,
            job.name,
            job.schedule,
            if job.enabled { "true" } else { "false" }.to_string(),
            last_run,
            next_run,
        ]);
    }

    println!("{}", table);
    Ok(())
}
```

### 3.3 Cron Add Command
```rust
pub async fn cron_add_command(
    cron_service: &CronService,
    name: String,
    schedule: String,
    prompt: String,
) -> Result<()> {
    // Validate cron expression
    if let Err(e) = Schedule::from_str(&schedule) {
        return Err(anyhow!("Invalid cron expression '{}': {}", schedule, e));
    }

    // Check for duplicate name
    let existing_jobs = cron_service.list_jobs().await?;
    if existing_jobs.iter().any(|j| j.name == name) {
        return Err(anyhow!("Cron job with name '{}' already exists", name));
    }

    let job = CronJob {
        id: Uuid::new_v4().to_string(),
        name: name.clone(),
        schedule: schedule.clone(),
        prompt,
        enabled: true,
        last_run: None,
        next_run: None,
        created_at: SystemTime::now(),
    };

    let job_id = cron_service.add_job(job).await?;

    println!("✓ Cron job '{}' created with ID: {}", name, job_id);
    println!("  Schedule: {}", schedule);
    Ok(())
}
```

### 3.4 Provider Login (Codex OAuth)
```rust
async fn codex_oauth_login() -> Result<()> {
    println!("Opening browser for OAuth login...");

    // Start local HTTP server
    let listener = TcpListener::bind("127.0.0.1:8765").await?;
    let (tx, mut rx) = tokio::sync::oneshot::channel::<String>();

    let server_task = tokio::spawn(async move {
        if let Ok((stream, _)) = listener.accept().await {
            let mut reader = BufReader::new(stream);

            // Read HTTP request
            let mut request_line = String::new();
            if reader.read_line(&mut request_line).await.is_ok() {
                // Parse code from query params
                if let Some(code) = extract_oauth_code(&request_line) {
                    let _ = tx.send(code);
                }
            }

            // Send HTTP response
            let response = "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: 50\r\n\r\n<html><body>Login successful! Close this window.</body></html>";
            let _ = reader.get_mut().write_all(response.as_bytes()).await;
        }
    });

    // Open browser
    let auth_url = format!(
        "https://api.openai.com/v1/oauth/authorize?client_id={}&redirect_uri=http://localhost:8765/callback&response_type=code",
        env::var("CODEX_CLIENT_ID")?
    );
    open::that(auth_url)?;

    // Wait for OAuth callback (timeout: 60 seconds)
    let code = tokio::time::timeout(Duration::from_secs(60), rx).await??;

    // Exchange code for token
    let token_response = reqwest::Client::new()
        .post("https://api.openai.com/v1/oauth/token")
        .json(&json!({
            "client_id": env::var("CODEX_CLIENT_ID")?,
            "client_secret": env::var("CODEX_CLIENT_SECRET")?,
            "code": code,
            "grant_type": "authorization_code",
            "redirect_uri": "http://localhost:8765/callback"
        }))
        .send().await?
        .json::<TokenResponse>().await?;

    // Store token in config
    let config_path = config_dir()?.join("weft").join("config.yaml");
    let mut config = Config::load(&config_path)?;
    config.providers.insert("codex".to_string(), ProviderConfig {
        api_key: Some(token_response.access_token),
        base_url: Some("https://api.openai.com/v1".to_string()),
    });
    config.save(&config_path)?;

    // Wait for server task to complete
    let _ = server_task.await;

    Ok(())
}

fn extract_oauth_code(request_line: &str) -> Option<String> {
    // Parse: GET /callback?code=abc123 HTTP/1.1
    let parts: Vec<&str> = request_line.split_whitespace().collect();
    if parts.len() < 2 {
        return None;
    }

    let path = parts[1];
    if let Some(query_start) = path.find('?') {
        let query = &path[query_start + 1..];
        for param in query.split('&') {
            if let Some((key, value)) = param.split_once('=') {
                if key == "code" {
                    return Some(value.to_string());
                }
            }
        }
    }

    None
}
```

### 3.5 Markdown Conversion (Telegram HTML)
```rust
impl MarkdownConverter for TelegramMarkdownConverter {
    fn convert(&self, markdown: &str) -> String {
        let parser = Parser::new_ext(markdown, Options::all());
        let mut html = String::new();
        let mut in_link = false;
        let mut link_url = String::new();

        for event in parser {
            match event {
                Event::Start(Tag::Strong) => html.push_str("<b>"),
                Event::End(Tag::Strong) => html.push_str("</b>"),

                Event::Start(Tag::Emphasis) => html.push_str("<i>"),
                Event::End(Tag::Emphasis) => html.push_str("</i>"),

                Event::Code(code) => {
                    html.push_str("<code>");
                    html.push_str(&escape_html(&code));
                    html.push_str("</code>");
                }

                Event::Start(Tag::CodeBlock(_)) => html.push_str("<pre>"),
                Event::End(Tag::CodeBlock(_)) => html.push_str("</pre>"),

                Event::Start(Tag::Link(_, url, _)) => {
                    in_link = true;
                    link_url = url.to_string();
                    html.push_str(&format!("<a href=\"{}\">", escape_html(&url)));
                }
                Event::End(Tag::Link(..)) => {
                    in_link = false;
                    html.push_str("</a>");
                }

                Event::Text(text) => {
                    html.push_str(&escape_html(&text));
                }

                Event::SoftBreak => html.push('\n'),
                Event::HardBreak => html.push_str("<br>"),

                _ => {}
            }
        }

        html
    }
}

fn escape_html(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
```

---

## 4. Architecture

### 4.1 CLI Command Structure
```
weft (main.rs)
├── channels
│   └── status → channels_status_command()
│
├── cron
│   ├── list → cron_list_command()
│   ├── add → cron_add_command()
│   ├── remove → cron_remove_command()
│   ├── enable → cron_enable_command()
│   ├── disable → cron_enable_command(enabled=false)
│   └── run → cron_run_command()
│
├── provider
│   └── login <provider> → provider_login_command()
│
└── chat (existing)
    └── ... → chat_command()
```

### 4.2 Markdown Conversion Architecture
```
MarkdownConverter trait
├── TelegramMarkdownConverter
│   ├── convert(markdown) → HTML
│   └── escape_html(text)
│
├── SlackMarkdownConverter
│   ├── convert(markdown) → mrkdwn
│   └── convert_mentions(text)
│
└── DiscordMarkdownConverter
    └── convert(markdown) → Discord markdown (mostly unchanged)
```

### 4.3 Dependency Graph
```
clawft-cli
├── clawft-core (Config, ChannelManager, MessageBus)
├── clawft-services (CronService, provider OAuth)
├── clap 4.5 (CLI argument parsing)
├── comfy-table 7.0 (table display)
├── pulldown-cmark 0.12 (Markdown parsing)
├── tokio 1.42 (async runtime)
├── reqwest 0.12 (HTTP client for OAuth)
├── open 5.0 (open browser)
└── chrono 0.4 (timestamp formatting)
```

---

## 5. Refinement (TDD Test Plan)

### 5.1 Channel Status Command Tests

```rust
// tests/channels_tests.rs
#[tokio::test]
async fn test_channels_status_command() {
    let config = Config {
        channels: vec![
            ChannelConfig { name: "telegram".to_string(), enabled: true, ..Default::default() },
            ChannelConfig { name: "slack".to_string(), enabled: true, ..Default::default() },
        ],
        ..Default::default()
    };

    let channel_manager = MockChannelManager::new();
    channel_manager.expect_get_channel("telegram")
        .returning(|| Ok(MockChannel { running: true, connected: true, last_message: Some(SystemTime::now()) }));
    channel_manager.expect_get_channel("slack")
        .returning(|| Ok(MockChannel { running: false, connected: false, last_message: None }));

    // Execute command
    channels_status_command(&config, &channel_manager).await.unwrap();

    // Verify output (manual inspection or capture stdout)
}

#[test]
fn test_timestamp_formatting() {
    let timestamp = SystemTime::UNIX_EPOCH + Duration::from_secs(1704067200); // 2025-01-01 00:00:00 UTC
    let formatted = format_timestamp(timestamp);
    assert!(formatted.starts_with("2025-01-01"));
}
```

### 5.2 Cron Command Tests

```rust
// tests/cron_tests.rs
#[tokio::test]
async fn test_cron_list_command() {
    let cron_service = MockCronService::new();
    cron_service.expect_list_jobs()
        .returning(|| Ok(vec![
            CronJob {
                id: "abc123".to_string(),
                name: "daily-summary".to_string(),
                schedule: "0 0 * * *".to_string(),
                enabled: true,
                last_run: Some(SystemTime::UNIX_EPOCH + Duration::from_secs(1704067200)),
                next_run: Some(SystemTime::UNIX_EPOCH + Duration::from_secs(1704153600)),
                ..Default::default()
            },
        ]));

    cron_list_command(&cron_service).await.unwrap();
}

#[tokio::test]
async fn test_cron_add_command_valid() {
    let cron_service = MockCronService::new();
    cron_service.expect_list_jobs().returning(|| Ok(vec![])); // No existing jobs
    cron_service.expect_add_job()
        .returning(|_| Ok("abc123".to_string()));

    let result = cron_add_command(
        &cron_service,
        "test-job".to_string(),
        "0 0 * * *".to_string(),
        "test prompt".to_string(),
    ).await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_cron_add_command_invalid_schedule() {
    let cron_service = MockCronService::new();

    let result = cron_add_command(
        &cron_service,
        "test-job".to_string(),
        "invalid".to_string(),
        "test prompt".to_string(),
    ).await;

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Invalid cron expression"));
}

#[tokio::test]
async fn test_cron_remove_command() {
    let cron_service = MockCronService::new();
    cron_service.expect_remove_job()
        .with("abc123")
        .returning(|_| Ok(()));

    cron_remove_command(&cron_service, "abc123".to_string()).await.unwrap();
}
```

### 5.3 Provider Login Tests (Manual)

```
Manual test procedure:
1. Set environment variables:
   export CODEX_CLIENT_ID="test-client-id"
   export CODEX_CLIENT_SECRET="test-client-secret"

2. Run: weft provider login codex

3. Verify browser opens to OAuth page

4. Log in with test account

5. Verify redirect to localhost:8765/callback

6. Verify "Login successful!" message in browser

7. Verify token stored in ~/.config/weft/config.yaml:
   providers:
     codex:
       api_key: "token-abc123"
       base_url: "https://api.openai.com/v1"

8. Test provider: weft chat --provider codex "Hello"
```

### 5.4 Markdown Conversion Tests

```rust
// tests/markdown_tests.rs
#[test]
fn test_telegram_markdown_bold() {
    let converter = TelegramMarkdownConverter;
    let result = converter.convert("**bold text**");
    assert_eq!(result, "<b>bold text</b>");
}

#[test]
fn test_telegram_markdown_italic() {
    let converter = TelegramMarkdownConverter;
    let result = converter.convert("*italic text*");
    assert_eq!(result, "<i>italic text</i>");
}

#[test]
fn test_telegram_markdown_code() {
    let converter = TelegramMarkdownConverter;
    let result = converter.convert("`code`");
    assert_eq!(result, "<code>code</code>");
}

#[test]
fn test_telegram_markdown_link() {
    let converter = TelegramMarkdownConverter;
    let result = converter.convert("[link](https://example.com)");
    assert_eq!(result, "<a href=\"https://example.com\">link</a>");
}

#[test]
fn test_telegram_markdown_html_escape() {
    let converter = TelegramMarkdownConverter;
    let result = converter.convert("<script>alert('xss')</script>");
    assert_eq!(result, "&lt;script&gt;alert('xss')&lt;/script&gt;");
}

#[test]
fn test_slack_markdown_bold() {
    let converter = SlackMarkdownConverter;
    let result = converter.convert("**bold text**");
    assert_eq!(result, "*bold text*");
}

#[test]
fn test_slack_markdown_italic() {
    let converter = SlackMarkdownConverter;
    let result = converter.convert("*italic text*");
    assert_eq!(result, "_italic text_");
}

#[test]
fn test_slack_markdown_link() {
    let converter = SlackMarkdownConverter;
    let result = converter.convert("[link](https://example.com)");
    assert_eq!(result, "<https://example.com|link>");
}

#[test]
fn test_discord_markdown_unchanged() {
    let converter = DiscordMarkdownConverter;
    let result = converter.convert("**bold** *italic* `code`");
    assert_eq!(result, "**bold** *italic* `code`");
}

#[test]
fn test_discord_markdown_spoiler() {
    let converter = DiscordMarkdownConverter;
    let result = converter.convert("||spoiler text||");
    assert_eq!(result, "||spoiler text||");
}
```

### 5.5 Test Coverage Requirements
- **Unit test coverage**: >80% for all CLI commands
- **Integration test coverage**: >60% for end-to-end flows
- **Critical paths**: 100% coverage for cron validation, OAuth flow, markdown escaping

---

## 6. Completion (Integration Checklist)

### 6.1 Pre-Integration Validation
- [x] All unit tests passing (>80% coverage)
- [ ] Integration tests passing (>60% coverage)
- [x] `weft channels status` tested with multiple channels
- [x] `weft cron list/add/remove/enable/run` tested with real cron jobs
- [ ] `weft provider login codex` manually tested (browser OAuth)
- [x] Markdown conversion tested for Telegram/Slack/Discord
- [x] HTML escaping tested for XSS prevention

### 6.2 CLI Argument Parsing (clap)
- [x] `weft channels status` command registered
- [x] `weft cron` subcommands registered (list, add, remove, enable, run)
- [ ] `weft provider login <provider>` command registered
- [x] Help text updated for all new commands
- [ ] Command aliases added (e.g., `weft cron ls` → `weft cron list`)

### 6.3 Table Display Integration
- [x] comfy-table dependency added to Cargo.toml
- [x] Table formatting tested with long column values
- [x] Table padding/alignment configured
- [x] Empty table handling (e.g., "No cron jobs configured")

### 6.4 Configuration Integration
- [ ] CronService injected into CLI commands
- [ ] ChannelManager injected into CLI commands
- [ ] Config loaded from `~/.config/weft/config.yaml`
- [ ] Token storage tested (0600 permissions)

### 6.5 Markdown Conversion Integration
- [x] MarkdownConverter trait implemented for 3 platforms
- [x] Telegram HTML conversion tested in clawft-channels/telegram
- [x] Slack mrkdwn conversion tested in clawft-channels/slack
- [x] Discord markdown conversion tested in clawft-channels/discord
- [x] pulldown-cmark dependency added to Cargo.toml

### 6.6 Error Handling Validation
- [x] Invalid cron expression returns helpful error
- [ ] Missing OAuth credentials returns error
- [ ] OAuth timeout (60s) returns error
- [x] Duplicate cron job name returns error
- [x] Channel not found returns error

### 6.7 Documentation
- [ ] README.md updated with new CLI commands
- [ ] `weft --help` output tested
- [ ] `weft cron --help` output tested
- [ ] `weft provider login --help` output tested
- [ ] Examples added for common workflows

### 6.8 User Experience
- [x] Success messages shown (e.g., "✓ Cron job created")
- [x] Error messages clear and actionable
- [ ] Progress indicators for OAuth flow ("Opening browser...")
- [x] Timestamps formatted consistently (YYYY-MM-DD HH:MM:SS)
- [x] Table columns aligned properly

### 6.9 Security Audit
- [ ] OAuth token stored securely (0600 permissions)
- [ ] HTML escaping prevents XSS in Telegram
- [ ] No secrets logged in debug output
- [ ] OAuth redirect URL validated (localhost only)

### 6.10 Final Review
- [ ] Code review by at least 2 reviewers
- [ ] Manual testing on Linux/macOS/Windows
- [ ] CLI responsiveness tested (<100ms for status commands)
- [ ] Changelog updated with new CLI features
- [ ] Version bumped (clawft-cli 0.2.0)

---

## Cross-Stream Integration Requirements

### Reuse Stream 1 Test Infrastructure
- **Import mocks from 1A**: `use clawft_platform::test_utils::{MockEnvironment, MockFileSystem};`
- **Use shared fixtures**: Load `tests/fixtures/config.json` for CLI config loading tests

### Integration Tests (Required)
```rust
#[tokio::test]
async fn test_channels_status_loads_real_config() {
    // Load config from shared fixture
    let config = load_config_from(fixtures_dir.join("config.json")).await.unwrap();
    // Run channels status command
    let output = run_cli_command(&["channels", "status"], &config).await;
    assert!(output.contains("telegram"));
}

#[tokio::test]
async fn test_cron_list_with_persisted_jobs() {
    // Load cron jobs from fixture
    let output = run_cli_command(&["cron", "list"], &config).await;
    assert!(output.contains("daily-summary"));
}
```

### Security Tests (Required)
- HTML escaping for Telegram output (prevent XSS via bot responses)
- OAuth callback URL validation (prevent open redirects)
- Config file permissions check (warn if world-readable)

### Coverage Target
- Unit test coverage: >= 80% (measured via `cargo-tarpaulin`)
- Critical paths (OAuth flow, markdown conversion, config loading): 100%

---

## Notes for Implementation Agent

1. **Read Python source files first** to understand exact CLI command implementations
2. **Use TDD London School**: Mock services, write failing tests, then implement
3. **Parallel file operations**: Create all test files + implementation files in single message
4. **Table display**: Use comfy-table for consistent formatting
5. **Timestamp formatting**: Use chrono for consistent format (YYYY-MM-DD HH:MM:SS)
6. **OAuth flow**: Handle timeout (60s), browser open, callback server
7. **Markdown parsing**: Use pulldown-cmark for robust parsing
8. **HTML escaping**: CRITICAL for Telegram to prevent XSS
9. **Cron validation**: Use `cron` crate's error messages
10. **Error messages**: User-friendly, actionable (e.g., "Invalid cron expression '...'")
11. **Success messages**: Clear, concise (e.g., "✓ Cron job created")
12. **Help text**: Comprehensive, with examples
13. **Command aliases**: Add common shortcuts (ls, rm, etc.)
14. **Logging**: Use tracing for debug logs, avoid logging secrets
15. **Config storage**: Use ~/.config/weft/config.yaml, ensure 0600 permissions
