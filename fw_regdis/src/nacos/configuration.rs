use crate::nacos::client::NacosClient;
use dashmap::DashMap;
use fw_error::lib_error::FwError;
use fw_error::result::FwResult;
use nacos_sdk::api::config;
use std::sync::Arc;

#[derive(Clone)]
pub struct NacosConfiguration {
    cfg_cli: config::ConfigService,
    name_to_listener: DashMap<String, Arc<dyn config::ConfigChangeListener>>,
}

pub struct ChangeListener<F: Fn(Arc<config::ConfigResponse>)> {
    pub handler: F,
}

impl<F> config::ConfigChangeListener for ChangeListener<F>
where
    F: Fn(Arc<config::ConfigResponse>) + Send + Sync + 'static,
{
    fn notify(&self, config_resp: config::ConfigResponse) {
        (self.handler)(Arc::new(config_resp));
    }
}

impl NacosConfiguration {
    pub fn new(nacos_cli: &NacosClient) -> Self {
        Self {
            cfg_cli: nacos_cli.get_cfg_cli(),
            name_to_listener: Default::default(),
        }
    }

    pub async fn fetch_config(
        &self,
        data_id: String,
        group: String,
    ) -> FwResult<config::ConfigResponse> {
        self.cfg_cli
            .get_config(data_id, group)
            .await
            .map_err(|e| FwError::SdkError("nacos sdk config", e.to_string()))
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
        let listener = Arc::new(ChangeListener { handler });

        // data-id:group合并为key
        let key = format!("{}:{}", data_id, group);
        self.name_to_listener.insert(key, listener.clone());

        self.cfg_cli
            .add_listener(data_id, group, listener)
            .await
            .map_err(|e| FwError::SdkError("nacos sdk listen config", e.to_string()))
    }

    pub async fn unlisten_all(&self) -> Vec<FwResult<()>> {
        let mut targets = Vec::new();

        for entry in self.name_to_listener.iter() {
            targets.push(entry.key().clone());
        }

        let mut rst_list = Vec::<FwResult<()>>::with_capacity(self.name_to_listener.len());
        for key in targets {
            if let Some((_, listener)) = self.name_to_listener.remove(&key) {
                if let Some((data_id, group)) = key.split_once(':') {
                    tracing::info!("unlisten configuration, data_id={data_id}, group={group}");

                    rst_list.push(
                        self.cfg_cli
                            .remove_listener(data_id.to_string(), group.to_string(), listener)
                            .await
                            .map_err(|e| {
                                FwError::SdkError("nacos sdk unlisten config", e.to_string())
                            }),
                    )
                }
            }
        }

        rst_list
    }
}
