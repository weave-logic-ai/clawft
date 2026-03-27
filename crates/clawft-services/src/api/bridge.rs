//! Bridge implementations connecting core services to API trait objects.
//!
//! These structs implement the `*Access` traits defined in `api/mod.rs`
//! by wrapping Arc references to real core services. The Platform generic
//! is erased at this boundary so the API layer never needs to know `P`.

use std::sync::Arc;

use clawft_core::bus::MessageBus;
use clawft_core::session::SessionManager;
use clawft_core::tools::registry::ToolRegistry;
use clawft_platform::Platform;
use tracing::warn;

use super::{
    AgentAccess, AgentInfo, BusAccess, ChannelAccess, ChannelStatusInfo, ConfigAccess,
    MemoryAccess, MemoryEntryInfo, SessionAccess, SessionDetail, SessionInfo, SkillAccess,
    SkillInfo, ToolInfo, ToolRegistryAccess, TtsProviderInfo, VoiceAccess, VoiceSettingsInfo,
    VoiceSettingsUpdate, VoiceStatusInfo,
};

// ---------------------------------------------------------------------------
// ToolBridge
// ---------------------------------------------------------------------------

/// Wraps an `Arc<ToolRegistry>` and implements [`ToolRegistryAccess`].
///
/// Since `ToolRegistry` is immutable after registration, sharing via
/// `Arc` is safe without any locking.
pub struct ToolBridge {
    registry: Arc<ToolRegistry>,
}

impl ToolBridge {
    /// Create a new bridge from a shared tool registry.
    pub fn new(registry: Arc<ToolRegistry>) -> Self {
        Self { registry }
    }
}

impl ToolRegistryAccess for ToolBridge {
    fn list_tools(&self) -> Vec<ToolInfo> {
        self.registry
            .list()
            .into_iter()
            .map(|name| {
                let description = self
                    .registry
                    .get(&name)
                    .map(|t| t.description().to_string())
                    .unwrap_or_default();
                ToolInfo { name, description }
            })
            .collect()
    }

    fn tool_schema(&self, name: &str) -> Option<serde_json::Value> {
        self.registry.get(name).map(|t| t.parameters())
    }
}

// ---------------------------------------------------------------------------
// SessionBridge
// ---------------------------------------------------------------------------

/// Wraps an `Arc<SessionManager<P>>` and implements [`SessionAccess`].
///
/// The `SessionManager` methods are async, but the `SessionAccess` trait
/// is sync (for Axum handler ergonomics). We bridge this gap using
/// `tokio::task::block_in_place` + `Handle::current().block_on()`.
pub struct SessionBridge<P: Platform> {
    manager: Arc<SessionManager<P>>,
}

impl<P: Platform> SessionBridge<P> {
    /// Create a new bridge from a shared session manager.
    pub fn new(manager: Arc<SessionManager<P>>) -> Self {
        Self { manager }
    }
}

impl<P: Platform + 'static> SessionAccess for SessionBridge<P> {
    fn list_sessions(&self) -> Vec<SessionInfo> {
        let mgr = self.manager.clone();
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                let keys = match mgr.list_sessions().await {
                    Ok(keys) => keys,
                    Err(e) => {
                        warn!(error = %e, "failed to list sessions");
                        return vec![];
                    }
                };
                let mut infos = Vec::with_capacity(keys.len());
                for key in keys {
                    match mgr.load_session(&key).await {
                        Ok(session) => {
                            infos.push(SessionInfo {
                                key: session.key,
                                message_count: session.messages.len(),
                                created_at: Some(session.created_at.to_rfc3339()),
                                updated_at: Some(session.updated_at.to_rfc3339()),
                            });
                        }
                        Err(e) => {
                            warn!(key = %key, error = %e, "failed to load session metadata");
                            infos.push(SessionInfo {
                                key,
                                message_count: 0,
                                created_at: None,
                                updated_at: None,
                            });
                        }
                    }
                }
                infos
            })
        })
    }

    fn get_session(&self, key: &str) -> Option<SessionDetail> {
        let mgr = self.manager.clone();
        let key = key.to_string();
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                match mgr.load_session(&key).await {
                    Ok(session) => Some(SessionDetail {
                        key: session.key,
                        messages: session.messages,
                    }),
                    Err(_) => None,
                }
            })
        })
    }

    fn delete_session(&self, key: &str) -> bool {
        let mgr = self.manager.clone();
        let key = key.to_string();
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                mgr.delete_session(&key).await.is_ok()
            })
        })
    }
}

