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
        #[cfg(feature = "fake")]
        let (core, data, frontend) = (
            "http://127.0.0.1:3001".to_string(),
            "http://127.0.0.1:3002".to_string(),
            "http://127.0.0.1:3000".to_string(),
        );

        #[cfg(not(feature = "fake"))]
        let (core, data, frontend) = (
            "https://api.data.banyan.computer".to_string(),
            "https://distributor.data.banyan.computer".to_string(),
            "https://alpha.data.banyan.computer".to_string(),
        );

        Self {
            core,
            data,
            frontend,
        }
    }
}
