use crate::nacos::client::NacosClient;
use dashmap::DashMap;
use fw_error::lib_error::FwError;
use fw_error::result::FwResult;
use nacos_sdk::api::naming;
use std::sync::Arc;

pub struct NacosDiscovery {
    naming_cli: naming::NamingService,
    // 统一存储，Key 为 "group:service_name"
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

    /// 原有的基础查询逻辑
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

    // --- 新增：像 NacosConfigure 一样的 Listener 风格接口 ---

    /// 核心方法：添加自定义监听器实例
    pub async fn add_naming_listener<L>(
        &self,
        service_name: String,
        group_name: Option<String>,
        listener: Arc<L>,
    ) -> FwResult<()>
    where
        L: naming::NamingEventListener + 'static,
    {
        let group = group_name.unwrap_or_else(|| "DEFAULT_GROUP".to_string());
        let key = format!("{}:{}", group, service_name);

        self.name_to_notifier.insert(key, listener.clone());

        self.naming_cli
            .subscribe(service_name, Some(group), Vec::new(), listener)
            .await
            .map_err(|e| FwError::SdkError("nacos sdk add naming listener", e.to_string()))
    }

    /// 取消单个服务的监听
    pub async fn unwatch(&self, service_name: String, group_name: Option<String>) -> FwResult<()> {
        let group = group_name.unwrap_or_else(|| "DEFAULT_GROUP".to_string());
        let key = format!("{}:{}", group, service_name);

        if let Some((_, listener)) = self.name_to_notifier.remove(&key) {
            tracing::info!("unsubscribe watch, group={group}, srv={service_name}");
            self.naming_cli
                .unsubscribe(service_name, Some(group), vec![], listener)
                .await
                .map_err(|e| FwError::SdkError("nacos sdk unwatch", e.to_string()))
        } else {
            Ok(())
        }
    }

    // --- 保留原闭包方式 ---

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
        // 直接复用 add_naming_listener
        self.add_naming_listener(service_name, group_name, notifier)
            .await
    }

    /// 全量清理
    pub async fn unwatch_all(&self) -> Vec<FwResult<()>> {
        let targets: Vec<String> = self
            .name_to_notifier
            .iter()
            .map(|e| e.key().clone())
            .collect();

        let mut rst_list = Vec::with_capacity(targets.len());
        for key in targets {
            if let Some((data_id, group)) = key.split_once(':') {
                rst_list.push(
                    self.unwatch(group.to_string(), Some(data_id.to_string()))
                        .await,
                );
            }
        }
        rst_list
    }
}

// 地址辅助函数保持不变...
pub fn get_addr_by_http(ins: &naming::ServiceInstance, need_schema: bool) -> FwResult<String> {
    _get_addr(ins, "http", need_schema)
}

pub fn get_addr_by_rpc(ins: &naming::ServiceInstance) -> FwResult<String> {
    _get_addr(ins, "rpc", true)
}

fn _get_addr(
    ins: &naming::ServiceInstance,
    addr_type: &str,
    need_schema: bool,
) -> FwResult<String> {
    let attr = if addr_type == "http" {
        "http_port"
    } else {
        "rpc_port"
    };

    ins.metadata
        .get(attr)
        .map(|port| {
            if need_schema {
                format!("http://{}:{}", ins.ip, port)
            } else {
                format!("{}:{}", ins.ip, port)
            }
        })
        .ok_or_else(|| {
            FwError::ResultError(format!(
                "can not found attribute value in metadata for {}",
                attr
            ))
        })
}