// ---------------------------------------------------------------------------
// AgentBridge
// ---------------------------------------------------------------------------

/// Holds a snapshot of agent definitions and implements [`AgentAccess`].
///
/// Built once at startup from the `AgentRegistry` discovery results.
/// The list is immutable after construction.
pub struct AgentBridge {
    agents: Vec<AgentInfo>,
}

impl AgentBridge {
    /// Create a new bridge from a pre-built list of agent infos.
    pub fn new(agents: Vec<AgentInfo>) -> Self {
        Self { agents }
    }

    /// Create an empty bridge (no agents discovered).
    pub fn empty() -> Self {
        Self { agents: vec![] }
    }
}

impl AgentAccess for AgentBridge {
    fn list_agents(&self) -> Vec<AgentInfo> {
        self.agents.clone()
    }

    fn get_agent(&self, name: &str) -> Option<AgentInfo> {
        self.agents.iter().find(|a| a.name == name).cloned()
    }
}

// ---------------------------------------------------------------------------
// BusBridge
// ---------------------------------------------------------------------------

/// Wraps an `Arc<MessageBus>` and implements [`BusAccess`].
///
/// Publishes inbound messages to the bus so they are picked up by the
/// agent loop. Uses `publish_inbound` (sync try_send) which is safe
/// to call from sync context.
pub struct BusBridge {
    bus: Arc<MessageBus>,
}

impl BusBridge {
    /// Create a new bridge from a shared message bus.
    pub fn new(bus: Arc<MessageBus>) -> Self {
        Self { bus }
    }
}

impl BusAccess for BusBridge {
    fn send_message(&self, channel: &str, chat_id: &str, content: &str) {
        use clawft_types::event::InboundMessage;
        use std::collections::HashMap;

        let msg = InboundMessage {
            channel: channel.to_string(),
            sender_id: "api".to_string(),
            chat_id: chat_id.to_string(),
            content: content.to_string(),
            timestamp: chrono::Utc::now(),
            media: vec![],
            metadata: HashMap::new(),
        };

        if let Err(e) = self.bus.publish_inbound(msg) {
            warn!(error = %e, "failed to publish inbound message from API");
        }
    }
}

// ---------------------------------------------------------------------------
// SkillBridge
// ---------------------------------------------------------------------------

/// Wraps an `Arc<SkillsLoader<P>>` and implements [`SkillAccess`].
///
/// Lists skills by calling the async `list_skills()` + `load_skill()` methods
/// via `block_in_place` + `block_on`, matching the pattern used by SessionBridge.
pub struct SkillBridge<P: Platform> {
    loader: Arc<clawft_core::agent::skills::SkillsLoader<P>>,
}

impl<P: Platform> SkillBridge<P> {
    /// Create a new bridge from a shared skills loader.
    pub fn new(loader: Arc<clawft_core::agent::skills::SkillsLoader<P>>) -> Self {
        Self { loader }
    }
}

impl<P: Platform + 'static> SkillAccess for SkillBridge<P> {
    fn list_skills(&self) -> Vec<SkillInfo> {
        let loader = self.loader.clone();
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                let names = match loader.list_skills().await {
                    Ok(names) => names,
                    Err(e) => {
                        warn!(error = %e, "failed to list skills");
                        return vec![];
                    }
                };
                let mut infos = Vec::with_capacity(names.len());
                for name in names {
                    match loader.load_skill(&name).await {
                        Ok(skill) => {
                            infos.push(SkillInfo {
                                name: skill.name,
                                description: skill.description,
                                version: skill.version,
                                user_invocable: skill.user_invocable,
                            });
                        }
                        Err(e) => {
                            warn!(skill = %name, error = %e, "failed to load skill");
                            infos.push(SkillInfo {
                                name,
                                description: String::new(),
                                version: String::new(),
                                user_invocable: false,
                            });
                        }
                    }
                }
                infos
            })
        })
    }

    fn install_skill(&self, _id: &str) -> Result<(), String> {
        // TODO: implement skill installation via ClawHub registry
        Err("not implemented".into())
    }

    fn uninstall_skill(&self, _name: &str) -> Result<(), String> {
        // TODO: implement skill uninstallation
        Err("not implemented".into())
    }
}

