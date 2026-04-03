pub mod router_config;

use serde::Deserialize;
use std::time;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct GatewayServerConfig {
    #[serde(with = "humantime_serde")]
    pub grace_period_timeout: time::Duration,

    #[serde(with = "humantime_serde")]
    pub graceful_shutdown_timeout: time::Duration,

    pub listen_addr: String,

    pub listen_port: u16,

    pub worker_count: u8,

    pub conn_pool_size: u16,
}
