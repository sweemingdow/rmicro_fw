use serde::Deserialize;
use std::collections::HashMap;
use std::time;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct RouterConfig {
    pub extract_depth: u8,
    pub table_mode: String,
    pub default_timeout_config: TimeoutItem,
    pub table: HashMap<String, TableItem>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct TableItem {
    pub addresses: Vec<String>,
    pub timeout_config: Option<TimeoutItem>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct TimeoutItem {
    #[serde(with = "humantime_serde")]
    pub read_timeout: time::Duration,

    #[serde(with = "humantime_serde")]
    pub write_timeout: time::Duration,

    #[serde(with = "humantime_serde")]
    pub idle_timeout: time::Duration,

    #[serde(with = "humantime_serde")]
    pub conn_timeout: time::Duration,

    #[serde(with = "humantime_serde")]
    pub total_conn_timeout: time::Duration,
}
