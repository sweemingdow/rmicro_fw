use std::time::{Duration, Instant};
use tokio::{task::JoinHandle, time};

use fw_error::{FwError, FwResult};
use tokio_util::sync;
use tracing::Instrument;

pub struct BootNode {
    name: String,
    handle: JoinHandle<()>,
    token: sync::CancellationToken,
}

impl BootNode {
    pub fn new(name: String, handle: JoinHandle<()>, token: sync::CancellationToken) -> Self {
        Self {
            name,
            handle,
            token,
        }
    }
}

/*
控制启动顺序和销毁顺序
前端节点并发销毁
后端节点在所有前端节点销毁后, 再进行销毁
*/
pub struct BootChain {
    // 取消总令牌(监听signal)
    parent_token: sync::CancellationToken,

    // 所有前端节点
    frontends: Vec<BootNode>,

    // 所有后端节点
    backends: Vec<BootNode>,
}

impl BootChain {
    pub fn new(parent_token: sync::CancellationToken) -> Self {
        Self {
            parent_token,
            frontends: Vec::new(),
            backends: Vec::new(),
        }
    }

    pub fn add_frontend<F, Fut>(mut self, name: &str, factory: F) -> Self
    where
        F: FnOnce(sync::CancellationToken) -> Fut + Send + 'static,
        Fut: Future<Output = FwResult<()>> + Send + 'static,
    {
        let token = sync::CancellationToken::new();
        let token_clone = token.clone();
        let parent_token = self.parent_token.clone();
        let node_name = name.to_string();

        let current_span = tracing::Span::current();

        let handle = tokio::spawn(
            async move {
                if let Err(e) = factory(token_clone).await {
                    tracing::error!(
                        "[BootChain Starter]: node=[{}] start failed, err={:?}",
                        node_name,
                        e
                    );
                    parent_token.cancel();
                }
            }
            .instrument(current_span),
        );

        self.frontends
            .push(BootNode::new(name.to_string(), handle, token));

        self
    }


    pub fn add_backend<F, Fut>(mut self, name: &str, factory: F) -> Self
    where
        F: FnOnce(sync::CancellationToken) -> Fut + Send + 'static,
        Fut: Future<Output = FwResult<()>> + Send + 'static,
    {
        let token = sync::CancellationToken::new();
        let token_clone = token.clone();
        let parent_token = self.parent_token.clone();
        let node_name = name.to_string();

        let current_span = tracing::Span::current();

        let handle = tokio::spawn(
            async move {
                // 如果后端服务（比如 RPC）启动报错
                if let Err(e) = factory(token_clone).await {
                    tracing::error!(
                        "[BootChain Starter]: backend node=[{}] start failed, err={:?}",
                        node_name,
                        e
                    );

                    parent_token.cancel();
                }
            }
            .instrument(current_span),
        );

        self.backends
            .push(BootNode::new(name.to_string(), handle, token));

        self
    }

    /// 执行链式停机
    /// stage_timeout: 每一个阶段（Frontend/Backend）允许的最长退出时间
    pub async fn run<F, Fut>(self, stage_timeout: Duration, ready_action: F) -> FwResult<()>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = FwResult<()>>,
    {
        ready_action().await?;

        // 1. 等待 App 框架的总控信号
        self.parent_token.cancelled().await;

        let exit_start = Instant::now();

        tracing::trace!(
            "[BootChain Destroy]: starting shutdown sequence, frontends={}, backends={}",
            self.frontends.len(),
            self.backends.len()
        );

        // 2. 第一阶段：关闭所有前台节点 (Axum, MQ...)
        tracing::trace!("[BootChain Destroy]: stopping all frontends now...");
        for node in &self.frontends {
            node.token.cancel();
        }

        let frontend_stop_task = async {
            for node in self.frontends {
                let _ = node.handle.await;
                tracing::debug!(
                    "[BootChain Destroy]: frontend node=[{}] exited safely",
                    node.name
                );
            }
        };

        if let Err(e) = time::timeout(stage_timeout, frontend_stop_task).await {
            tracing::error!(
                err_msg = %e,
                "[BootChain Destroy]: frontends shutdown timeout after {:?}, Forcing next stage...",
                stage_timeout
            );
        }

        // 3. 第二阶段：关闭所有后台节点 (Tonic RPC...)
        tracing::trace!("[BootChain Destroy]: stopping all backends now...");
        for node in &self.backends {
            node.token.cancel();
        }

        let backend_stop_task = async {
            for node in self.backends {
                let _ = node.handle.await;
                tracing::debug!(
                    "[BootChain Destroy]: backend node=[{}] exited safely",
                    node.name
                );
            }
        };

        if let Err(e) = time::timeout(stage_timeout, backend_stop_task).await {
            tracing::error!(
                err_msg = %e,
                "[BootChain Destroy]: backends shutdown timeout after {:?}, Forcing next stage...",
                stage_timeout
            );
        }

        tracing::info!(
            "[BootChain Destroy]: all nodes exit safely, took={:?}",
            exit_start.elapsed()
        );

        Ok(())
    }
}
