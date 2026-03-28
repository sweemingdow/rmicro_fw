use crate::nacos::NacosResult;
use crate::nacos::client::NacosClient;
use dashmap::DashMap;
use fw_error::lib_error::FwError;
use fw_error::result::FwResult;
use nacos_sdk::api::naming;
use std::sync::Arc;

pub struct NacosDiscovery {
    naming_cli: naming::NamingService,
    name_to_notifier: DashMap<String, Arc<dyn naming::NamingEventListener>>,
}

pub struct ChangeNotify<F: Fn(Arc<naming::NamingChangeEvent>)> {
    pub handler: F,
}

impl<F> naming::NamingEventListener for ChangeNotify<F>
where
    F: Fn(Arc<naming::NamingChangeEvent>) + Send + Sync + 'static,
{
    fn event(&self, event: Arc<naming::NamingChangeEvent>) {
        (self.handler)(event)
    }
}

impl NacosDiscovery {
    pub fn new(nacos_cli: &NacosClient) -> Self {
        Self {
            naming_cli: nacos_cli.get_naming_cli(),
            name_to_notifier: DashMap::new(),
        }
    }

    pub async fn discover(
        &self,
        service_name: String,
        group_name: Option<String>,
    ) -> FwResult<Vec<naming::ServiceInstance>> {
        self.naming_cli
            .select_instances(service_name, group_name, Vec::new(), false, true)
            .await
            .map_err(|e| FwError::SdkError("nacos sdk discover", e.to_string()))
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
        let notifier = Arc::new(ChangeNotify { handler });
        let group_name = group_name.unwrap_or_else(|| "DEFAULT_GROUP".to_string());
        // group_name:srv_name 合并为key
        let key = format!("{}:{}", group_name, service_name);
        self.name_to_notifier.insert(key, notifier.clone());

        self.naming_cli
            .subscribe(service_name, Some(group_name), Vec::new(), notifier)
            .await
            .map_err(|e| FwError::SdkError("nacos sdk watch", e.to_string()))
    }

    pub async fn unwatch_all(&self) -> Vec<FwResult<()>> {
        let mut targets = Vec::new();
        for entry in self.name_to_notifier.iter() {
            targets.push(entry.key().clone());
        }

        let mut rst_list = Vec::<FwResult<()>>::with_capacity(self.name_to_notifier.len());
        for key in targets {
            if let Some((_, listener)) = self.name_to_notifier.remove(&key) {
                if let Some((group, srv)) = key.split_once(':') {
                    tracing::info!("unsubscribe watch, group={group}, srv={srv}");

                    rst_list.push(
                        self.naming_cli
                            .unsubscribe(srv.to_string(), Some(group.to_string()), vec![], listener)
                            .await
                            .map_err(|e| FwError::SdkError("nacos sdk unwatch", e.to_string())),
                    )
                }
            }
        }

        rst_list
    }
}

pub fn get_addr_by_http(ins: &naming::ServiceInstance) -> FwResult<String> {
    _get_addr(ins, "http")
}

pub fn get_addr_by_rpc(ins: &naming::ServiceInstance) -> FwResult<String> {
    _get_addr(ins, "rpc")
}

fn _get_addr(ins: &naming::ServiceInstance, addr_type: &str) -> FwResult<String> {
    let attr = if addr_type == "http" {
        "http_port"
    } else {
        "rpc_port"
    };

    ins.metadata
        .get(attr)
        .map(|port| format!("http://{}:{}", ins.ip, port))
        .ok_or_else(|| {
            FwError::ResultError(format!(
                "can not found attribute value in metadata for {}",
                attr
            ))
        })
}
