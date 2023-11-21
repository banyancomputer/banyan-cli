use serde::{Deserialize, Serialize};

/// Simple struct for moving around API endpoints
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct Endpoints {
    /// Remote endpoint for Metadata API
    pub core: String,
    /// Remote endpoint for Full Data API
    pub data: String,
    /// Remote endpoint for Frontend interaction
    pub frontend: String,
}

impl Default for Endpoints {
    fn default() -> Self {
        if option_env!("DEV_ENDPOINTS").is_some() {
            Self {
                core: "http://127.0.0.1:3001".to_string(),
                data: "http://127.0.0.1:3002".to_string(),
                frontend: "http://127.0.0.1:3000".to_string(),
            }
        } else {
            Self {
                core: "https://api.data.banyan.computer".to_string(),
                data: "https://distributor.data.banyan.computer".to_string(),
                frontend: "https://alpha.data.banyan.computer".to_string(),
            }
        }
    }
}
