use crate::config::router_config::{RouterConfig, TimeoutItem};
use crate::proxy::HttpServerProxy;
use crate::router::GatewayRouter;
use fw_base::parse_yaml_in_fw;
use fw_boot::state::RunState;
use fw_error::FwResult;
use fw_regdis::nacos::discovery;
use nacos_sdk::api::config::{ConfigChangeListener, ConfigResponse};
use nacos_sdk::api::naming::{NamingChangeEvent, NamingEventListener};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex, Weak};
use tokio::runtime::Handle;

pub const ROUTER_TABLE: &'static str = "router-tables.yaml";

pub struct GwState {
    rs: Arc<RunState>,

    router: Arc<GatewayRouter>,

    rt_handle: Handle,

    cfg_group: String,

    dis_group: String,

    // 当前路由表中的所有服务
    last_servers: Mutex<HashSet<String>>,

    // 真正已经启动 Nacos 监听的服务
    had_watched_servers: Mutex<HashSet<String>>,

    // 桥接 &self 到 Arc<Self>
    me: Mutex<Weak<GwState>>,
}

impl GwState {
    pub fn new(
        rs: Arc<RunState>,
        router: Arc<GatewayRouter>,
        rt_handle: Handle,
        dis_group: &str,
        cfg_group: &str,
    ) -> Arc<Self> {
        let state = Arc::new(Self {
            rs,
            router,
            rt_handle,
            dis_group: dis_group.to_string(),
            cfg_group: cfg_group.to_string(),
            last_servers: Mutex::new(Default::default()),
            had_watched_servers: Mutex::new(Default::default()),
            me: Mutex::new(Weak::new()),
        });

        // 建立弱引用闭环
        *state.me.lock().unwrap() = Arc::downgrade(&state);
        state
    }

    /// 辅助方法：从回调的 &self 中找回 Arc 身份
    fn arc_self(&self) -> Arc<Self> {
        self.me
            .lock()
            .unwrap()
            .upgrade()
            .expect("GwState should be alive")
    }

    /// 核心入口：向 Nacos 注册配置文件的监听器
    pub async fn listen_router_table(self: Arc<Self>) {
        let self_clone = self.clone();
        let group = self.cfg_group.clone();
        let rt_handle = self.rt_handle.clone();

        rt_handle.spawn(async move {
            let res = self_clone
                .rs
                .nacos_proxy()
                .get_nacos_configure()
                .add_config_listener(ROUTER_TABLE.to_string(), group, self_clone.clone())
                .await;

            if let Err(fe) = res {
                tracing::error!("listener {ROUTER_TABLE} failed in nacos, err={fe}");
            }
        });
    }

    /// 初始化加载：开机时的第一次同步拉取
    pub async fn init_load(self: Arc<Self>, router_table: RouterConfig) -> FwResult<()> {
        let mut preload_proxies = Vec::with_capacity(router_table.table.len());
        let mut srv_to_timeouts = HashMap::with_capacity(router_table.table.len());
        let mut servers = HashSet::new();

        for (srv_name, item) in router_table.table {
            let instances = self
                .rs
                .nacos_proxy()
                .get_nacos_discover()
                .discover(srv_name.clone(), Some(self.dis_group.clone()))
                .await?;

            let addrs = instances
                .into_iter()
                .filter_map(|ins| discovery::get_addr_by_http(&ins, false).ok())
                .collect();

            let proxy = HttpServerProxy::new(&srv_name, false, addrs)?;

            let timeout_item_fallback = || {
                if let Some(item) = item.timeout_config {
                    if item.read_timeout.as_nanos() == 0 {
                        router_table.default_timeout_config.clone()
                    } else {
                        item
                    }
                } else {
                    router_table.default_timeout_config.clone()
                }
            };

            srv_to_timeouts.insert(srv_name.clone(), timeout_item_fallback());

            preload_proxies.push(proxy);
            servers.insert(srv_name);
        }

        self.router.replace_all(preload_proxies, srv_to_timeouts);

        let self_clone = self.clone();

        {
            let mut last_guard = self.last_servers.lock().unwrap();
            *last_guard = servers.clone();
            let to_add: Vec<String> = servers.into_iter().collect();
            // 初始化完成后，启动对这些服务实例的监听
            self_clone.manage_server_watchers(to_add, vec![]).await;
        }
        Ok(())
    }

