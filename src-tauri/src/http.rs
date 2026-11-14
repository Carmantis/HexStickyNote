//! Shared HTTP client (Ollama API, HexTime sidecar)

use reqwest::Client;
use std::time::Duration;

/// Managed state: one connection pool for the whole app
pub struct HttpClient {
    client: Client,
}

impl HttpClient {
    pub fn new() -> Self {
        let client = Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_default();
        Self { client }
    }

    pub fn client(&self) -> &Client {
        &self.client
    }
}

impl Default for HttpClient {
    fn default() -> Self {
        Self::new()
    }
}
