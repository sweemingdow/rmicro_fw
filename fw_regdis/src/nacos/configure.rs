use crate::nacos;
use crate::nacos::client::NacosClient;
use dashmap::DashMap;
use fw_error::lib_error::FwError;
use fw_error::result::FwResult;
use nacos_sdk::api::config;
use serde::de::DeserializeOwned;
use std::sync::Arc;

#[derive(Clone)]
pub struct NacosConfigure {
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

impl NacosConfigure {
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
            .get_config(data_id.clone(), group.clone())
            .await
            .map_err(|e| {
                FwError::SdkError(
                    "nacos fetch config",
                    format!(
                        "data_id={}, group={}, err={}",
                        data_id,
                        group,
                        e.to_string()
                    ),
                )
            })
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

impl NacosConfigure {
    pub async fn add_config_listener<L>(
        &self,
        data_id: String,
        group: String,
        listener: Arc<L>,
    ) -> FwResult<()>
    where
        L: config::ConfigChangeListener + 'static,
    {
        let key = format!("{}:{}", data_id, group);
        self.name_to_listener.insert(key, listener.clone());

        self.cfg_cli
            .add_listener(data_id, group, listener)
            .await
            .map_err(|e| FwError::SdkError("nacos sdk add listener", e.to_string()))
    }
}

impl NacosConfigure {
    pub async fn fetch_static_config<T: DeserializeOwned>(
        &self,
        config_group: &str,
    ) -> FwResult<T> {
        self.fetch_standard_config(nacos::STATIC_CONFIG_NAME, config_group)
            .await
    }

    pub async fn fetch_dynamic_config<T: DeserializeOwned>(
        &self,
        config_group: &str,
    ) -> FwResult<T> {
        self.fetch_standard_config(nacos::DYNAMIC_CONFIG_NAME, config_group)
            .await
    }

    async fn fetch_standard_config<T: DeserializeOwned>(
        &self,
        config_name: &str,
        config_group: &str,
    ) -> FwResult<T> {
        let resp = self
            .fetch_config(config_name.to_string(), config_group.to_string())
            .await?;

        serde_yaml::from_str::<T>(&resp.content()).map_err(|e| {
            FwError::ParseError(format!(
                "parse {} failed, group={}, err={}",
                config_name,
                config_group,
                e.to_string()
            ))
        })
    }

    pub async fn listen_dynamic_config<L>(
        &self,
        config_group: &str,
        listener: Arc<L>,
    ) -> FwResult<()>
    where
        L: config::ConfigChangeListener + 'static,
    {
        self.add_config_listener(
            nacos::DYNAMIC_CONFIG_NAME.to_string(),
            config_group.to_string(),
            listener,
        )
        .await
    }
}
