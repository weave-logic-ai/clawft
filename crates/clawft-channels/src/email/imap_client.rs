//! Real IMAP backend (`imap` crate + native-tls).
//!
//! The `imap` crate is synchronous, so all network work runs inside
//! `tokio::task::spawn_blocking`. Each `fetch_unseen` call opens a
//! fresh connection (login → SELECT → UID SEARCH UNSEEN → UID FETCH
//! → UID STORE \Seen → logout). That makes reconnect-on-error trivial
//! and keeps session state out of the async runtime.

use async_trait::async_trait;
use tracing::{debug, warn};

use clawft_plugin::error::PluginError;

use super::transport::ImapBackend;
use super::types::{EmailAdapterConfig, ParsedEmail};

/// Production IMAP backend.
pub struct RealImapBackend {
    config: EmailAdapterConfig,
}

impl RealImapBackend {
    pub fn new(config: EmailAdapterConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl ImapBackend for RealImapBackend {
    async fn fetch_unseen(&self) -> Result<Vec<ParsedEmail>, PluginError> {
        let config = self.config.clone();
        tokio::task::spawn_blocking(move || fetch_unseen_blocking(&config))
            .await
            .map_err(|e| {
                PluginError::ExecutionFailed(format!("email imap: task join failed: {e}"))
            })?
    }
}

/// Blocking IMAP poll cycle.
fn fetch_unseen_blocking(config: &EmailAdapterConfig) -> Result<Vec<ParsedEmail>, PluginError> {
    let creds = config
        .auth
        .resolve_password()
        .map_err(PluginError::ExecutionFailed)?;

    let tls = native_tls::TlsConnector::builder().build().map_err(|e| {
        PluginError::ExecutionFailed(format!("email imap: tls connector: {e}"))
    })?;

    let domain = config.imap_host.as_str();
    let addr = (domain, config.imap_port);

    // Implicit TLS (port 993 default) when `imap_use_tls`; otherwise
    // attempt STARTTLS upgrade on a cleartext socket (port 143).
    let client = if config.imap_use_tls {
        imap::connect(addr, domain, &tls).map_err(|e| {
            PluginError::ExecutionFailed(format!(
                "email imap: connect {domain}:{} failed: {e}",
                config.imap_port
            ))
        })?
    } else {
        // STARTTLS path — still upgrades to TLS before AUTH.
        imap::connect_starttls(addr, domain, &tls).map_err(|e| {
            PluginError::ExecutionFailed(format!(
                "email imap: starttls {domain}:{} failed: {e}",
                config.imap_port
            ))
        })?
    };

    let mut session = client
        .login(creds.username.as_str(), creds.password.expose())
        .map_err(|e| {
            PluginError::ExecutionFailed(format!("email imap: login failed: {}", e.0))
        })?;

    session.select(&config.mailbox).map_err(|e| {
        PluginError::ExecutionFailed(format!(
            "email imap: SELECT {} failed: {e}",
            config.mailbox
        ))
    })?;

    let uids = session.uid_search("UNSEEN").map_err(|e| {
        PluginError::ExecutionFailed(format!("email imap: SEARCH UNSEEN failed: {e}"))
    })?;

    if uids.is_empty() {
        let _ = session.logout();
        return Ok(Vec::new());
    }

    // Stable order for deterministic delivery.
    let mut uid_list: Vec<u32> = uids.into_iter().collect();
    uid_list.sort_unstable();
    let uid_set = uid_list
        .iter()
        .map(|u: &u32| u.to_string())
        .collect::<Vec<_>>()
        .join(",");

    let fetches = session
        .uid_fetch(&uid_set, "BODY.PEEK[]")
        .map_err(|e| {
            PluginError::ExecutionFailed(format!("email imap: FETCH failed: {e}"))
        })?;

    let mut parsed = Vec::with_capacity(fetches.len());
    let mut seen_uids: Vec<u32> = Vec::new();

    for fetch in fetches.iter() {
        let uid = match fetch.uid {
            Some(u) => u,
            None => continue,
        };
        let bytes = match fetch.body() {
            Some(b) => b,
            None => {
                warn!(uid, "email imap: FETCH missing body, skipping");
                continue;
            }
        };
        match parse_rfc822(bytes, config.max_body_chars) {
            Ok(email) => {
                parsed.push(email);
                seen_uids.push(uid);
            }
            Err(e) => {
                warn!(uid, error = %e, "email imap: failed to parse RFC822");
            }
        }
    }

    // Mark successfully parsed messages as \Seen so the next poll does
    // not redeliver. Failures here are logged but not fatal — the
    // adapter also dedupes by Message-ID in memory.
    if !seen_uids.is_empty() {
        let store_set = seen_uids
            .iter()
            .map(|u| u.to_string())
            .collect::<Vec<_>>()
            .join(",");
        if let Err(e) = session.uid_store(&store_set, "+FLAGS (\\Seen)") {
            warn!(error = %e, "email imap: STORE \\Seen failed");
        }
    }

    if let Err(e) = session.logout() {
        debug!(error = %e, "email imap: logout error (ignored)");
    }

    Ok(parsed)
}

/// Parse an RFC822 byte blob into [`ParsedEmail`].
pub fn parse_rfc822(bytes: &[u8], max_body_chars: usize) -> Result<ParsedEmail, String> {
    let mail = mailparse::parse_mail(bytes).map_err(|e| format!("mailparse: {e}"))?;

    let get_header = |name: &str| -> String {
        mail.headers
            .iter()
            .find(|h| h.get_key().eq_ignore_ascii_case(name))
            .map(|h| h.get_value())
            .unwrap_or_default()
    };

    let from_raw = get_header("From");
    let to_raw = get_header("To");
    let subject = get_header("Subject");
    let message_id = {
        let mid = get_header("Message-ID");
        if mid.is_empty() {
            // Synthetic fallback so delivery metadata always has a key.
            format!(
                "<generated-{}@local>",
                chrono::Utc::now().timestamp_millis()
            )
        } else {
            mid
        }
    };
    let in_reply_to = {
        let v = get_header("In-Reply-To");
        if v.is_empty() {
            None
        } else {
            Some(v)
        }
    };

    let body_raw = extract_text_body(&mail);
    let body = if body_raw.chars().count() > max_body_chars {
        let truncated: String = body_raw.chars().take(max_body_chars).collect();
        format!("{truncated}\n\n[truncated]")
    } else {
        body_raw
    };

    Ok(ParsedEmail {
        from: extract_email_address(&from_raw),
        to: extract_email_address(&to_raw),
        subject,
        body,
        message_id,
        in_reply_to,
    })
}

/// Prefer text/plain parts; fall back to text/html stripped lightly,
/// then the top-level body.
fn extract_text_body(mail: &mailparse::ParsedMail<'_>) -> String {
    // Multipart: walk subparts for text/plain first.
    if !mail.subparts.is_empty() {
        for part in &mail.subparts {
            let ct = part.ctype.mimetype.to_ascii_lowercase();
            if ct == "text/plain"
                && let Ok(body) = part.get_body()
            {
                return body;
            }
        }
        for part in &mail.subparts {
            let ct = part.ctype.mimetype.to_ascii_lowercase();
            if ct == "text/html"
                && let Ok(body) = part.get_body()
            {
                return strip_simple_html(&body);
            }
            // Nested multiparts (e.g. multipart/alternative inside mixed).
            if ct.starts_with("multipart/") {
                let nested = extract_text_body(part);
                if !nested.is_empty() {
                    return nested;
                }
            }
        }
    }

    mail.get_body().unwrap_or_default()
}

fn strip_simple_html(html: &str) -> String {
    // Intentionally minimal — not a full HTML parser. Strips tags so the
    // agent pipeline gets something readable when only HTML is present.
    let mut out = String::with_capacity(html.len());
    let mut in_tag = false;
    for ch in html.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(ch),
            _ => {}
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Extract `user@host` from `Display Name <user@host>` or bare addresses.
pub fn extract_email_address(raw: &str) -> String {
    let trimmed = raw.trim();
    if let Some(start) = trimmed.find('<')
        && let Some(end) = trimmed.find('>')
        && end > start
    {
        return trimmed[start + 1..end].trim().to_string();
    }
    // Bare address — take the first whitespace-delimited token.
    trimmed
        .split_whitespace()
        .next()
        .unwrap_or(trimmed)
        .trim_matches(|c| c == '"' || c == '\'')
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_angle_bracket_address() {
        assert_eq!(
            extract_email_address("Alice <alice@example.com>"),
            "alice@example.com"
        );
    }

    #[test]
    fn extract_bare_address() {
        assert_eq!(
            extract_email_address("bob@example.com"),
            "bob@example.com"
        );
    }

    #[test]
    fn parse_simple_rfc822() {
        let raw = b"From: Alice <alice@example.com>\r\n\
                    To: bot@test.com\r\n\
                    Subject: Hello\r\n\
                    Message-ID: <abc@example.com>\r\n\
                    In-Reply-To: <prev@example.com>\r\n\
                    Content-Type: text/plain; charset=utf-8\r\n\
                    \r\n\
                    Body line one\r\n\
                    Body line two\r\n";
        let email = parse_rfc822(raw, 12000).unwrap();
        assert_eq!(email.from, "alice@example.com");
        assert_eq!(email.to, "bot@test.com");
        assert_eq!(email.subject, "Hello");
        assert_eq!(email.message_id, "<abc@example.com>");
        assert_eq!(email.in_reply_to.as_deref(), Some("<prev@example.com>"));
        assert!(email.body.contains("Body line one"));
    }

    #[test]
    fn parse_truncates_body() {
        let body = "X".repeat(100);
        let raw = format!(
            "From: a@b.com\r\nTo: c@d.com\r\nSubject: t\r\n\
             Message-ID: <m@x>\r\nContent-Type: text/plain\r\n\r\n{body}\r\n"
        );
        let email = parse_rfc822(raw.as_bytes(), 10).unwrap();
        assert!(email.body.contains("[truncated]"));
        assert!(email.body.chars().count() < 40);
    }
}
