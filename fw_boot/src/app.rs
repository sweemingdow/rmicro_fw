use crate::config::Config;
use crate::ext::RunConfigExt;
use crate::state::RunState;
use crate::{BootChain, graceful};
use fw_base::{init_pass_strategy, my_utils, set_gw_dispatch_val};
use fw_error::lib_error::FwError;
use fw_error::recorder;
use fw_error::result::FwResult;
use fw_log::my_log;
use fw_regdis::nacos::proxy::NacosProxy;
use fw_regdis::nacos::registry::{DeregisterOptions, NacosRegister, RegisterOptions};
use nacos_sdk::api::config::ConfigChangeListener;
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::{Arc, Mutex};
use std::{env, time};
use tokio_util::sync;
use tracing::Instrument;
use tracing_appender::non_blocking;

const DEFAULT_STOP_TIMEOUT: time::Duration = time::Duration::from_secs(30);
const DEFAULT_CLEAN_TIMEOUT: time::Duration = time::Duration::from_secs(10);
const DEFAULT_STOP_STAGES: u8 = 2;
const DEFAULT_STOP_STAGE_TIMEOUT: time::Duration = time::Duration::from_secs(10);

#[derive(Debug)]
pub struct App {
    cfg: Arc<Config>,

    app_name: String,

    // 本机Ip
    mip: String,

    http_port: u16,

    http_server_addr: String,

    rpc_port: u16,

    rpc_server_addr: String,

    profile: String,

    cancel_token: sync::CancellationToken,

    _log_guard: Mutex<Option<non_blocking::WorkerGuard>>, // more
}

impl App {
    pub fn new() -> FwResult<Self> {
        // 解析配置文件
        let cfg = Arc::new(Config::from_env()?);

        let app_name = cfg.app_cfg.app_name.clone();
        let profile = cfg.app_cfg.profile.clone();
        let http_port = Self::reselect_http_port(cfg.app_cfg.http_port);
        let rpc_port = Self::reselect_rpc_port(cfg.app_cfg.rpc_port);

        recorder::init_project_name(app_name.clone());

        let log_guard = my_log::init_logger(my_log::LogOptions {
            max_log_files: cfg.log_cfg.max_log_files,
            log_dir: cfg.log_cfg.log_dir.to_string(),
            app_name: app_name.clone(),
            port: http_port,
            thread_name: cfg.log_cfg.thread_name,
            thread_id: cfg.log_cfg.thread_id,
        });

        tracing::debug!("parse configuration completed, cfg=\n{:#?}", cfg);

        let mip = my_utils::get_machine_ip();

        Ok(Self {
            cfg,
            app_name,
            http_port,
            http_server_addr: format!("{}:{}", mip, http_port),
            rpc_port,
            rpc_server_addr: format!("{}:{}", mip, rpc_port),
            mip,
            profile,
            cancel_token: sync::CancellationToken::new(),
            _log_guard: Mutex::new(Some(log_guard)),
        })
    }
}

impl App {
    pub async fn prepare<ST: RunConfigExt>(self: Arc<Self>) -> FwResult<(Arc<RunState>, ST)>
    where
        ST: serde::de::DeserializeOwned + Debug,
    {
        let rs = Arc::new(self.clone().new_run_state().await?);

        let config_group = &self.cfg.nacos_center_cfg.config.group_name;

        let static_cfg: ST = rs
            .nacos_proxy()
            .get_nacos_configure()
            .fetch_static_config(config_group)
            .await?;

        set_gw_dispatch_val(&static_cfg.get_gw_dispatch_cfg().dispatch_val)?;

        let strategy = &static_cfg
            .get_gw_dispatch_cfg()
            .pass_strategy
            .clone()
            .unwrap_or("".to_string());
        init_pass_strategy(strategy)?;

        tracing::debug!(
            "fetch and parse static config completed, static_cfg=\n{:#?}",
            static_cfg
        );

        Ok((rs, static_cfg))
    }

    pub async fn prepare_with_dynamic<ST: RunConfigExt, DT, L>(
        self: Arc<Self>,
        dynamic_listener: Arc<L>,
    ) -> FwResult<(Arc<RunState>, ST, DT)>
    where
        ST: serde::de::DeserializeOwned + Debug + Send + 'static,
        DT: serde::de::DeserializeOwned + Debug + Send + 'static,
        L: ConfigChangeListener + 'static,
    {
        let rs = Arc::new(self.clone().new_run_state().await?);

        let config_group = self.cfg.nacos_center_cfg.config.group_name.clone();
        let configure = rs.nacos_proxy().get_nacos_configure();

        // 使用 try_join!，只要有一个失败就立即返回错误
        let (static_cfg, dynamic_cfg) = tokio::try_join!(
            configure.fetch_static_config::<ST>(&config_group),
            configure.fetch_dynamic_config::<DT>(&config_group)
        )?;

        set_gw_dispatch_val(&static_cfg.get_gw_dispatch_cfg().dispatch_val)?;

        tracing::debug!(
            "fetch and parse static config completed, content=\n{:#?}",
            static_cfg
        );
        tracing::debug!(
            "fetch and parse dynamic config completed, content=\n{:#?}",
            dynamic_cfg
        );

        configure
            .listen_dynamic_config(&config_group, dynamic_listener)
            .await?;

        Ok((rs, static_cfg, dynamic_cfg))
    }
}

