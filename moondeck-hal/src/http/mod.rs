use anyhow::{Context, Result};
use esp_idf_svc::http::client::{Configuration as HttpConfig, EspHttpConnection};
use esp_idf_svc::io::{Write};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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

    pub fn get_with_headers(&self, url: &str, headers: &[(&str, &str)]) -> Result<HttpResponse> {
        let config = self.create_config();

        let mut client = EspHttpConnection::new(&config)
            .context("Failed to create HTTP connection")?;

        client.initiate_request(
            esp_idf_svc::http::Method::Get,
            url,
            headers,
        ).context("Failed to initiate request")?;

        client.initiate_response()
            .context("Failed to initiate response")?;

        let status = client.status();

        let mut body = Vec::new();
        let mut buf = [0u8; 1024];
        loop {
            match client.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => body.extend_from_slice(&buf[..n]),
                Err(e) => return Err(anyhow::anyhow!("Read error: {:?}", e)),
            }
        }

        let body_str = String::from_utf8_lossy(&body).to_string();

        Ok(HttpResponse {
            status,
            body: body_str,
            headers: HashMap::new(),
        })
    }

    pub fn post(&self, url: &str, body: &str, content_type: &str) -> Result<HttpResponse> {
        self.post_with_headers(url, body, content_type, &[])
    }

    pub fn post_with_headers(&self, url: &str, body: &str, content_type: &str, extra_headers: &[(&str, &str)]) -> Result<HttpResponse> {
        let config = self.create_config();

        let mut client = EspHttpConnection::new(&config)
            .context("Failed to create HTTP connection")?;

        let content_len = body.len().to_string();
        let mut headers: Vec<(&str, &str)> = vec![
            ("Content-Type", content_type),
            ("Content-Length", &content_len),
        ];
        headers.extend_from_slice(extra_headers);

        client.initiate_request(
            esp_idf_svc::http::Method::Post,
            url,
            &headers,
        ).context("Failed to initiate request")?;

        client.write_all(body.as_bytes())
            .context("Failed to write body")?;

        client.flush().context("Failed to flush")?;

        client.initiate_response()
            .context("Failed to initiate response")?;

        let status = client.status();

        let mut response_body = Vec::new();
        let mut buf = [0u8; 1024];
        loop {
            match client.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => response_body.extend_from_slice(&buf[..n]),
                Err(e) => return Err(anyhow::anyhow!("Read error: {:?}", e)),
            }
        }

        Ok(HttpResponse {
            status,
            body: String::from_utf8_lossy(&response_body).to_string(),
            headers: HashMap::new(),
        })
    }

    pub fn get_json<T: for<'de> Deserialize<'de>>(&self, url: &str) -> Result<T> {
        let response = self.get(url)?;
        serde_json::from_str(&response.body)
            .context("Failed to parse JSON response")
    }

    pub fn post_json<T: Serialize, R: for<'de> Deserialize<'de>>(
        &self,
        url: &str,
        body: &T,
    ) -> Result<R> {
        let body_str = serde_json::to_string(body)?;
        let response = self.post(url, &body_str, "application/json")?;
        serde_json::from_str(&response.body)
            .context("Failed to parse JSON response")
    }
}

impl Default for HttpClient {
    fn default() -> Self {
        Self::new()
    }
}
