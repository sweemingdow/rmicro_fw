use crate::nacos::configuration::NacosConfiguration;
use crate::nacos::discovery::NacosDiscovery;
use crate::nacos::registry::{DeregisterOptions, NacosRegister, RegisterOptions};
use async_trait::async_trait;
use fw_error::result::FwResult;
use nacos_sdk::api::{config, naming};
use std::sync::Arc;

pub struct NacosProxy {
    _register: Arc<dyn NacosRegister + Send + Sync>,
    _configuration: Arc<NacosConfiguration>,
    _discovery: Arc<NacosDiscovery>,
}

impl NacosProxy {
    pub fn with(
        register: Box<dyn NacosRegister + Send + Sync>,
        configuration: NacosConfiguration,
        discovery: NacosDiscovery,
    ) -> Self {
        Self {
            // box直接转arc
            _register: register.into(),
            _configuration: Arc::new(configuration),
            _discovery: Arc::new(discovery),
        }
    }

    pub fn get_nacos_register(&self) -> Arc<dyn NacosRegister + Send + Sync> {
        self._register.clone()
    }

    pub fn get_nacos_discover(&self) -> Arc<NacosDiscovery> {
        self._discovery.clone()
    }

    pub fn get_nacos_configuration(&self) -> Arc<NacosConfiguration> {
        self._configuration.clone()
    }
}

#[async_trait]
impl NacosRegister for NacosProxy {
    async fn register(&self, reg_ops: RegisterOptions) -> FwResult<()> {
        self._register.register(reg_ops).await
    }

    async fn deregister(&self, deg_ops: DeregisterOptions) -> FwResult<()> {
        self._register.deregister(deg_ops).await
    }
}

impl NacosProxy {
    pub async fn fetch_config(
        &self,
        data_id: String,
        group: String,
    ) -> FwResult<config::ConfigResponse> {
        self._configuration.fetch_config(data_id, group).await
    }

    pub async fn listen_then_save<F>(
        &self,
        data_id: String,
        group: String,
        handler: F,
    ) -> FwResult<()>
    where
        F: Fn(Arc<config::ConfigResponse>) + Send + Sync + 'static,
    {
        self._configuration
            .listen_then_save(data_id, group, handler)
            .await
    }
}

impl NacosProxy {
    pub async fn discover(
        &self,
        service_name: String,
        group_name: Option<String>,
    ) -> FwResult<Vec<naming::ServiceInstance>> {
        self._discovery.discover(service_name, group_name).await
    }

    pub async fn watch_then_save<F>(
        &self,
        service_name: String,
        group_name: Option<String>,
        handler: F,
    ) -> FwResult<()>
    where
        F: Fn(Arc<naming::NamingChangeEvent>) + Send + Sync + 'static,
    {
        self._discovery
            .watch_then_save(service_name, group_name, handler)
            .await
    }

    pub async fn unwatch_all(&self) -> Vec<FwResult<()>> {
        self._discovery.unwatch_all().await
    }
}
