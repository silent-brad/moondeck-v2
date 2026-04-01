use std::collections::HashMap;

use anyhow::{Context, Result};
use esp_idf_svc::{
    http::client::{Configuration as HttpConfig, EspHttpConnection},
    io::Write,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpResponse {
    pub status: u16,
    pub body: String,
    pub headers: HashMap<String, String>,
}

pub struct HttpClient {
    timeout_ms: u32,
}

impl HttpClient {
    pub fn new() -> Self {
        Self { timeout_ms: 10_000 }
    }

    pub fn with_timeout(timeout_ms: u32) -> Self {
        Self { timeout_ms }
    }

    const MAX_BODY_SIZE: usize = 256 * 1024;

    fn read_body(client: &mut EspHttpConnection) -> Result<String> {
        let mut body = Vec::new();
        let mut buf = [0u8; 1024];
        loop {
            match client.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    body.extend_from_slice(&buf[..n]);
                    if body.len() > Self::MAX_BODY_SIZE {
                        return Err(anyhow::anyhow!("Response body too large"));
                    }
                }
                Err(e) => return Err(anyhow::anyhow!("Read error: {:?}", e)),
            }
        }
        Ok(String::from_utf8_lossy(&body).to_string())
    }

    fn read_body_bytes(client: &mut EspHttpConnection) -> Result<Vec<u8>> {
        let mut body = Vec::new();
        let mut buf = [0u8; 1024];
        loop {
            match client.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    body.extend_from_slice(&buf[..n]);
                    if body.len() > Self::MAX_BODY_SIZE {
                        return Err(anyhow::anyhow!("Response body too large"));
                    }
                }
                Err(e) => return Err(anyhow::anyhow!("Read error: {:?}", e)),
            }
        }
        Ok(body)
    }

    fn create_config(&self) -> HttpConfig {
        HttpConfig {
            timeout: Some(std::time::Duration::from_millis(self.timeout_ms as u64)),
            crt_bundle_attach: Some(esp_idf_svc::sys::esp_crt_bundle_attach),
            ..Default::default()
        }
    }

    pub fn get(&self, url: &str) -> Result<HttpResponse> {
        self.get_with_headers(url, &[])
    }

    pub fn get_bytes(&self, url: &str) -> Result<(u16, Vec<u8>)> {
        let config = self.create_config();
        let mut client =
            EspHttpConnection::new(&config).context("Failed to create HTTP connection")?;
        client
            .initiate_request(
                esp_idf_svc::http::Method::Get,
                url,
                &[
                    ("Connection", "close"),
                    (
                        "User-Agent",
                        "Moondeck/0.1 (ESP32-S3; +https://github.com/silent-brad/moondeck)",
                    ),
                ],
            )
            .context("Failed to initiate request")?;
        client
            .initiate_response()
            .context("Failed to initiate response")?;
        let status = client.status();
        let body = Self::read_body_bytes(&mut client)?;
        Ok((status, body))
    }

    pub fn get_with_headers(&self, url: &str, headers: &[(&str, &str)]) -> Result<HttpResponse> {
        let config = self.create_config();

        let mut client =
            EspHttpConnection::new(&config).context("Failed to create HTTP connection")?;

        let mut all_headers: Vec<(&str, &str)> = headers.to_vec();
        all_headers.push(("Connection", "close"));

        client
            .initiate_request(esp_idf_svc::http::Method::Get, url, &all_headers)
            .context("Failed to initiate request")?;

        client
            .initiate_response()
            .context("Failed to initiate response")?;

        let status = client.status();
        let body_str = Self::read_body(&mut client)?;

        Ok(HttpResponse {
            status,
            body: body_str,
            headers: HashMap::new(),
        })
    }

    pub fn post(&self, url: &str, body: &str, content_type: &str) -> Result<HttpResponse> {
        self.post_with_headers(url, body, content_type, &[])
    }

    pub fn post_with_headers(
        &self,
        url: &str,
        body: &str,
        content_type: &str,
        extra_headers: &[(&str, &str)],
    ) -> Result<HttpResponse> {
        let config = self.create_config();

        let mut client =
            EspHttpConnection::new(&config).context("Failed to create HTTP connection")?;

        let content_len = body.len().to_string();
        let mut headers: Vec<(&str, &str)> = vec![
            ("Content-Type", content_type),
            ("Content-Length", &content_len),
            ("Connection", "close"),
        ];
        headers.extend_from_slice(extra_headers);

        client
            .initiate_request(esp_idf_svc::http::Method::Post, url, &headers)
            .context("Failed to initiate request")?;

        client
            .write_all(body.as_bytes())
            .context("Failed to write body")?;

        client.flush().context("Failed to flush")?;

        client
            .initiate_response()
            .context("Failed to initiate response")?;

        let status = client.status();
        let body_str = Self::read_body(&mut client)?;

        Ok(HttpResponse {
            status,
            body: body_str,
            headers: HashMap::new(),
        })
    }

    pub fn get_json<T: for<'de> Deserialize<'de>>(&self, url: &str) -> Result<T> {
        let response = self.get(url)?;
        serde_json::from_str(&response.body).context("Failed to parse JSON response")
    }

    pub fn post_json<T: Serialize, R: for<'de> Deserialize<'de>>(
        &self,
        url: &str,
        body: &T,
    ) -> Result<R> {
        let body_str = serde_json::to_string(body)?;
        let response = self.post(url, &body_str, "application/json")?;
        serde_json::from_str(&response.body).context("Failed to parse JSON response")
    }
}

impl Default for HttpClient {
    fn default() -> Self {
        Self::new()
    }
}