// ---------------------------------------------------------------------------
// MemoryBridge
// ---------------------------------------------------------------------------

/// Wraps an `Arc<MemoryStore<P>>` and implements [`MemoryAccess`].
///
/// Memory is stored as markdown files. The search method delegates to
/// `MemoryStore::search()`.
pub struct MemoryBridge<P: Platform> {
    store: Arc<clawft_core::agent::memory::MemoryStore<P>>,
}

impl<P: Platform> MemoryBridge<P> {
    /// Create a new bridge from a shared memory store.
    pub fn new(store: Arc<clawft_core::agent::memory::MemoryStore<P>>) -> Self {
        Self { store }
    }
}

impl<P: Platform + 'static> MemoryAccess for MemoryBridge<P> {
    fn list_entries(&self) -> Vec<MemoryEntryInfo> {
        // Memory is stored as paragraphs in markdown files.
        // Return the content split into paragraph entries.
        let store = self.store.clone();
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                let mut entries = Vec::new();
                let content = match store.read_long_term().await {
                    Ok(c) => c,
                    Err(e) => {
                        warn!(error = %e, "failed to read long-term memory");
                        return entries;
                    }
                };
                for (i, paragraph) in content.split("\n\n").enumerate() {
                    let trimmed = paragraph.trim();
                    if !trimmed.is_empty() {
                        entries.push(MemoryEntryInfo {
                            key: format!("memory:{i}"),
                            value: trimmed.to_string(),
                            namespace: "long_term".into(),
                            tags: Vec::new(),
                            updated_at: String::new(),
                            similarity: None,
                        });
                    }
                }
                entries
            })
        })
    }

    fn search(&self, query: &str, max_results: usize) -> Vec<MemoryEntryInfo> {
        let store = self.store.clone();
        let query = query.to_string();
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                let results = store.search(&query, max_results).await;
                results
                    .into_iter()
                    .enumerate()
                    .map(|(i, value)| MemoryEntryInfo {
                        key: format!("search:{i}"),
                        value,
                        namespace: "search".into(),
                        tags: Vec::new(),
                        updated_at: String::new(),
                        similarity: None,
                    })
                    .collect()
            })
        })
    }

    fn store(
        &self,
        _key: &str,
        value: &str,
        _namespace: &str,
        _tags: &[String],
    ) -> Result<MemoryEntryInfo, String> {
        // Append to long-term memory
        let store = self.store.clone();
        let value = value.to_string();
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                store
                    .append_long_term(&value)
                    .await
                    .map_err(|e| e.to_string())?;
                Ok(MemoryEntryInfo {
                    key: format!("memory:{}", uuid::Uuid::new_v4()),
                    value,
                    namespace: "long_term".into(),
                    tags: Vec::new(),
                    updated_at: chrono::Utc::now().to_rfc3339(),
                    similarity: None,
                })
            })
        })
    }

    fn delete(&self, _key: &str) -> bool {
        // TODO: implement memory entry deletion
        // Memory files are append-only; deletion requires rewriting.
        false
    }
}

// ---------------------------------------------------------------------------
// ConfigBridge
// ---------------------------------------------------------------------------

/// Holds a snapshot of the configuration and implements [`ConfigAccess`].
///
/// For `get_config()`, serializes the Config to JSON. SecretString fields
/// auto-serialize as empty strings for safety.
/// For `save_config()`, currently returns an error (read-only for now).
pub struct ConfigBridge {
    config: clawft_types::config::Config,
}

impl ConfigBridge {
    /// Create a new bridge from a cloned Config.
    pub fn new(config: clawft_types::config::Config) -> Self {
        Self { config }
    }
}

