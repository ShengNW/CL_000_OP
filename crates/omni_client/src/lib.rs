use anyhow::{anyhow, Context, Result};
use serde_json::Value;

#[derive(Clone, Debug)]
pub struct OmniClient {
    base_url: String,
}

impl OmniClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
        }
    }

    pub fn probe(&self) -> Result<Value> {
        match self.get_json("/probe") {
            Ok(value) => Ok(value),
            Err(_) => self.get_json("/probe/"),
        }
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
