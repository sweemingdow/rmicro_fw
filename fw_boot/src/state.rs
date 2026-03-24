use crate::config::Config;
use fw_error::result::FwResult;
use fw_regdis::nacos::client::{NacosCliOptions, NacosClient};
use fw_regdis::nacos::configuration::NacosConfiguration;
use fw_regdis::nacos::discovery::NacosDiscovery;
use fw_regdis::nacos::proxy::NacosProxy;
use fw_regdis::nacos::registry::NacosRegisterImpl;
use fw_rpc::tonic_srv::chan_factory::RpcChanFactory;
use std::sync::Arc;
use tokio_util::sync;

pub struct RunState {
    cfg: Arc<Config>,
    nacos_proxy: Arc<NacosProxy>,
    cancel_token: sync::CancellationToken,
    rpc_chan_factory: Arc<RpcChanFactory>,
}

impl RunState {
    pub async fn new(cfg: Arc<Config>, cancel_token: sync::CancellationToken) -> FwResult<Self> {
        let nacos_cli = NacosClient::with_ops(NacosCliOptions {
            server_addr: "".to_string(),
            namespace_id: "".to_string(),
            username: "".to_string(),
            password: "".to_string(),
        })
        .await?;

        tracing::info!("init nacos client successfully");

        let register = NacosRegisterImpl::new(&nacos_cli);
        let discover = NacosDiscovery::new(&nacos_cli);
        let configuration = NacosConfiguration::new(&nacos_cli);
        let nacos_proxy = Arc::new(NacosProxy::with(register, configuration, discover));

        let chan_factory = Arc::new(RpcChanFactory::new(
            &cfg.nacos_center_cfg.registry.group_name,
            nacos_proxy.clone(),
        ));

        Ok(Self {
            cfg,
            nacos_proxy,
            cancel_token,
            rpc_chan_factory: chan_factory,
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

    pub fn rpc_chan_factory(&self) -> Arc<RpcChanFactory> {
        self.rpc_chan_factory.clone()
    }
}
