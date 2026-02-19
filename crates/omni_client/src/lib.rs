use anyhow::{anyhow, Context, Result};
use serde_json::{json, Value};

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

    pub fn parse(&self, base64_image: &str, parse_options: Option<&Value>) -> Result<Value> {
        let mut payload = json!({ "base64_image": base64_image });
        if let Some(options) = parse_options {
            if !options.is_null() {
                payload["parse_options"] = options.clone();
            }
        }

        match self.post_json("/parse", &payload) {
            Ok(value) => Ok(value),
            Err(_) => self.post_json("/parse/", &payload),
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

    fn post_json(&self, path: &str, payload: &Value) -> Result<Value> {
        let url = format!("{}{}", self.base_url.trim_end_matches('/'), path);
        let response = ureq::post(&url)
            .send_json(payload.clone())
            .with_context(|| format!("POST {} failed", url))?;
        let text = response
            .into_string()
            .map_err(|e| anyhow!("read response body failed: {}", e))?;
        let value = serde_json::from_str(&text).map_err(|e| anyhow!("parse json failed: {}", e))?;
        Ok(value)
    }
}
