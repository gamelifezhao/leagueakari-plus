use reqwest::StatusCode;
use serde::de::DeserializeOwned;
use thiserror::Error;

use super::{auth, lockfile::Lockfile};

#[derive(Debug, Clone)]
pub struct LcuClient {
    base_url: String,
    auth_header: String,
    http: reqwest::Client,
}

#[derive(Debug, Error)]
pub enum LcuClientError {
    #[error("failed to build LCU HTTP client: {0}")]
    Build(#[source] reqwest::Error),
    #[error("LCU request failed for {url}: {source}")]
    Request { url: String, source: reqwest::Error },
    #[error("LCU returned {status} for {url}: {body}")]
    Status {
        url: String,
        status: StatusCode,
        body: String,
    },
    #[error("failed to parse LCU JSON from {url}: {source}")]
    Json { url: String, source: reqwest::Error },
}

impl LcuClient {
    pub fn new(lockfile: &Lockfile) -> Result<Self, LcuClientError> {
        let http = reqwest::Client::builder()
            .danger_accept_invalid_certs(true)
            .build()
            .map_err(LcuClientError::Build)?;

        Ok(Self {
            base_url: format!("{}://127.0.0.1:{}", lockfile.protocol, lockfile.port),
            auth_header: auth::basic_auth_header(&lockfile.password),
            http,
        })
    }

    pub async fn get_json<T>(&self, path: &str) -> Result<T, LcuClientError>
    where
        T: DeserializeOwned,
    {
        let url = self.url(path);
        let response = self
            .http
            .get(&url)
            .header("Authorization", &self.auth_header)
            .send()
            .await
            .map_err(|source| LcuClientError::Request {
                url: url.clone(),
                source,
            })?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_else(|_| String::new());
            return Err(LcuClientError::Status { url, status, body });
        }

        response
            .json::<T>()
            .await
            .map_err(|source| LcuClientError::Json { url, source })
    }

    fn url(&self, path: &str) -> String {
        if path.starts_with('/') {
            format!("{}{}", self.base_url, path)
        } else {
            format!("{}/{}", self.base_url, path)
        }
    }
}
