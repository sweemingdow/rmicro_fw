use crate::background::BackgroundRunner;
use crate::config::router_config::RouterConfig;
use crate::ext::GatewayHookExt;
use crate::ext::config_ext::GatewayRunConfigExt;
use crate::router::{GatewayRouter, GatewayRouterExt};
use crate::state::{GwState, ROUTER_TABLE};
use fw_base::{parse_yaml_in_fw, set_gw_dispatch_val};
use fw_boot::App;
use fw_boot::state::RunState;
use fw_error::{FwError, FwResult};
use fw_regdis::nacos::registry::{NacosRegister, RegisterOptions};
use pingora_core::prelude::background_service;
use pingora_core::server::Server;
use pingora_core::server::configuration::ServerConf;
use pingora_proxy::http_proxy_service;
use std::sync::Arc;
use tokio::runtime::Runtime;

pub struct GatewayRunner {
    _rt: Runtime,

    rs: Arc<RunState>,

    app: Arc<App>,
}

impl GatewayRunner {
    pub fn new() -> FwResult<(Self, Arc<GatewayRouter>)> {
        let app = Arc::new(App::new()?);
        let app_clone = app.clone();
        let rt = Self::init_rt()?;
        let rt_handle = rt.handle().clone(); // 克隆 Handle (内含 Arc)

        let config_group = app.get_cfg().nacos_center_cfg.config.group_name.clone();
        let dis_group = app.get_cfg().nacos_center_cfg.registry.group_name.clone();

        // 在 Runtime 中初始化组件
        let (gs, router) = rt.block_on(async {
            let rs = Arc::new(app.clone().new_run_state().await?);

            // 1. 获取初始配置
            let router_table =
                Self::fetch_and_parse_router_table(rs.clone(), &config_group).await?;

            // 2. 构造路由组件
            let router = Arc::new(GatewayRouter::new(router_table.extract_depth));

            // 3. 构造状态管理器
            let gs = GwState::new(
                rs.clone(),
                router.clone(),
                rt_handle,
                &dis_group,
                &config_group,
            );

            // 4. 执行初次加载（发现所有服务）
            gs.clone().init_load(router_table).await?;

            // 5. 启动配置监听 (异步)
            gs.clone().listen_router_table().await;

            // 6. 将自身注册到nacos
            app.register_to_nacos(rs.nacos_proxy()).await?;

            Ok::<(Arc<GwState>, Arc<GatewayRouter>), FwError>((gs.clone(), router.clone()))
        })?;

        Ok((
            Self {
                _rt: rt,
                rs: gs.get_rs(),
                app: app_clone,
            },
            router,
        ))
    }

    async fn fetch_and_parse_router_table(
        rs: Arc<RunState>,
        dis_group: &str,
    ) -> FwResult<RouterConfig> {
        let router_resp = rs
            .nacos_proxy()
            .fetch_config(ROUTER_TABLE.to_string(), dis_group.to_string())
            .await?;

        // 解析
        Ok(parse_yaml_in_fw::<RouterConfig>(router_resp.content())?)
    }

    fn init_rt() -> FwResult<Runtime> {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1) // 你的 mini-rt 需求
            .enable_all()
            .thread_name("gw-rt")
            .build()
            .map_err(|e| FwError::InitError("gw-rt", e.to_string()))
    }
}

impl GatewayRunner {
    pub fn get_rs(&self) -> Arc<RunState> {
        self.rs.clone()
    }

    pub fn get_app(&self) -> Arc<App> {
        self.app.clone()
    }

    pub fn execute<F, Fut, T>(&self, action: F) -> FwResult<T>
    where
        F: FnOnce() -> Fut + Send + 'static,
        Fut: Future<Output = FwResult<T>> + Send + 'static,
        T: Send + 'static,
    {
        self._rt.block_on(async move { action().await })
    }
}

impl GatewayRunner {
    pub fn run<E, C>(&self, cfg_ext: C, router: Arc<GatewayRouter>, hook_ext: E) -> FwResult<()>
    where
        E: GatewayHookExt + 'static,
        C: GatewayRunConfigExt,
    {
        let gw_config = cfg_ext.get_gateway_server_config();
        let srv_cfg = {
            let mut srv_cfg = ServerConf::default();
            srv_cfg.threads = gw_config.worker_count as usize;
            srv_cfg.upstream_keepalive_pool_size = gw_config.conn_pool_size as usize;
            srv_cfg.grace_period_seconds = Some(gw_config.grace_period_timeout.as_secs());
            srv_cfg.graceful_shutdown_timeout_seconds =
                Some(gw_config.graceful_shutdown_timeout.as_secs());

            srv_cfg
        };

        set_gw_dispatch_val(&cfg_ext.get_gw_dispatch_cfg().dispatch_val)?;

        tracing::info!("pingora server config, cfg=\n{:#?}", srv_cfg);

        // 创建 Pingora 服务器
        let mut gw_server = Server::new_with_opt_and_conf(None, srv_cfg);
        gw_server.bootstrap();

        let router_ext = GatewayRouterExt::new(router, hook_ext);
        let mut proxy_service = http_proxy_service(&gw_server.configuration, router_ext);

        proxy_service.add_tcp(&format!(
            "{}:{}",
            gw_config.listen_addr, gw_config.listen_port
        ));

        gw_server.add_service(proxy_service);

        let bg_runner = background_service(
            "BgRunner",
            BackgroundRunner::new(self.rs.clone(), self.app.clone()),
        );

        gw_server.add_service(bg_runner);

        gw_server.run_forever();
    }
}
