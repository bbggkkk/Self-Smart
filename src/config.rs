use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub endpoint: String,
    pub model: String,
    pub workdir: String,
    pub auto_commit: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            endpoint: "http://localhost:48000".to_string(),
            model: "Intel/Qwen3.5-9B-int4-AutoRound".to_string(),
            workdir: ".".to_string(),
            auto_commit: false,
        }
    }
}
