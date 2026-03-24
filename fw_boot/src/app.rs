use crate::config::Config;
use crate::graceful;
use crate::state::RunState;
use fw_base::my_utils;
use fw_error::lib_error::FwError;
use fw_error::result::FwResult;
use fw_log::my_log;
use fw_regdis::nacos::client::{NacosCliOptions, NacosClient};
use fw_regdis::nacos::configuration::NacosConfiguration;
use fw_regdis::nacos::discovery::NacosDiscovery;
use fw_regdis::nacos::proxy::NacosProxy;
use fw_regdis::nacos::registry::{
    DeregisterOptions, NacosRegister, NacosRegisterImpl, RegisterOptions,
};
use fw_rpc::tonic_srv::chan_factory::RpcChanFactory;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time;
use tokio_util::sync;
use tracing::Instrument;
use tracing_appender::non_blocking;

const DEFAULT_STOP_TIMEOUT: time::Duration = time::Duration::from_secs(30);
const DEFAULT_CLEAN_TIMEOUT: time::Duration = time::Duration::from_secs(10);
const DEFAULT_STOP_STAGES: time::Duration = time::Duration::from_secs(2);
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
        let http_port = cfg.app_cfg.http_port;
        let rpc_port = cfg.app_cfg.rpc_port;

        let log_guard = my_log::init_logger(my_log::LogOptions {
            max_log_files: cfg.log_cfg.max_log_files,
            log_dir: cfg.log_cfg.log_dir.to_string(),
            app_name: app_name.clone(),
            port: http_port,
        });

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
    pub async fn run<F1, Fut1, F2, Fut2>(
        self: Arc<Self>,
        run_action: F1,
        clean_action: F2,
    ) -> FwResult<()>
    where
        F1: FnOnce(Arc<RunState>) -> Fut1,
        F2: FnOnce(Arc<RunState>) -> Fut2,
        Fut1: Future<Output = ()>,
        Fut2: Future<Output = ()>,
    {
        let app_name = self.get_app_name();

        // 创建root span
        let root_span = tracing::info_span!(
            "app_meta",
            %app_name,
            profile = %self.get_profile(),
            mip = %self.get_mip()
        );

        let self_clone = self.clone();
        let stop_timeout = self.get_stop_timeout();
        let clean_timeout = self.get_clean_timeout();
        let runner = async move {
            // 初始化RunState
            let rs = Arc::new(
                RunState::new(self_clone.cfg.clone(), self_clone.cancel_token.clone()).await?,
            );

            let rs_clone = rs.clone();
            let self_in_lis = self_clone.clone();
            tokio::spawn(async move {
                // 监听并退出
                self_in_lis.clone().listen_then_exit(rs_clone).await;
            });

            let self_in_stop = self_clone.clone();
            tokio::select! {
                // 业务自身响应cancellation token或自身退出
                _ = run_action(rs.clone()) => {
                    tracing::info!("run_action shutdown completed gracefully");
                }

                // 响应了中断signal后, shutdown超时了
                _ = async {
                    // 先等待信号被触发（不管是 Ctrl+C 还是其他地方调了 cancel）
                    self_in_stop.cancel_token.cancelled().await;

                    tracing::warn!("run_action shutdown now...");

                    tokio::time::sleep(stop_timeout).await;
                    // 走到这, 已经run_action shutdown已经超时了
                } => {
                    tracing::error!(
                        "run_action shutdown timeout after {:?}",
                        stop_timeout
                    );
                }
            }

            // 最后注册到nacos
            self_clone
                .clone()
                .register_to_nacos(rs.nacos_proxy())
                .await?;
            tracing::info!("register instance to nacos successfully");

            // 确保停机信号已经触发或业务自然的结束
            if !self_clone.cancel_token.is_cancelled() {
                self_clone.cancel_token.cancel();
            }

            // 自定义清理
            let clean_task = async {
                clean_action(rs).await;
            };

            if let Err(_) = tokio::time::timeout(clean_timeout, clean_task).await {
                tracing::error!("clean_action shutdown timeout after:{:?}", clean_timeout);
            }

            tracing::info!("application loop done, exit process...");

            Ok::<(), FwError>(())
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
    async fn register_to_nacos(&self, nacos_proxy: Arc<NacosProxy>) -> FwResult<()> {
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

        nacos_proxy.register(reg_ops).await
    }

    async fn deregister_from_nacos(&self, nacos_proxy: Arc<NacosProxy>) -> FwResult<()> {
        let deg_ops = DeregisterOptions {
            group_name: Some(self.cfg.nacos_center_cfg.registry.group_name.clone()),
            srv_name: self.app_name.clone(),
            addr: self.http_server_addr.clone(),
            cluster_name: None,
        };

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
}

impl Drop for App {
    fn drop(&mut self) {
        self.flush_log()
    }
}
