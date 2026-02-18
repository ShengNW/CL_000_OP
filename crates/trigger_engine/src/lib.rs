use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TriggerInput {
    pub reason: String,
    pub hint: Option<f32>,
}

pub fn score(input: &TriggerInput) -> f32 {
    let mut base = input.hint.unwrap_or(0.5);
    if !input.reason.trim().is_empty() {
        base += 0.2;
    }
    base.clamp(0.0, 1.0)
}
