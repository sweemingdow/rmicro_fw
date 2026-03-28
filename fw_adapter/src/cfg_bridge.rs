use fw_base::configuration::static_config::MySqlConfig;
use fw_error::{FwError, FwResult};
use fw_sqlx::mysql::client::MySqlOptions;

pub struct MysqlConfigWrapper(pub MySqlConfig);

impl MysqlConfigWrapper {
    pub fn try_into_options(cfg: Option<MySqlConfig>) -> FwResult<MySqlOptions> {
        let c = cfg.ok_or_else(|| FwError::ConfigError("mysql", "configuration missing".into()))?;
        Ok(MysqlConfigWrapper(c).into())
    }
}

impl From<MysqlConfigWrapper> for MySqlOptions {
    fn from(wrapper: MysqlConfigWrapper) -> Self {
        let cfg = wrapper.0;
        Self {
            addr: cfg.host,
            username: cfg.username,
            password: cfg.password,
            db_name: cfg.db_name,
            max_conn: Some(cfg.max_conn),
            min_conn: Some(cfg.min_conn),
            max_lifetime: Some(cfg.max_lifetime),
            idle_timeout: Some(cfg.idle_timeout),
        }
    }
}
