//! HTTP client abstraction and native implementation.
//!
//! Provides a platform-agnostic [`HttpClient`] trait and a native implementation
//! backed by [`reqwest`]. A WASM implementation would use the browser's fetch API.

use async_trait::async_trait;
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

    /// Parse body as JSON.
    pub fn json<T: serde::de::DeserializeOwned>(&self) -> Result<T, serde_json::Error> {
        serde_json::from_slice(&self.body)
    }

    /// Check if status is success (2xx).
    pub fn is_success(&self) -> bool {
        (200..300).contains(&self.status)
    }
}

/// Platform-agnostic HTTP client.
///
/// Implementors provide transport-level HTTP operations. The native
/// implementation uses [`reqwest`]; a WASM implementation would use fetch.
#[cfg_attr(not(feature = "browser"), async_trait)]
#[cfg_attr(feature = "browser", async_trait(?Send))]
pub trait HttpClient: Send + Sync {
    /// Send an HTTP request with the given method, URL, headers, and optional body.
    async fn request(
        &self,
        method: &str,
        url: &str,
        headers: &HashMap<String, String>,
        body: Option<&[u8]>,
    ) -> Result<HttpResponse, Box<dyn std::error::Error + Send + Sync>>;

    /// Send an HTTP GET request.
    async fn get(
        &self,
        url: &str,
        headers: &HashMap<String, String>,
    ) -> Result<HttpResponse, Box<dyn std::error::Error + Send + Sync>> {
        self.request("GET", url, headers, None).await
    }

    /// Send an HTTP POST request with a body.
    async fn post(
        &self,
        url: &str,
        headers: &HashMap<String, String>,
        body: &[u8],
    ) -> Result<HttpResponse, Box<dyn std::error::Error + Send + Sync>> {
        self.request("POST", url, headers, Some(body)).await
    }
}

/// Native HTTP client using [`reqwest`].
#[cfg(feature = "native")]
pub struct NativeHttpClient {
    client: reqwest::Client,
}

#[cfg(feature = "native")]
impl NativeHttpClient {
    /// Create a new native HTTP client with sensible defaults.
    ///
    /// Uses a 60-second timeout and 30-second idle connection timeout
    /// with connection pooling enabled.
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(60))
                .pool_idle_timeout(std::time::Duration::from_secs(30))
                .build()
                .expect("failed to build reqwest client"),
        }
    }
}

#[cfg(feature = "native")]
impl Default for NativeHttpClient {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "native")]
#[async_trait]
impl HttpClient for NativeHttpClient {
    async fn request(
        &self,
        method: &str,
        url: &str,
        headers: &HashMap<String, String>,
        body: Option<&[u8]>,
    ) -> Result<HttpResponse, Box<dyn std::error::Error + Send + Sync>> {
        let reqwest_method = method.parse::<reqwest::Method>()?;
        let mut builder = self.client.request(reqwest_method, url);

        for (key, value) in headers {
            builder = builder.header(key.as_str(), value.as_str());
        }

        if let Some(body_bytes) = body {
            builder = builder.body(body_bytes.to_vec());
        }

        let response = builder.send().await?;

        let status = response.status().as_u16();
        let mut resp_headers = HashMap::new();
        for (key, value) in response.headers() {
            if let Ok(v) = value.to_str() {
                resp_headers.insert(key.as_str().to_string(), v.to_string());
            }
        }
        let resp_body = response.bytes().await?.to_vec();

        Ok(HttpResponse {
            status,
            headers: resp_headers,
            body: resp_body,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_http_response_text() {
        let response = HttpResponse {
            status: 200,
            headers: HashMap::new(),
            body: b"hello world".to_vec(),
        };
        assert_eq!(response.text().unwrap(), "hello world");
    }

    #[test]
    fn test_http_response_text_invalid_utf8() {
        let response = HttpResponse {
            status: 200,
            headers: HashMap::new(),
            body: vec![0xFF, 0xFE],
        };
        assert!(response.text().is_err());
    }

    #[test]
    fn test_http_response_json() {
        let response = HttpResponse {
            status: 200,
            headers: HashMap::new(),
            body: br#"{"key": "value"}"#.to_vec(),
        };
        let parsed: HashMap<String, String> = response.json().unwrap();
        assert_eq!(parsed.get("key").unwrap(), "value");
    }

    #[test]
    fn test_http_response_json_invalid() {
        let response = HttpResponse {
            status: 200,
            headers: HashMap::new(),
            body: b"not json".to_vec(),
        };
        let result: Result<HashMap<String, String>, _> = response.json();
        assert!(result.is_err());
    }

    #[test]
    fn test_http_response_is_success() {
        for status in [200, 201, 204, 299] {
            let response = HttpResponse {
                status,
                headers: HashMap::new(),
                body: vec![],
            };
            assert!(response.is_success(), "status {status} should be success");
        }
    }

    #[test]
    fn test_http_response_is_not_success() {
        for status in [100, 301, 400, 404, 500] {
            let response = HttpResponse {
                status,
                headers: HashMap::new(),
                body: vec![],
            };
            assert!(
                !response.is_success(),
                "status {status} should not be success"
            );
        }
    }

    #[test]
    fn test_native_http_client_default() {
        let _client = NativeHttpClient::default();
    }
}
