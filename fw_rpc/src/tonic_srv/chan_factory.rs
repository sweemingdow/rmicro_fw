use fw_error::result::FwResult;
use fw_regdis::nacos::discovery;
use fw_regdis::nacos::proxy::NacosProxy;
use singleflight_async::SingleFlight;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex, RwLock};
use tokio::sync::mpsc::Sender;
use tonic::transport::{Channel, Endpoint};
use tower::discover::Change;

pub struct RpcChanFactory {
    // 缓存已经创建好的负载均衡 Channel
    name_to_channels: Arc<RwLock<HashMap<String, Channel>>>,

    // 影子地址池：缓存每个服务当前已知的地址列表，用于对比增量变化
    name_to_addrs: Arc<Mutex<HashMap<String, HashSet<String>>>>,

    nacos_proxy: Arc<NacosProxy>,

    // 服务发现所需的组名
    dis_group_name: String,

    sf_group: SingleFlight<String, FwResult<Channel>>,
}

impl RpcChanFactory {
    pub fn new(dis_group_name: &str, nacos_proxy: Arc<NacosProxy>) -> Self {
        Self {
            name_to_channels: Arc::new(RwLock::new(HashMap::new())),
            name_to_addrs: Arc::new(Mutex::new(HashMap::new())),
            nacos_proxy,
            dis_group_name: dis_group_name.to_string(),
            sf_group: SingleFlight::new(),
        }
    }

    pub async fn acquire_chan(&self, srv_name: &str) -> FwResult<Channel> {
        {
            if let Some(channel) = self.name_to_channels.read().unwrap().get(srv_name) {
                return Ok(channel.clone());
            }
        }

        self.sf_group
            .work(srv_name.to_string(), move || async move {
                self.do_init_chan(srv_name).await
            })
            .await
    }

    async fn do_init_chan(&self, srv_name: &str) -> FwResult<Channel> {
        // 主动从Nacos拉取一次全量地址
        let initial_addrs = self.select_once(srv_name).await?;

        let (channel, tx) = Channel::balance_channel::<String>(64);
        let mut current_set = HashSet::new();

        for addr in initial_addrs {
            if let Ok(endpoint) = Endpoint::from_shared(addr.clone()) {
                let endpoint = endpoint.tcp_keepalive(Some(std::time::Duration::from_secs(60)));
                let _ = tx.send(Change::Insert(addr.clone(), endpoint)).await;
                current_set.insert(addr);
            }
        }

        // 监听对应服务变化
        if let Err(e) = self.watch(srv_name, tx).await {
            tracing::warn!(
                "watch failed for {srv_name}, dis_group={}",
                self.dis_group_name
            )
        }

        {
            let mut write_guard = self.name_to_channels.write().unwrap();
            // Double check
            if let Some(existing) = write_guard.get(srv_name) {
                return Ok(existing.clone());
            }

            // 更新影子地址
            {
                let mut addr_guard = self.name_to_addrs.lock().unwrap();
                addr_guard.insert(srv_name.to_string(), current_set);
            }

            write_guard.insert(srv_name.to_string(), channel.clone());
        }

        Ok(channel)
    }

    async fn select_once(&self, srv_name: &str) -> FwResult<Vec<String>> {
        let ins_list = self
            .nacos_proxy
            .discover(srv_name.to_string(), Some(self.dis_group_name.clone()))
            .await?;

        let mut result = Vec::with_capacity(ins_list.len());
        for ins in &ins_list {
            let addr = discovery::get_addr_by_rpc(ins)?;
            result.push(addr);
        }

        Ok(result)
    }

    async fn watch(&self, srv_name: &str, tx: Sender<Change<String, Endpoint>>) -> FwResult<()> {
        let srv_name_inner = srv_name.to_string();
        let addr_map_arc = self.name_to_addrs.clone();

        self.nacos_proxy
            .watch_then_save(
                srv_name.to_string(),
                Some(self.dis_group_name.clone()),
                move |event| {
                    tracing::info!(
                        srv_name= event.service_name ,
                        instances =?event.instances,
                        "server instances had be changed");

                    let mut addr_guard = addr_map_arc.lock().unwrap();
                    let current_set = addr_guard.entry(srv_name_inner.clone()).or_default();

                    let mut new_addrs = HashSet::new();
                    if let Some(instances) = &event.instances {
                        for ins in instances {
                            if let Some(addr) = discovery::get_addr_by_rpc(ins).ok() {
                                new_addrs.insert(addr);
                            }
                        }
                    }

                    // 需要下线的实例
                    for old_addr in current_set.iter() {
                        if !new_addrs.contains(old_addr) {
                            let _ = tx.try_send(Change::Remove(old_addr.clone()));
                        }
                    }

                    // 需要上线的实例
                    for new_addr in new_addrs.iter() {
                        if !current_set.contains(new_addr) {
                            if let Ok(endpoint) = Endpoint::from_shared(new_addr.clone()) {
                                let _ = tx.try_send(Change::Insert(new_addr.clone(), endpoint));
                            }
                        }
                    }

                    *current_set = new_addrs;
                },
            )
            .await
    }
}
