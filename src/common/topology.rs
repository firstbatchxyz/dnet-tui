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
