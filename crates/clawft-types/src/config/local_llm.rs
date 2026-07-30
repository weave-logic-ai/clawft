//! WEFT-604 — single source of truth for the local LLM endpoint + model.
//!
//! ADR-060 pins the production local loop to Hermes served via
//! `~/llm/bin/serve-llamacpp` on port **8090** with alias **hermes-4.3-36b**.
//!
//! Every consumer (daemon `[kernel.llm]` defaults, `weft agent` provider
//! registry, voice `LocalProvider::hermes_serving`) must derive its
//! zero-config defaults from these constants so a freshly-served Hermes
//! is discovered with no machine-local edits.
//!
//! # URL shapes
//!
//! | Constant | Shape | Used by |
//! |---|---|---|
//! | [`DEFAULT_LOCAL_LLM_SERVICE_URL`] | `http://host:port` (no `/v1`) | `clawft-service-llm::LlmClient` (appends `/v1/chat/completions`) |
//! | [`DEFAULT_LOCAL_LLM_API_BASE`] | `http://host:port/v1` | `clawft-llm` providers (`…/chat/completions`) |
//!
//! Use [`service_url_to_api_base`] / [`api_base_to_service_url`] when
//! bridging between the two shapes.

/// ADR-060 local Hermes OpenAI root (no `/v1` suffix).
///
/// Consumed by `clawft-service-llm` and the daemon's LLM resolver.
pub const DEFAULT_LOCAL_LLM_SERVICE_URL: &str = "http://127.0.0.1:8090";

/// ADR-060 local Hermes OpenAI-compat API base (with `/v1`).
///
/// Consumed by `clawft-llm` `LocalProvider` / builtin `local` provider.
pub const DEFAULT_LOCAL_LLM_API_BASE: &str = "http://127.0.0.1:8090/v1";

/// ADR-060 default model alias (`serve-llamacpp --alias`).
pub const DEFAULT_LOCAL_LLM_MODEL: &str = "hermes-4.3-36b";

/// Default `agents.defaults.model` routing string.
///
/// The `local/` prefix selects the keyless local provider so the CLI agent
/// path does not demand `OPENAI_API_KEY` / `DEEPSEEK_API_KEY` for the
/// zero-config Hermes case.
pub const DEFAULT_LOCAL_LLM_MODEL_ROUTED: &str = "local/hermes-4.3-36b";

/// Environment variable for the service-url override (daemon + service-llm).
pub const LLM_SERVICE_URL_ENV: &str = "LLM_SERVICE_URL";

/// Environment variable for the model-name override (daemon + service-llm).
pub const LLM_MODEL_ENV: &str = "LLM_MODEL";

/// Convert a service URL (`http://host:port` or with trailing `/v1`) into
/// the OpenAI-compat API base expected by `clawft-llm` providers.
pub fn service_url_to_api_base(url: &str) -> String {
    let u = url.trim().trim_end_matches('/');
    if u.ends_with("/v1") {
        u.to_string()
    } else {
        format!("{u}/v1")
    }
}

/// Convert an OpenAI-compat API base into the service-llm root URL
/// (strip a trailing `/v1` when present).
pub fn api_base_to_service_url(base: &str) -> String {
    let u = base.trim().trim_end_matches('/');
    u.strip_suffix("/v1").unwrap_or(u).to_string()
}

/// True when `model` is the routed or bare ADR-060 Hermes default.
pub fn is_default_local_model(model: &str) -> bool {
    model == DEFAULT_LOCAL_LLM_MODEL
        || model == DEFAULT_LOCAL_LLM_MODEL_ROUTED
        || model == format!("local/{DEFAULT_LOCAL_LLM_MODEL}")
        || model.strip_prefix("local/") == Some(DEFAULT_LOCAL_LLM_MODEL)
}

/// True when `model` is a legacy cloud default that should yield to a
/// configured local endpoint under the WEFT-604 bridge.
pub fn is_legacy_cloud_default_model(model: &str) -> bool {
    matches!(
        model,
        "deepseek/deepseek-chat" | "openai/gpt-4o" | "gpt-4o"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn service_url_and_api_base_round_trip() {
        assert_eq!(
            service_url_to_api_base(DEFAULT_LOCAL_LLM_SERVICE_URL),
            DEFAULT_LOCAL_LLM_API_BASE
        );
        assert_eq!(
            api_base_to_service_url(DEFAULT_LOCAL_LLM_API_BASE),
            DEFAULT_LOCAL_LLM_SERVICE_URL
        );
        assert_eq!(
            service_url_to_api_base("http://127.0.0.1:8090/v1"),
            "http://127.0.0.1:8090/v1"
        );
        assert_eq!(
            api_base_to_service_url("http://127.0.0.1:8090"),
            "http://127.0.0.1:8090"
        );
    }

    #[test]
    fn default_model_helpers() {
        assert!(is_default_local_model(DEFAULT_LOCAL_LLM_MODEL));
        assert!(is_default_local_model(DEFAULT_LOCAL_LLM_MODEL_ROUTED));
        assert!(is_legacy_cloud_default_model("deepseek/deepseek-chat"));
        assert!(!is_legacy_cloud_default_model(DEFAULT_LOCAL_LLM_MODEL_ROUTED));
    }
}