impl ConfigAccess for ConfigBridge {
    fn get_config(&self) -> serde_json::Value {
        // Build a simplified config view matching the UI's ConfigData type.
        serde_json::json!({
            "agents": {
                "defaults": {
                    "model": self.config.agents.defaults.model,
                    "max_tokens": self.config.agents.defaults.max_tokens,
                    "temperature": self.config.agents.defaults.temperature,
                }
            },
            "providers": {
                "anthropic": {
                    "api_key_set": !self.config.providers.anthropic.api_key.is_empty(),
                    "api_base": self.config.providers.anthropic.api_base.clone().unwrap_or_default(),
                    "enabled": !self.config.providers.anthropic.api_key.is_empty(),
                },
                "openai": {
                    "api_key_set": !self.config.providers.openai.api_key.is_empty(),
                    "api_base": self.config.providers.openai.api_base.clone().unwrap_or_default(),
                    "enabled": !self.config.providers.openai.api_key.is_empty(),
                },
                "deepseek": {
                    "api_key_set": !self.config.providers.deepseek.api_key.is_empty(),
                    "api_base": self.config.providers.deepseek.api_base.clone().unwrap_or_default(),
                    "enabled": !self.config.providers.deepseek.api_key.is_empty(),
                },
                "openrouter": {
                    "api_key_set": !self.config.providers.openrouter.api_key.is_empty(),
                    "api_base": self.config.providers.openrouter.api_base.clone().unwrap_or_default(),
                    "enabled": !self.config.providers.openrouter.api_key.is_empty(),
                },
            },
            "channels": {
                "telegram": { "enabled": self.config.channels.telegram.enabled },
                "slack": { "enabled": self.config.channels.slack.enabled },
                "discord": { "enabled": self.config.channels.discord.enabled },
            },
            "gateway": {
                "api_port": self.config.gateway.api_port,
                "api_enabled": self.config.gateway.api_enabled,
            },
        })
    }

    fn save_config(&self, _config: serde_json::Value) -> Result<(), String> {
        // TODO: implement config persistence (deserialize, validate, write to file)
        Err("config saving not yet implemented".into())
    }
}

// ---------------------------------------------------------------------------
// ChannelBridge
// ---------------------------------------------------------------------------

/// Holds a snapshot of channel statuses and implements [`ChannelAccess`].
///
/// Built from the `ChannelsConfig` at startup.
pub struct ChannelBridge {
    channels: Vec<ChannelStatusInfo>,
}

impl ChannelBridge {
    /// Create a new bridge from the channels config.
    pub fn from_config(channels: &clawft_types::config::ChannelsConfig, api_enabled: bool) -> Self {
        let mut list = Vec::new();

        list.push(ChannelStatusInfo {
            name: "telegram".into(),
            channel_type: "telegram".into(),
            status: if channels.telegram.enabled {
                "connected".into()
            } else {
                "disconnected".into()
            },
            message_count: 0,
            last_activity: None,
            routes_to: None,
        });

        list.push(ChannelStatusInfo {
            name: "slack".into(),
            channel_type: "slack".into(),
            status: if channels.slack.enabled {
                "connected".into()
            } else {
                "disconnected".into()
            },
            message_count: 0,
            last_activity: None,
            routes_to: None,
        });

        list.push(ChannelStatusInfo {
            name: "discord".into(),
            channel_type: "discord".into(),
            status: if channels.discord.enabled {
                "connected".into()
            } else {
                "disconnected".into()
            },
            message_count: 0,
            last_activity: None,
            routes_to: None,
        });

        if api_enabled {
            list.push(ChannelStatusInfo {
                name: "web".into(),
                channel_type: "web".into(),
                status: "connected".into(),
                message_count: 0,
                last_activity: None,
                routes_to: None,
            });
        }

        Self { channels: list }
    }
}

impl ChannelAccess for ChannelBridge {
    fn list_channels(&self) -> Vec<ChannelStatusInfo> {
        self.channels.clone()
    }
}

// ---------------------------------------------------------------------------
// VoiceBridge
// ---------------------------------------------------------------------------

/// Holds voice configuration and runtime state, implements [`VoiceAccess`].
///
/// Voice settings are read from `VoiceConfig` at startup and can be updated
/// via the API. Runtime state (idle/listening/speaking) is tracked separately.
/// Uses `std::sync::RwLock` since the lock is held very briefly.
pub struct VoiceBridge {
    config: std::sync::RwLock<clawft_types::config::voice::VoiceConfig>,
    /// UI-specific settings not in VoiceConfig.
    ui_settings: std::sync::RwLock<UiVoiceSettings>,
    /// Provider credentials for cloud TTS.
    providers: clawft_types::config::ProvidersConfig,
}

/// Extra voice settings the UI tracks that aren't in VoiceConfig.
struct UiVoiceSettings {
    echo_cancel: bool,
    noise_suppression: bool,
    push_to_talk: bool,
}

