use async_trait::async_trait;
use fw_boot::App;
use fw_boot::state::RunState;
use fw_regdis::nacos::registry::DeregisterOptions;
use pingora_core::server::ShutdownWatch;
use pingora_core::services::background::BackgroundService;
use std::sync::Arc;

pub struct BackgroundRunner {
    rs: Arc<RunState>,
    app: Arc<App>,
}

impl BackgroundRunner {
    pub fn new(rs: Arc<RunState>, app: Arc<App>) -> Self {
        Self { rs, app }
    }
}

#[async_trait]
impl BackgroundService for BackgroundRunner {
    async fn start(&self, mut shutdown: ShutdownWatch) {
        loop {
            tokio::select! {
                _ = shutdown.changed() => {
                    tracing::warn!("BackgroundRunner received shutdown signal");

                    self.clean().await;
                    break;
                }
            }
        }
    }
}

impl BackgroundRunner {
    async fn clean(&self) {
        // 自身从nacos注销
        let _ = self.app.deregister_from_nacos(self.rs.nacos_proxy()).await;

        // 解除对配置监听
        self.rs
            .nacos_proxy()
            .get_nacos_configure()
            .unlisten_all()
            .await;

        // 解除对服务的监听
        self.rs
            .nacos_proxy()
            .get_nacos_discover()
            .unwatch_all()
            .await;
    }
}