impl App {
    async fn run<F1, Fut1, F2, Fut2>(
        self: Arc<Self>,
        rs: Arc<RunState>,
        run_action: F1,
        clean_action: F2,
    ) -> FwResult<()>
    where
        F1: FnOnce() -> Fut1 + Send + 'static,
        Fut1: Future<Output = FwResult<()>> + Send,
        F2: FnOnce() -> Fut2 + Send + 'static,
        Fut2: Future<Output = ()> + Send,
    {
        // 创建root span
        let root_span = self.clone().create_root_span();

        self.check_timeout_for_stop()?;

        let self_clone = self.clone();
        let stop_timeout = self.get_stop_timeout();
        let clean_timeout = self.get_clean_timeout();
        let lis_span = root_span.clone();
        let runner = async move {
            // 初始化RunState
            // let rs = Arc::new(
            //     RunState::new(self_clone.cfg.clone(), self_clone.cancel_token.clone()).await?,
            // );
            //
            let rs_clone = rs.clone();
            let self_in_lis = self_clone.clone();
            tokio::spawn(async move {
                // 监听并退出
                self_in_lis
                    .clone()
                    .listen_then_exit(rs_clone)
                    .instrument(lis_span)
                    .await;
            });

            let self_in_stop = self_clone.clone();
            let res = tokio::select! {
                // 业务自身响应cancellation token或自身退出
                r = run_action() => {
                    tracing::info!("run_action shutdown completed gracefully");
                    r
                }

                // 响应了中断signal后, shutdown超时了
                _ = async {
                    // 先等待信号被触发（不管是 Ctrl+C 还是其他地方调了 cancel）
                    self_in_stop.cancel_token.cancelled().await;

                    tracing::debug!("run_action start shutdown");

                    tokio::time::sleep(stop_timeout).await;
                    // 走到这, 已经run_action shutdown已经超时了
                } => {
                    Err(FwError::TimeoutError("run_action shutdown",format!("timeout after {:?}",stop_timeout)))
                }
            };

            // 确保停机信号已经触发或业务自然的结束
            if !self_clone.cancel_token.is_cancelled() {
                self_clone.cancel_token.cancel();
            }

            // 自定义清理
            let clean_task = async {
                clean_action().await;
            };

            if let Err(_) = tokio::time::timeout(clean_timeout, clean_task).await {
                tracing::error!("clean_action shutdown timeout after:{:?}", clean_timeout);
            }

            tracing::info!("application loop done, exit process!");

            res
        };

        runner.instrument(root_span).await?;

        // 显式刷新Log
        self.flush_log();

        Ok::<(), FwError>(())
    }

    fn flush_log(&self) {
        if let Ok(mut guard_opt) = self._log_guard.lock() {
            if let Some(guard) = guard_opt.take() {
                // tracing::info!("app is dropping, flushing logs...");
                drop(guard);
            }
        }
    }
}

impl App {
    pub async fn run_with<F, Fut, C, CFut>(
        self: Arc<Self>,
        rs: Arc<RunState>,
        build_chain: F,
        clean_action: C,
    ) -> FwResult<()>
    where
        F: FnOnce(BootChain) -> Fut + Send + 'static,
        Fut: Future<Output = BootChain> + Send,
        // 清理闭包
        C: FnOnce() -> CFut + Send + 'static,
        CFut: Future<Output = ()> + Send,
    {
        let stage_timeout = self.get_stage_timeout();
        let self_cp = self.clone();
        self.run(
            rs.clone(),
            move || async move {
                let chain = build_chain(BootChain::new(rs.cancel_token().clone())).await;

                chain
                    .run(stage_timeout, move || async move {
                        self_cp.register_to_nacos(rs.nacos_proxy()).await
                    })
                    .await
            },
            clean_action,
        )
        .await
    }
}

impl App {
    pub async fn register_to_nacos(&self, nacos_proxy: Arc<NacosProxy>) -> FwResult<()> {
        let mut meta_map = HashMap::with_capacity(3);
        meta_map.insert("http_port".to_string(), self.get_http_port().to_string());
        meta_map.insert("rpc_port".to_string(), self.get_rpc_port().to_string());
        meta_map.insert("source".to_string(), "rmicro-fw".to_string());

        let reg_ops = RegisterOptions {
            cluster_name: None,
            group_name: Some(self.cfg.nacos_center_cfg.registry.group_name.clone()),
            srv_name: self.app_name.clone(),
            addr: self.http_server_addr.clone(),
            weight: 10.0,
            meta_data: meta_map,
        };

        tracing::info!(
            "{} register to nacos, reg_ops={:#?}",
            self.app_name,
            reg_ops
        );

        nacos_proxy.register(reg_ops).await
    }

