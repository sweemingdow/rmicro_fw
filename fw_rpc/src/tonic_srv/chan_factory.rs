use fw_error::FwError;
use fw_error::result::FwResult;
use fw_regdis::nacos::discovery;
use fw_regdis::nacos::proxy::NacosProxy;
use singleflight_async::SingleFlight;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;
use tokio::sync::mpsc::Sender;
use tokio::task::JoinSet;
use tonic::transport::{Channel, Endpoint};
use tower::discover::Change;
use fw_base::configuration::static_config::RpcChannelConfig;

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
    pub fn new(dis_group_name: &str, nacos_proxy: Arc<NacosProxy>) -> Arc<Self> {
        Arc::new(Self {
            name_to_channels: Arc::new(RwLock::new(HashMap::new())),
            name_to_addrs: Arc::new(Mutex::new(HashMap::new())),
            nacos_proxy,
            dis_group_name: dis_group_name.to_string(),
            sf_group: SingleFlight::new(),
        })
    }

    pub async fn with_preload(
        dis_group_name: &str,
        nacos_proxy: Arc<NacosProxy>,
        // 传入引用，内部克隆 key
        srv_name_to_config: &HashMap<String, RpcChannelConfig>,
    ) -> FwResult<(Arc<Self>, HashMap<String, FwResult<Channel>>)> {
        let _self = Self::new(dis_group_name, nacos_proxy);
        let mut set = JoinSet::new();

        for (srv_name, cfg) in srv_name_to_config {
            let _self_clone = _self.clone();
            let srv_name_owned = srv_name.clone();
            let cfg_owned = cfg.clone(); // 确保 cfg 可克隆

            set.spawn(async move {
                let res = _self_clone
                    .do_init_chan(&srv_name_owned, Some(cfg_owned.into()))
                    .await;
                (srv_name_owned, res)
            });
        }

        let mut srv_name_to_fr = HashMap::with_capacity(srv_name_to_config.len());

        // 并发收集结果
        while let Some(res) = set.join_next().await {
            match res {
                Ok((name, init_res)) => {
                    srv_name_to_fr.insert(name, init_res);
                }
                Err(e) => {
                    // 这里处理 JoinSet 内部产生的 Panic
                    srv_name_to_fr.insert(
                        "unknown_task".to_string(),
                        Err(FwError::InitError("rpc preload panicked", e.to_string())),
                    );
                }
            }
        }

        Ok((_self, srv_name_to_fr))
    }

    pub async fn with_preload_then_log(
        dis_group_name: &str,
        nacos_proxy: Arc<NacosProxy>,
        srv_name_to_config: &HashMap<String, RpcChannelConfig>,
    ) -> FwResult<Arc<Self>> {
        match Self::with_preload(dis_group_name, nacos_proxy, srv_name_to_config).await {
            Ok((factory, results)) => {
                for (srv_name, fr) in results {
                    if let Err(e) = fr {
                        tracing::error!("preload callee={}, failed, err={}", srv_name, e)
                    }
                }
                Ok(factory)
            }

            Err(e) => Err(e),
        }
    }

    pub async fn acquire_chan(
        &self,
        srv_name: &str,
        ch_ops: Option<RpcChannelOptions>,
    ) -> FwResult<Channel> {
        {
            if let Some(channel) = self.name_to_channels.read().unwrap().get(srv_name) {
                return Ok(channel.clone());
            }
        }

        self.sf_group
            .work(srv_name.to_string(), move || async move {
                self.do_init_chan(srv_name, ch_ops).await
            })
            .await
    }

    async fn do_init_chan(
        &self,
        srv_name: &str,
        ch_ops: Option<RpcChannelOptions>,
    ) -> FwResult<Channel> {
        tracing::debug!(
            "init rpc channel factory, srv_name={}, ops:\n{:#?}",
            srv_name,
            ch_ops
        );

        let ops = ch_ops.unwrap_or(RpcChannelOptions::default());
        // 主动从Nacos拉取一次全量地址
        let initial_addrs = self.select_once(srv_name).await?;

        if initial_addrs.is_empty(){
            // todo
        }


        let (channel, tx) =
            Channel::balance_channel::<String>(ops.get_estimate_srv_max_count() as usize);

        let mut current_set = HashSet::new();

        for addr in initial_addrs {
            let ops_clone_inner = ops.clone();
            if let Ok(endpoint) = Endpoint::from_shared(addr.clone()) {
                let endpoint = Self::bind_param_to_endpoint(endpoint, ops_clone_inner);

                let _ = tx.send(Change::Insert(addr.clone(), endpoint)).await;
                current_set.insert(addr);
            }
        }

        // 监听对应服务变化
        if let Err(e) = self.watch(srv_name, tx, ops).await {
            tracing::warn!(
                "watch failed for {srv_name}, dis_group={}, err={}",
                self.dis_group_name,
                e
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

    async fn watch(
        &self,
        srv_name: &str,
        tx: Sender<Change<String, Endpoint>>,
        ops: RpcChannelOptions,
    ) -> FwResult<()> {
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
                                let endpoint = Self::bind_param_to_endpoint(endpoint, ops.clone());

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
impl RpcChanFactory {
    fn bind_param_to_endpoint(endpoint: Endpoint, ops: RpcChannelOptions) -> Endpoint {
        endpoint
            .connect_timeout(ops.get_connect_timeout())
            .timeout(ops.get_request_timeout())
            .tcp_keepalive(ops.get_tcp_keepalive())
            .keep_alive_timeout(ops.get_keep_alive_timeout())
            .http2_keep_alive_interval(ops.get_http2_keep_alive_interval())
    }
}

#[derive(Debug, Default, Clone)]
pub struct RpcChannelOptions {
    pub estimate_srv_max_count: Option<u16>, // 预估服务最多数量

    pub connect_timeout: Option<Duration>, // 连接超时

    pub request_timeout: Option<Duration>, // 请求总超时

    pub keep_alive_timeout: Option<Duration>, // 空闲连接超时

    pub tcp_keepalive: Option<Duration>, // TCP keepalive

    pub http2_keep_alive_interval: Option<Duration>, // HTTP2 ping超时
}

impl RpcChannelOptions {
    const DEFAULT_ESTIMATE_SRV_MAX_COUNT: u16 = 2;
    const DEFAULT_CONNECT_TIMEOUT: Duration = Duration::from_millis(800);
    const DEFAULT_REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
    const DEFAULT_KEEP_ALIVE_TIMEOUT: Duration = Duration::from_secs(30);
    const DEFAULT_TCP_KEEPALIVE: Duration = Duration::from_secs(60);
    const DEFAULT_HTTP2_KEEP_ALIVE_INTERVAL: Duration = Duration::from_secs(30);
}

impl RpcChannelOptions {
    pub fn get_estimate_srv_max_count(&self) -> u16 {
        self.estimate_srv_max_count
            .unwrap_or(Self::DEFAULT_ESTIMATE_SRV_MAX_COUNT)
    }

    pub fn get_connect_timeout(&self) -> Duration {
        self.connect_timeout
            .unwrap_or(Self::DEFAULT_CONNECT_TIMEOUT)
    }

    pub fn get_request_timeout(&self) -> Duration {
        self.request_timeout
            .unwrap_or(Self::DEFAULT_REQUEST_TIMEOUT)
    }

    pub fn get_keep_alive_timeout(&self) -> Duration {
        self.keep_alive_timeout
            .unwrap_or(Self::DEFAULT_KEEP_ALIVE_TIMEOUT)
    }

    pub fn get_tcp_keepalive(&self) -> Option<Duration> {
        self.tcp_keepalive.or(Some(Self::DEFAULT_TCP_KEEPALIVE))
    }

    pub fn get_http2_keep_alive_interval(&self) -> Duration {
        self.http2_keep_alive_interval
            .unwrap_or(Self::DEFAULT_HTTP2_KEEP_ALIVE_INTERVAL)
    }
}

impl From<RpcChannelConfig> for RpcChannelOptions {
    fn from(cfg: RpcChannelConfig) -> Self {
        Self {
            estimate_srv_max_count: cfg.estimate_srv_max_count,
            connect_timeout: cfg.connect_timeout,
            request_timeout: cfg.request_timeout,
            keep_alive_timeout: cfg.keep_alive_timeout,
            tcp_keepalive: cfg.tcp_keepalive,
            http2_keep_alive_interval: cfg.http2_keep_alive_interval,
        }
    }
}