impl VoiceBridge {
    /// Create a new bridge from the voice config and provider credentials.
    pub fn new(
        config: clawft_types::config::voice::VoiceConfig,
        providers: clawft_types::config::ProvidersConfig,
    ) -> Self {
        Self {
            config: std::sync::RwLock::new(config),
            ui_settings: std::sync::RwLock::new(UiVoiceSettings {
                echo_cancel: true,
                noise_suppression: true,
                push_to_talk: false,
            }),
            providers,
        }
    }
}

impl VoiceAccess for VoiceBridge {
    fn get_status(&self) -> VoiceStatusInfo {
        let cfg = self.config.read().unwrap();
        VoiceStatusInfo {
            state: "idle".into(),
            talk_mode_active: false,
            wake_word_enabled: cfg.wake.enabled,
        }
    }

    fn get_settings(&self) -> VoiceSettingsInfo {
        let cfg = self.config.read().unwrap();
        let ui = self.ui_settings.read().unwrap();
        let language = if cfg.stt.language.is_empty() {
            "en-US".to_string()
        } else {
            cfg.stt.language.clone()
        };
        VoiceSettingsInfo {
            enabled: cfg.enabled,
            wake_word_enabled: cfg.wake.enabled,
            language,
            echo_cancel: ui.echo_cancel,
            noise_suppression: ui.noise_suppression,
            push_to_talk: ui.push_to_talk,
        }
    }

    fn update_settings(&self, update: VoiceSettingsUpdate) -> Result<(), String> {
        let mut cfg = self.config.write().map_err(|e| e.to_string())?;
        let mut ui = self.ui_settings.write().map_err(|e| e.to_string())?;

        if let Some(enabled) = update.enabled {
            cfg.enabled = enabled;
        }
        if let Some(wake) = update.wake_word_enabled {
            cfg.wake.enabled = wake;
        }
        if let Some(lang) = update.language {
            cfg.stt.language = lang;
        }
        if let Some(echo) = update.echo_cancel {
            ui.echo_cancel = echo;
        }
        if let Some(noise) = update.noise_suppression {
            ui.noise_suppression = noise;
        }
        if let Some(ptt) = update.push_to_talk {
            ui.push_to_talk = ptt;
        }
        Ok(())
    }

    fn get_tts_config(&self) -> TtsProviderInfo {
        let cfg = self.config.read().unwrap();
        let provider = cfg.tts.provider.clone();

        // Resolve API key: config value > env var fallback.
        let (api_key, api_base) = match provider.as_str() {
            "openai" => {
                let key = resolve_secret(
                    &self.providers.openai.api_key,
                    "OPENAI_API_KEY",
                );
                (key, self.providers.openai.api_base.clone())
            }
            "elevenlabs" => {
                let key = resolve_secret(
                    &self.providers.elevenlabs.api_key,
                    "ELEVENLABS_API_KEY",
                );
                let base = self
                    .providers
                    .elevenlabs
                    .api_base
                    .clone()
                    .or_else(|| Some("https://api.elevenlabs.io".to_string()));
                (key, base)
            }
            _ => (String::new(), None),
        };

        // Default voice and model per provider when not explicitly set.
        let default_model = cfg.tts.model == "vits-piper-en_US-amy-medium";
        let (model, voice) = match provider.as_str() {
            "openai" => (
                if default_model { "tts-1".to_string() } else { cfg.tts.model.clone() },
                if cfg.tts.voice.is_empty() { "alloy".to_string() } else { cfg.tts.voice.clone() },
            ),
            "elevenlabs" => (
                if default_model { "eleven_multilingual_v2".to_string() } else { cfg.tts.model.clone() },
                if cfg.tts.voice.is_empty() { "Rachel".to_string() } else { cfg.tts.voice.clone() },
            ),
            _ => (cfg.tts.model.clone(), cfg.tts.voice.clone()),
        };

        TtsProviderInfo {
            provider,
            model,
            voice,
            speed: cfg.tts.speed,
            api_key,
            api_base,
        }
    }
}

/// Resolve a secret: use the config value if non-empty, else try an env var.
fn resolve_secret(
    secret: &clawft_types::secret::SecretString,
    env_var: &str,
) -> String {
    if !secret.is_empty() {
        return secret.expose().to_string();
    }
    std::env::var(env_var).unwrap_or_default()
}
