//! WASI HTTP client stub.
//!
//! Provides a [`WasiHttpClient`] with a self-contained API for HTTP operations.
//! Currently all methods return errors, as WASI HTTP preview2 support is not yet
//! available. Once the WASI HTTP outbound API stabilises, this will be replaced with
//! real network calls using the `wasi:http/outgoing-handler` interface.
//!
//! This module is fully decoupled from `clawft-platform` so it can compile for
//! `wasm32-wasip1` without pulling in tokio or reqwest.

use std::collections::HashMap;

/// HTTP response from a request.
#[derive(Debug, Clone)]
pub struct HttpResponse {
    /// HTTP status code (e.g., 200, 404, 500).
    pub status: u16,
    /// Response headers as key-value pairs.
    pub headers: HashMap<String, String>,
    /// Raw response body bytes.
    pub body: Vec<u8>,
}

impl HttpResponse {
    /// Parse body as UTF-8 text.
    pub fn text(&self) -> Result<String, std::string::FromUtf8Error> {
        String::from_utf8(self.body.clone())
    }

    /// Check if status is success (2xx).
    pub fn is_success(&self) -> bool {
        (200..300).contains(&self.status)
    }
}

/// HTTP client for WASI environments.
///
/// This is a stub implementation that will use WASI HTTP preview2
/// (`wasi:http/outgoing-handler`) once it is stable. Until then, all
/// methods return an error indicating the feature is not yet available.
pub struct WasiHttpClient;

impl WasiHttpClient {
    /// Create a new WASI HTTP client.
    pub fn new() -> Self {
        Self
    }

    /// Send an HTTP request with the given method, URL, headers, and optional body.
    pub fn request(
        &self,
        _method: &str,
        _url: &str,
        _headers: &HashMap<String, String>,
        _body: Option<&[u8]>,
    ) -> Result<HttpResponse, Box<dyn std::error::Error + Send + Sync>> {
        Err(
            "WASI HTTP not yet implemented: waiting for wasi:http/outgoing-handler stabilisation"
                .into(),
        )
    }

    /// Send an HTTP GET request.
    pub fn get(
        &self,
        url: &str,
        headers: &HashMap<String, String>,
    ) -> Result<HttpResponse, Box<dyn std::error::Error + Send + Sync>> {
        self.request("GET", url, headers, None)
    }

    /// Send an HTTP POST request with a body.
    pub fn post(
        &self,
        url: &str,
        headers: &HashMap<String, String>,
        body: &[u8],
    ) -> Result<HttpResponse, Box<dyn std::error::Error + Send + Sync>> {
        self.request("POST", url, headers, Some(body))
    }
}

impl Default for WasiHttpClient {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wasi_http_client_can_be_created() {
        let _client = WasiHttpClient::new();
    }

    #[test]
    fn wasi_http_client_default() {
        let _client = WasiHttpClient::default();
    }

    #[test]
    fn request_returns_error() {
        let client = WasiHttpClient::new();
        let headers = HashMap::new();
        let result = client.request("GET", "https://example.com", &headers, None);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("WASI HTTP not yet implemented"),
            "unexpected error message: {err}"
        );
    }

    #[test]
    fn get_returns_error() {
        let client = WasiHttpClient::new();
        let headers = HashMap::new();
        let result = client.get("https://example.com", &headers);
        assert!(result.is_err());
    }

    #[test]
    fn post_returns_error() {
        let client = WasiHttpClient::new();
        let headers = HashMap::new();
        let result = client.post("https://example.com", &headers, b"body");
        assert!(result.is_err());
    }

    #[test]
    fn wasi_http_client_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<WasiHttpClient>();
    }
}
