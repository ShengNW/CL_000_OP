use anyhow::{anyhow, Context, Result};
use serde_json::Value;

#[derive(Clone, Debug)]
pub struct AwClient {
    base_url: String,
}

impl AwClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
        }
    }

    pub fn get_info(&self) -> Result<Value> {
        self.get_json("/api/0/info")
    }

    pub fn get_buckets(&self) -> Result<Value> {
        self.get_json("/api/0/buckets")
    }

    fn get_json(&self, path: &str) -> Result<Value> {
        let url = format!("{}{}", self.base_url.trim_end_matches('/'), path);
        let response = ureq::get(&url)
            .call()
            .with_context(|| format!("GET {} failed", url))?;
        let text = response
            .into_string()
            .map_err(|e| anyhow!("read response body failed: {}", e))?;
        let value = serde_json::from_str(&text).map_err(|e| anyhow!("parse json failed: {}", e))?;
        Ok(value)
    }
}
