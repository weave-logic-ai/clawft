//! Sandboxed HTTP client backed by [`PluginSandbox`] validation.
//!
//! Host-side wrapper (WEFT-398): previously lived in `clawft-wasm::http`.

use std::collections::HashMap;
use std::sync::Arc;

use crate::sandbox::{self, HttpValidationError, PluginSandbox};

/// HTTP response from a sandboxed request (shape matches the WASI stub).
#[derive(Debug, Clone)]
pub struct HttpResponse {
    /// HTTP status code (e.g., 200, 404, 500).
    pub status: u16,
    /// Response headers as key-value pairs.
    pub headers: HashMap<String, String>,
    /// Raw response body bytes.
    pub body: Vec<u8>,
}

/// Sandboxed HTTP client that validates all requests against a plugin's
/// network permissions before executing them.
///
/// Note: Actual HTTP execution requires a runtime (e.g., reqwest + tokio).
/// This struct currently validates the request and returns an error for
/// the actual network call. When wasmtime integration is wired (C2.8+),
/// this will be connected to a real HTTP client.
pub struct SandboxedHttpClient {
    /// The plugin sandbox that governs all access decisions.
    pub sandbox: Arc<PluginSandbox>,
}

impl SandboxedHttpClient {
    /// Create a new sandboxed HTTP client for a plugin.
    pub fn new(sandbox: Arc<PluginSandbox>) -> Self {
        Self { sandbox }
    }

    /// Validate an HTTP request against the plugin's sandbox permissions.
    ///
    /// Returns the validated URL if the request is permitted.
    /// This does NOT execute the request -- it only performs security validation.
    pub fn validate_request(
        &self,
        method: &str,
        url: &str,
        body: Option<&str>,
    ) -> Result<url::Url, HttpValidationError> {
        sandbox::validate_http_request(&self.sandbox, method, url, body)
    }

    /// Send an HTTP request (validation + stub execution).
    ///
    /// The request is validated against the sandbox. If validation passes,
    /// an error is returned indicating that actual HTTP execution is not yet
    /// wired. This will be connected to reqwest once wasmtime integration
    /// is complete (C2.8).
    pub fn request(
        &self,
        method: &str,
        url: &str,
        _headers: &HashMap<String, String>,
        body: Option<&[u8]>,
    ) -> Result<HttpResponse, Box<dyn std::error::Error + Send + Sync>> {
        let body_str = body.map(|b| String::from_utf8_lossy(b));
        let body_ref = body_str.as_deref();

        let _validated_url = self
            .validate_request(method, url, body_ref)
            .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { Box::new(e) })?;

        Err("sandboxed HTTP request validated but execution not yet wired (pending C2.8)".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clawft_plugin::{PluginPermissions, PluginResourceConfig};

    fn sandbox_with_network(network: Vec<String>) -> Arc<PluginSandbox> {
        let permissions = PluginPermissions {
            network,
            ..Default::default()
        };
        Arc::new(PluginSandbox::from_manifest(
            "test-http-plugin".into(),
            permissions,
            &PluginResourceConfig::default(),
        ))
    }

    #[test]
    fn sandboxed_validate_allowed_domain() {
        let client =
            SandboxedHttpClient::new(sandbox_with_network(vec!["api.example.com".into()]));
        let result = client.validate_request("GET", "https://api.example.com/data", None);
        assert!(result.is_ok());
    }

    #[test]
    fn sandboxed_validate_denied_domain() {
        let client =
            SandboxedHttpClient::new(sandbox_with_network(vec!["api.example.com".into()]));
        let result = client.validate_request("GET", "https://evil.example.com/data", None);
        assert!(result.is_err());
    }

    #[test]
    fn sandboxed_validate_private_ip_blocked() {
        let client = SandboxedHttpClient::new(sandbox_with_network(vec!["*".into()]));
        let result = client.validate_request("GET", "http://127.0.0.1/", None);
        assert!(result.is_err());
    }

    #[test]
    fn sandboxed_request_validates_then_returns_not_wired() {
        let client =
            SandboxedHttpClient::new(sandbox_with_network(vec!["api.example.com".into()]));
        let headers = HashMap::new();
        let result = client.request("GET", "https://api.example.com/", &headers, None);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("not yet wired"),
            "expected 'not yet wired', got: {err}"
        );
    }

    #[test]
    fn sandboxed_request_fails_validation_before_wiring() {
        let client =
            SandboxedHttpClient::new(sandbox_with_network(vec!["api.example.com".into()]));
        let headers = HashMap::new();
        let result = client.request("GET", "https://evil.com/", &headers, None);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("not in network allowlist"),
            "expected allowlist error, got: {err}"
        );
    }

    #[test]
    fn sandboxed_request_no_network_denied() {
        let client = SandboxedHttpClient::new(sandbox_with_network(vec![]));
        let headers = HashMap::new();
        let result = client.request("GET", "https://api.example.com/", &headers, None);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("not permitted"),
            "expected denied error, got: {err}"
        );
    }
}