    /// 生命周期管理：增量处理 Nacos 服务订阅/退订
    pub async fn manage_server_watchers(
        self: Arc<Self>,
        to_add: Vec<String>,
        to_remove: Vec<String>,
    ) {
        let mut watch_guard = self.had_watched_servers.lock().unwrap();

        // 1. 处理移除的服务
        for srv in to_remove {
            if watch_guard.remove(&srv) {
                let rs = self.rs.clone();
                let group = self.dis_group.clone();
                tracing::warn!("server {} had be removed, starting unwatch", srv);
                let _ = rs
                    .nacos_proxy()
                    .get_nacos_discover()
                    .unwatch(srv, Some(group))
                    .await;
            }
        }

        // 2. 处理新增的服务
        for srv in to_add {
            if watch_guard.insert(srv.clone()) {
                let self_clone = self.clone();
                let group = self.dis_group.clone();
                let srv_name = srv.clone();

                tracing::info!("server {} instance watcher created", srv_name);
                let res = self_clone
                    .rs
                    .nacos_proxy()
                    .get_nacos_discover()
                    .add_naming_listener(srv_name, Some(group), self_clone)
                    .await;

                if let Err(e) = res {
                    tracing::error!("nacos naming subscribe failed: {e}");
                }
            }
        }
    }

    pub fn get_rs(&self) -> Arc<RunState> {
        self.rs.clone()
    }
}

/// 处理配置变更 (router_tables.yaml 发生变化)
impl ConfigChangeListener for GwState {
    fn notify(&self, config_resp: ConfigResponse) {
        let content = config_resp.content();

        tracing::info!(
            "router table content had be changed, content=\n{:#?}",
            content
        );

        let group_for_async = self.dis_group.clone();
        let rs = self.rs.clone();
        let arc_self = self.arc_self();

        let router_table = match parse_yaml_in_fw::<RouterConfig>(content) {
            Ok(cfg) => cfg,
            Err(e) => {
                tracing::error!("parse router table failed, err={e}");
                return;
            }
        };

        // 2. 同步计算差异（逻辑保持不变）
        let (actual_add, actual_remove) = {
            let mut last_guard = arc_self.last_servers.lock().unwrap();
            let mut watch_guard = arc_self.had_watched_servers.lock().unwrap();

            let new_set: HashSet<String> = router_table.table.keys().cloned().collect();

            let to_add: Vec<String> = new_set.difference(&last_guard).cloned().collect();
            let to_remove: Vec<String> = last_guard.difference(&new_set).cloned().collect();

            if to_add.is_empty() && to_remove.is_empty() {
                return;
            }

            *last_guard = new_set;

            let filtered_add: Vec<String> = to_add
                .into_iter()
                .filter(|srv| watch_guard.insert(srv.clone()))
                .collect();
            let filtered_remove: Vec<String> = to_remove
                .into_iter()
                .filter(|srv| watch_guard.remove(srv))
                .collect();

            (filtered_add, filtered_remove)
        };

        let rt_handle = arc_self.rt_handle.clone();

        rt_handle.spawn(async move {
            // A. 处理移除
            for srv in actual_remove {
                tracing::warn!("service {} removed, starting unwatch", srv);
                arc_self.router.remove_proxy(&srv);
                let _ = rs
                    .nacos_proxy()
                    .get_nacos_discover()
                    .unwatch(srv, Some(group_for_async.clone())) // 使用 group_for_async
                    .await;
            }

            // B. 处理新增
            for srv in actual_add {
                tracing::info!("server {} added, starting fetch and watch", srv);

                // 初始拉取
                match rs
                    .nacos_proxy()
                    .get_nacos_discover()
                    .discover(srv.clone(), Some(group_for_async.clone()))
                    .await
                {
                    Ok(instances) => {
                        let addrs = instances
                            .into_iter()
                            .filter_map(|ins| discovery::get_addr_by_http(&ins, false).ok())
                            .collect();
                        if let Ok(proxy) = HttpServerProxy::new(&srv, false, addrs) {
                            arc_self.router.add_proxy(proxy);
                        }
                    }
                    Err(fe) => {
                        tracing::warn!("server {srv} fetch failed after added, err={fe}");
                    }
                }

                match rs // 使用之前 clone 的 rs
                    .nacos_proxy()
                    .get_nacos_discover()
                    .add_naming_listener(
                        srv.clone(),
                        Some(group_for_async.clone()),
                        arc_self.clone(),
                    )
                    .await
                {
                    Ok(_) => {}
                    Err(fe) => {
                        tracing::error!("server {srv} watch failed after added, err={fe}");
                        arc_self.had_watched_servers.lock().unwrap().remove(&srv);
                    }
                }
            }
        });
    }
}

/// 处理服务实例变更 (微服务上下线)
impl NamingEventListener for GwState {
    fn event(&self, event: Arc<NamingChangeEvent>) {
        let srv_name = event.service_name.clone();
        let addrs: HashSet<String> = event
            .instances
            .as_ref()
            .unwrap_or(&vec![])
            .iter()
            .filter(|ins| ins.healthy && ins.enabled)
            .filter_map(|ins| discovery::get_addr_by_http(ins, false).ok())
            .collect();

        tracing::info!(
            "server {} instances had be changed, instances={:#?}",
            srv_name,
            addrs
        );

        if let Some(proxy) = self.router.get_proxy(&srv_name) {
            if let Err(fe) = proxy.replace_all(addrs) {
                tracing::error!("replace proxy instances failed for {}, err={fe}", srv_name);
            }
        }
    }
}
