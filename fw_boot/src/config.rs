use fw_error::lib_error::FwError;
use fw_error::result::FwResult;
use serde::{Deserialize, Serialize};
use std::{env, fs, time};

#[derive(Debug, Deserialize)]
pub struct Config {
    pub app_cfg: AppConfig,

    pub nacos_cli_cfg: NacosClientConfig,

    pub nacos_center_cfg: NacosCenterConfig,

    pub log_cfg: LogConfig,
}

#[derive(Debug, Deserialize)]
pub struct AppConfig {
    #[serde(skip)]
    pub app_name: String,

    #[serde(skip)]
    pub profile: String,

    pub http_port: u16,

    pub rpc_port: u16,

    // 服务停机超时时间(http_server, rpc_server...)
    pub stop_timeout: Option<time::Duration>,

    // 组件清理超时时间(sqL, redis...)
    pub component_clean_timeout: Option<time::Duration>,

    // 停止的阶段数
    pub stop_stages: Option<u8>,

    // 每个阶段停止的超时时间
    /*
    stop_timeout > stop_stages * stage_stop_timeout
    */
    pub stage_stop_timeout: Option<time::Duration>,
}

#[derive(Debug, Deserialize)]
pub struct NacosClientConfig {
    pub server_addr: String,

    pub namespace_id: String,

    #[serde(skip)]
    pub username: String,

    #[serde(skip)]
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct NacosCenterConfig {
    pub config: NacosConfig,

    pub registry: NacosRegistry,
}

#[derive(Debug, Deserialize)]
pub struct NacosConfig {
    pub group_name: String,
}

#[derive(Debug, Deserialize)]
pub struct NacosRegistry {
    pub group_name: String,
}

#[derive(Debug, Deserialize)]
pub struct LogConfig {
    pub max_log_files: u16,
    pub log_dir: String,
    pub rust_log: String, // env中没有设置, 兜底的
}

impl Config {
    // .env.user-api.local
    pub fn from_env() -> FwResult<Self> {
        let prepare_lam = || -> FwResult<(String, String, String)> {
            // export中获取
            let app_name = get_var_from_env("APP_NAME")?;
            let profile = get_var_from_env("PROFILE")?;

            load_env(&build_env_path(&app_name, &profile))?;

            Ok((app_name, profile, get_var_from_env("CONFIG_PATH")?))
        };

        let (app_name, profile, cfg_path) = prepare_lam()?;

        // 解析配置文件
        let contents = fs::read_to_string(&cfg_path)
            .map_err(|e| FwError::FileError("read config contents", e.to_string()))?;

        let mut cfg: Self = serde_yaml::from_str(&contents)
            .map_err(|e| FwError::ParseError(format!("parse config failed, e={}", e)))?;

        cfg.app_cfg.profile = profile;
        cfg.app_cfg.app_name = app_name;

        // 从env中回填数据到config中
        let nacos_username = get_var_from_env("NACOS_USERNAME")?;
        let nacos_pwd = get_var_from_env("NACOS_PWD")?;

        cfg.nacos_cli_cfg.username = nacos_username;
        cfg.nacos_cli_cfg.password = nacos_pwd;

        Ok(cfg)
    }
}

// .env.user-api.local
fn build_env_path(app_name: &str, profile: &str) -> String {
    format!(".env.{}.{}", app_name, profile)
}

fn load_env(env_path: &str) -> FwResult<()> {
    dotenv::from_path(env_path).map_err(|e| FwError::LoadError("env", e.to_string()))
}

fn get_var_from_env(key: &str) -> FwResult<String> {
    env::var(key).map_err(|e| FwError::LoadError("env", e.to_string()))
}
