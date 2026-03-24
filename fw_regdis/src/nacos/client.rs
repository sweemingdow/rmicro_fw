use crate::nacos::NacosResult;
use nacos_sdk::api::{config, error, naming, plugin, props};
use fw_error::lib_error::FwError;
use fw_error::result::FwResult;

pub struct NacosClient {
    cfg_cli: config::ConfigService,
    naming_cli: naming::NamingService,
}

pub struct NacosCliOptions {
    pub server_addr: String,
    pub namespace_id: String,
    pub username: String,
    pub password: String,
}

impl NacosClient {
    pub async fn with_ops(op: NacosCliOptions) -> FwResult<NacosClient> {
        let naming_cli = naming::NamingServiceBuilder::new(Self::new_client_ops(&op))
            .enable_auth_plugin_http()
            .build()
            .await
            .map_err(|e| FwError::SdkError("nacos naming client init", e.to_string()))?;

        let cfg_cli = config::ConfigServiceBuilder::new(Self::new_client_ops(&op))
            .enable_auth_plugin_http()
            .build()
            .await
            .map_err(|e| FwError::SdkError("nacos config client init", e.to_string()))?;

        Ok(NacosClient {
            cfg_cli,
            naming_cli,
        })
    }

    pub fn get_cfg_cli(&self) -> config::ConfigService {
        self.cfg_cli.clone()
    }

    pub fn get_naming_cli(&self) -> naming::NamingService {
        self.naming_cli.clone()
    }

    fn new_client_ops(op: &NacosCliOptions) -> props::ClientProps {
        props::ClientProps::new()
            .server_addr(op.server_addr.clone())
            .namespace(op.namespace_id.clone())
            .config_load_cache_at_start(false)
            .load_cache_at_start(false)
            .naming_load_cache_at_start(false)
            .auth_username(op.username.clone())
            .auth_password(op.password.clone())
    }
}
