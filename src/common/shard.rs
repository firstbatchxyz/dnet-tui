//! Shard API related stuff.
//!
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardHealthResponse {
    /// Health status (e.g., 'ok')
    pub status: String,
    /// Node identifier
    pub node_id: u32,
    /// Whether the node is running
    pub running: bool,
    /// Whether a model is currently loaded
    pub model_loaded: bool,
    /// Path to currently loaded model
    pub model_path: Option<String>,
    /// Layers assigned to this shard
    pub assigned_layers: Vec<u32>,
    /// Current activation queue size
    pub queue_size: u32,
    /// gRPC server port
    pub grpc_port: u16,
    /// HTTP server port
    pub http_port: u16,
    /// Short shard instance name (service label)
    pub instance: Option<String>,
}
