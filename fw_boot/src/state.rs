use crate::config::Config;
use fw_error::result::FwResult;
use fw_regdis::nacos::client::{NacosCliOptions, NacosClient};
use fw_regdis::nacos::configure::NacosConfigure;
use fw_regdis::nacos::discovery::NacosDiscovery;
use fw_regdis::nacos::proxy::NacosProxy;
use fw_regdis::nacos::registry::NacosRegisterImpl;
use std::sync::Arc;
use tokio_util::sync;

pub struct RunState {
    app_name: String,
    profile: String,
    mip: String,
    cfg: Arc<Config>,
    nacos_proxy: Arc<NacosProxy>,
    cancel_token: sync::CancellationToken,
}

impl RunState {
    pub async fn new(
        app_name: &str,
        profile: &str,
        mip: &str,
        cfg: Arc<Config>,
        cancel_token: sync::CancellationToken,
    ) -> FwResult<Self> {
        let server_addr = cfg.nacos_cli_cfg.server_addr.clone();
        let namespace_id = cfg.nacos_cli_cfg.namespace_id.clone();
        let username = cfg.nacos_cli_cfg.username.clone();
        let password = cfg.nacos_cli_cfg.password.clone();

        let nacos_cli = NacosClient::with_ops(NacosCliOptions {
            server_addr,
            namespace_id,
            username,
            password,
        })
        .await?;

        tracing::info!("init nacos client successfully");

        let register = NacosRegisterImpl::new(&nacos_cli);
        let discover = NacosDiscovery::new(&nacos_cli);
        let configuration = NacosConfigure::new(&nacos_cli);
        let nacos_proxy = Arc::new(NacosProxy::with(register, configuration, discover));

        Ok(Self {
            app_name: app_name.to_string(),
            profile: profile.to_string(),
            mip: mip.to_string(),
            cfg,
            nacos_proxy,
            cancel_token,
        })
    }
}

impl RunState {
    pub fn cfg(&self) -> Arc<Config> {
        self.cfg.clone()
    }

    pub fn nacos_proxy(&self) -> Arc<NacosProxy> {
        self.nacos_proxy.clone()
    }

    pub fn cancel_token(&self) -> sync::CancellationToken {
        self.cancel_token.clone()
    }

    pub fn app_name(&self) -> &str {
        &self.app_name
    }

    pub fn profile(&self) -> &str {
        &self.profile
    }

    pub fn mip(&self) -> &str {
        &self.mip
    }
}
