use super::DeviceProperties;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TopologyInfo {
    pub model: String,
    pub num_layers: u32,
    pub devices: Vec<DeviceProperties>,
    pub assignments: Vec<Assignment>,
    // can be anything
    pub solution: Option<serde_json::Value>,
}

impl TopologyInfo {
    /// Fetch topology from the API
    pub async fn fetch(api_url: &str) -> color_eyre::Result<TopologyInfo> {
        let url = format!("{}/v1/topology", api_url);
        let response = reqwest::get(&url).await?;

        // Get the response text first, regardless of status
        let status = response.status();
        let text = response.text().await?;

        // Check if the response contains an error detail message (for any status code)
        if let Ok(error_response) = serde_json::from_str::<serde_json::Value>(&text) {
            if let Some(detail) = error_response.get("detail").and_then(|d| d.as_str()) {
                if detail.contains("No topology configured") || detail.contains("prepare_topology")
                {
                    return Err(color_eyre::eyre::eyre!("No topology configured"));
                }
                // For other detail messages, include them in the error
                if !status.is_success() {
                    return Err(color_eyre::eyre::eyre!("{}", detail));
                }
            }
        }

        // If we couldn't parse a detail message and status is not success, return generic error
        if !status.is_success() {
            if status == reqwest::StatusCode::NOT_FOUND {
                return Err(color_eyre::eyre::eyre!(
                    "No topology found - model not loaded"
                ));
            }
            return Err(color_eyre::eyre::eyre!(
                "API returned error: {} {}",
                status.as_u16(),
                status.canonical_reason().unwrap_or("Unknown error")
            ));
        }

        // Try to parse as successful topology response
        let topology: TopologyInfo = serde_json::from_str(&text)
            .map_err(|e| color_eyre::eyre::eyre!("Failed to parse topology response: {}", e))?;
        Ok(topology)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ManualTopologyInfo {
    pub model: String,
    pub devices: Vec<DeviceProperties>,
    pub assignments: Vec<Assignment>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Assignment {
    pub service: String,
    pub layers: Vec<Vec<u32>>,
    pub next_service: String,
    pub window_size: u32,
}
