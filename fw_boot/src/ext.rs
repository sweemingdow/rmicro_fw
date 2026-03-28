use fw_base::configuration::static_config::{CommStaticConfig, GwDispatchConfig};
use serde::Deserialize;

pub trait RunConfigExt {
    fn get_gw_dispatch_cfg(&self) -> &GwDispatchConfig;
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct SimpleStaticConfig {
    pub comm_static_cfg: CommStaticConfig,
}

impl RunConfigExt for SimpleStaticConfig {
    fn get_gw_dispatch_cfg(&self) -> &GwDispatchConfig {
        &self.comm_static_cfg.gw_dispatch_cfg
    }
}
