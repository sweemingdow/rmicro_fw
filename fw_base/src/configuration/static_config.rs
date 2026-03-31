use fw_error::{AppError, FwError, FwResult};
use serde::Deserialize;
use std::collections::HashMap;
use std::time::Duration;

// 通用的静态配置
#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct CommStaticConfig {
    pub gw_dispatch_cfg: GwDispatchConfig,
    pub mysql_cfg: Option<MySqlConfig>,
    pub rpc_call_cfg: Option<RpcCallConfig>,
}

impl CommStaticConfig {
    pub fn get_rpc_srv_ele(&self) -> FwResult<&HashMap<String, RpcChannelConfig>> {
        Ok(&self.get_rpc_config()?.caller_cfg.srv_ele)
    }

    pub fn get_rpc_global_timeout(&self) -> FwResult<Duration> {
        Ok(self.get_rpc_config()?.callee_cfg.global_timeout)
    }

    fn get_rpc_config(&self) -> FwResult<&RpcCallConfig> {
        self.rpc_call_cfg
            .as_ref()
            .ok_or_else(|| FwError::ConfigError("rpc config", "config missing".to_string()))
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct GwDispatchConfig {
    pub dispatch_val: String,
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
#[derive(Debug, Deserialize, Default, Clone)]
#[serde(rename_all = "kebab-case", default)]
pub struct RpcCallConfig {
    pub caller_cfg: CallerConfig,
    pub callee_cfg: CalleeConfig,
}

#[derive(Debug, Deserialize, Default, Clone)]
#[serde(rename_all = "kebab-case", default)]
pub struct CallerConfig {
    pub srv_ele: HashMap<String, RpcChannelConfig>,
}

#[derive(Debug, Deserialize, Default, Clone)]
#[serde(rename_all = "kebab-case", default)]
pub struct CalleeConfig {
    #[serde(with = "humantime_serde")]
    pub global_timeout: Duration,
}

#[derive(Debug, Deserialize, Default, Clone)]
#[serde(rename_all = "kebab-case", default)]
pub struct RpcChannelConfig {
    pub estimate_srv_max_count: Option<u16>, // 预估服务最多数量

    #[serde(with = "humantime_serde")]
    pub connect_timeout: Option<Duration>, // 连接超时

    #[serde(with = "humantime_serde")]
    pub request_timeout: Option<Duration>, // 请求总超时

    #[serde(with = "humantime_serde")]
    pub keep_alive_timeout: Option<Duration>, // 空闲连接超时

    #[serde(with = "humantime_serde")]
    pub tcp_keepalive: Option<Duration>, // TCP keepalive

    #[serde(with = "humantime_serde")]
    pub http2_keep_alive_interval: Option<Duration>, // HTTP2 ping间隔
}
