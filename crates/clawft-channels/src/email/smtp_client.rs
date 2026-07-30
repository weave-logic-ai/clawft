//! Real SMTP backend (`lettre` async + rustls).
//!
//! Builds a MIME message with a client-generated Message-ID and submits
//! it via STARTTLS (default port 587) or implicit TLS (port 465 / when
//! `smtp_use_tls` is true and the port is 465).

use async_trait::async_trait;
use lettre::message::{header::ContentType, Mailbox, Message};
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Tokio1Executor};
use tracing::info;

use clawft_plugin::error::PluginError;

use super::transport::SmtpBackend;
use super::types::EmailAdapterConfig;

/// Production SMTP backend.
pub struct RealSmtpBackend {
    config: EmailAdapterConfig,
}

impl RealSmtpBackend {
    pub fn new(config: EmailAdapterConfig) -> Self {
        Self { config }
    }

    fn domain_part(&self) -> &str {
        self.config
            .email_address
            .split('@')
            .nth(1)
            .unwrap_or("localhost")
    }

    fn generate_message_id(&self) -> String {
        format!(
            "<{}@{}>",
            uuid::Uuid::new_v4(),
            self.domain_part()
        )
    }

    async fn build_transport(
        &self,
    ) -> Result<AsyncSmtpTransport<Tokio1Executor>, PluginError> {
        let creds = self
            .config
            .auth
            .resolve_password()
            .map_err(PluginError::ExecutionFailed)?;
        let credentials = Credentials::new(
            creds.username.clone(),
            creds.password.expose().to_string(),
        );

        let host = self.config.smtp_host.as_str();
        let port = self.config.smtp_port;

        // Port 465 → implicit TLS. Otherwise STARTTLS when smtp_use_tls
        // (default true). Cleartext is only allowed when smtp_use_tls is
        // false (dev / mock harnesses).
        let builder = if !self.config.smtp_use_tls {
            AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(host).port(port)
        } else if port == 465 {
            AsyncSmtpTransport::<Tokio1Executor>::relay(host)
                .map_err(|e| {
                    PluginError::ExecutionFailed(format!(
                        "email smtp: implicit-tls relay setup failed: {e}"
                    ))
                })?
                .port(port)
        } else {
            AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(host)
                .map_err(|e| {
                    PluginError::ExecutionFailed(format!(
                        "email smtp: starttls relay setup failed: {e}"
                    ))
                })?
                .port(port)
        };

        Ok(builder.credentials(credentials).build())
    }
}

#[async_trait]
impl SmtpBackend for RealSmtpBackend {
    async fn send(
        &self,
        to: &str,
        subject: &str,
        body: &str,
        in_reply_to: Option<&str>,
    ) -> Result<String, PluginError> {
        if self.config.smtp_host.is_empty() {
            return Err(PluginError::ExecutionFailed(
                "email adapter: smtp_host is not configured".into(),
            ));
        }
        if self.config.email_address.is_empty() {
            return Err(PluginError::ExecutionFailed(
                "email adapter: email_address is required for SMTP send".into(),
            ));
        }

        let from: Mailbox = self.config.email_address.parse().map_err(|e| {
            PluginError::ExecutionFailed(format!(
                "email smtp: invalid from address `{}`: {e}",
                self.config.email_address
            ))
        })?;
        let to_mb: Mailbox = to.parse().map_err(|e| {
            PluginError::ExecutionFailed(format!("email smtp: invalid to address `{to}`: {e}"))
        })?;

        let message_id = self.generate_message_id();

        let mut builder = Message::builder()
            .from(from)
            .to(to_mb)
            .subject(subject)
            .message_id(Some(message_id.clone()));

        if let Some(irt) = in_reply_to {
            // Textual headers implement From<String>.
            let irt = irt.trim().to_string();
            builder = builder.header(lettre::message::header::InReplyTo::from(irt.clone()));
            builder = builder.header(lettre::message::header::References::from(irt));
        }

        let email = builder
            .header(ContentType::TEXT_PLAIN)
            .body(body.to_string())
            .map_err(|e| {
                PluginError::ExecutionFailed(format!("email smtp: MIME build failed: {e}"))
            })?;

        let transport = self.build_transport().await?;
        let response = transport.send(email).await.map_err(|e| {
            PluginError::ExecutionFailed(format!(
                "email smtp: send to `{to}` via {}:{} failed: {e}",
                self.config.smtp_host, self.config.smtp_port
            ))
        })?;

        info!(
            to = %to,
            smtp_host = %self.config.smtp_host,
            message_id = %message_id,
            code = ?response.code(),
            "email smtp: message submitted"
        );

        Ok(message_id)
    }
}
