use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NowFrame {
    pub timestamp: String,
    pub reason: String,
    pub aw_info: Option<Value>,
    pub aw_buckets: Option<Value>,
    pub omni_probe: Option<Value>,
}