    pub async fn deregister_from_nacos(&self, nacos_proxy: Arc<NacosProxy>) -> FwResult<()> {
        let deg_ops = DeregisterOptions {
            group_name: Some(self.cfg.nacos_center_cfg.registry.group_name.clone()),
            srv_name: self.app_name.clone(),
            addr: self.http_server_addr.clone(),
            cluster_name: None,
        };

        tracing::info!(
            "{} deregister from nacos, deg_ops={:#?}",
            self.app_name,
            deg_ops
        );

        nacos_proxy.deregister(deg_ops).await
    }

    async fn unwatch_all(&self, nacos_proxy: Arc<NacosProxy>) -> Vec<FwResult<()>> {
        nacos_proxy.unwatch_all().await
    }

    async fn listen_then_exit(self: Arc<Self>, rs: Arc<RunState>) {
        graceful::listen_exit_signal(|sig| async move {
            // Step1: 注销自身, 切断流量
            match self.deregister_from_nacos(rs.nacos_proxy()).await {
                Ok(_) => {
                    tracing::debug!("deregistered from nacos successfully, with sig={:?}", sig)
                }
                Err(e) => tracing::error!(err_msg=?e,"deregistered from nacos failed"),
            }

            // 停止对其他服务的监听
            self.unwatch_all(rs.nacos_proxy()).await;

            // 分发停机的总信号
            tracing::info!("dispatch interrupt signal");

            self.cancel_token.clone().cancel();
        })
        .await;
    }
}

impl App {
    pub fn get_mip(&self) -> &str {
        &self.mip
    }

    pub fn get_app_name(&self) -> &str {
        &self.app_name
    }

    pub fn get_cfg(&self) -> Arc<Config> {
        self.cfg.clone()
    }

    pub fn get_reg_addr(&self) -> &str {
        &self.http_server_addr
    }

    pub fn get_http_port(&self) -> u16 {
        self.http_port
    }

    pub fn get_rpc_port(&self) -> u16 {
        self.rpc_port
    }

    pub fn get_http_addr(&self) -> &str {
        &self.http_server_addr
    }

    pub fn get_rpc_addr(&self) -> &str {
        &self.rpc_server_addr
    }

    pub fn get_profile(&self) -> &str {
        &self.profile
    }

    fn get_stop_timeout(&self) -> time::Duration {
        self.cfg
            .app_cfg
            .stop_timeout
            .unwrap_or(DEFAULT_STOP_TIMEOUT)
    }

    fn get_clean_timeout(&self) -> time::Duration {
        self.cfg
            .app_cfg
            .component_clean_timeout
            .unwrap_or(DEFAULT_CLEAN_TIMEOUT)
    }

    pub fn get_stage_timeout(&self) -> time::Duration {
        self.cfg
            .app_cfg
            .stage_stop_timeout
            .unwrap_or(DEFAULT_STOP_STAGE_TIMEOUT)
    }

    fn get_stage(&self) -> u8 {
        self.cfg.app_cfg.stop_stages.unwrap_or(DEFAULT_STOP_STAGES)
    }

    fn check_timeout_for_stop(&self) -> FwResult<()> {
        let stop_timeout = self.get_stop_timeout();
        let stages = self.get_stage() as u32;
        let stage_timeout = self.get_stage_timeout();

        let total_stage_duration = stage_timeout.checked_mul(stages).ok_or_else(|| {
            FwError::ConfigError(
                "timeout configuration",
                "stop_stages or stage_stop_timeout is too large".to_string(),
            )
        })?;

        if stop_timeout <= total_stage_duration {
            let err_msg = format!(
                "invalid shutdown config: total stop_timeout ({:?}) must be greater than \
                all stages combined ({:?} * {} = {:?})",
                stop_timeout, stage_timeout, stages, total_stage_duration
            );

            tracing::error!("{}", err_msg);
            return Err(FwError::ConfigError("timeout configuration", err_msg));
        }

        Ok(())
    }

    pub fn create_root_span(self: Arc<Self>) -> tracing::Span {
        tracing::info_span!(
            "trace_meta",
            app_name = %self.get_app_name(),
            profile = %self.get_profile(),
            mip = %self.get_mip())
    }

    pub async fn new_run_state(self: Arc<Self>) -> FwResult<RunState> {
        RunState::new(
            self.get_app_name(),
            self.get_profile(),
            self.get_mip(),
            self.get_http_port(),
            self.get_rpc_port(),
            self.cfg.clone(),
            self.cancel_token.clone(),
        )
        .await
    }

    fn reselect_port(port_from_cfg: u16, port_name: &str) -> u16 {
        if let Ok(port_str) = env::var(port_name) {
            if let Ok(port) = port_str.parse::<u16>() {
                return port;
            }
        }

        port_from_cfg
    }

    fn reselect_http_port(port_from_cfg: u16) -> u16 {
        Self::reselect_port(port_from_cfg, "HTTP_PORT")
    }

    fn reselect_rpc_port(port_from_cfg: u16) -> u16 {
        Self::reselect_port(port_from_cfg, "RPC_PORT")
    }
}

impl Drop for App {
    fn drop(&mut self) {
        self.flush_log()
    }
}
