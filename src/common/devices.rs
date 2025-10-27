use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// A device info as retrieved from the API, which reads from the discovery module.
///
/// As such, this should match the [`DnetDeviceProperties`](https://github.com/firstbatchxyz/dnet-p2p/blob/master/src/service/properties.rs#L10)
/// class of dnet-p2p.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeviceProperties {
    /// Whether this device is a manager node (API).
    #[serde(default)]
    pub is_manager: bool,
    /// Whether this device is currently busy.
    #[serde(default)]
    pub is_busy: bool,
    /// The instance name, e.g., "shard-01".
    /// FIXME: name mismatch issues due to prepare_topology vs devices arrays
    #[serde(rename(serialize = "name", deserialize = "instance"))]
    pub instance: String,
    /// The port that HTTP server listens on.
    pub server_port: u16,
    /// The port that shard communication (gRPC) listens on.
    pub shard_port: u16,
    /// The local IP address of the device.
    pub local_ip: String,
    /// Additional Thunderbolt-specific info, if applicable.
    pub thunderbolt: Option<ThunderboltInfo>,
}

/// The response from the `/v1/devices` endpoint.
#[derive(Debug, Clone, Deserialize)]
pub struct DevicesResponse {
    pub devices: HashMap<String, DeviceProperties>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThunderboltInfo {
    pub ip_addr: String,
    // Using serde_json::Value to handle the complex nested structure
    pub instances: serde_json::Value,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_properties_serialization() {
        let json_data = r#"
        {
            "is_manager": true,
            "is_busy": false,
            "instance": "shard-01",
            "server_port": 8080,
            "shard_port": 50051,
            "local_ip": "192.168.1.100",
    }"#;

        let device: DeviceProperties = serde_json::from_str(json_data).unwrap();
        assert_eq!(device.is_manager, true);
        assert_eq!(device.is_busy, false);
        assert_eq!(device.instance, "shard-01");
        assert_eq!(device.server_port, 8080);
        assert_eq!(device.shard_port, 50051);
        assert_eq!(device.local_ip, "192.168.1.100");
    }
}
