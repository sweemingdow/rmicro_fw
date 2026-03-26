use serde::Deserialize;
use std::time::Duration;

// 通用的静态配置
#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct CommStaticConfig {
    pub mysql_cfg: Option<MySqlConfig>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct MySqlConfig {
    pub host: String,

    pub username: String,

    pub password: String,

    pub db_name: String,

    pub max_conn: u16,

    pub min_conn: u16,

    #[serde(with = "humantime_serde")]
    pub max_lifetime: Duration,

    #[serde(with = "humantime_serde")]
    pub idle_timeout: Duration,
}
