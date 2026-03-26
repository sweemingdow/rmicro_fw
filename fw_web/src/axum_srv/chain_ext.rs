// fw_web/src/lib.rs
use crate::axum_srv::server::AxumServer;
use axum::http::Request;
use fw_boot::BootChain;
use fw_boot::state::RunState;
use fw_error::result::FwResult;
use std::sync::Arc;
use std::time;
use tower_http::trace::{DefaultOnResponse, TraceLayer};
use tracing::{Instrument, Level};

pub trait BootChainWebExt {
    fn add_web_server<F, Fut>(self, name: &str, rs: Arc<RunState>, init_router: F) -> Self
    where
        F: FnOnce(axum::Router) -> Fut + Send + 'static,
        Fut: Future<Output = axum::Router> + Send + 'static;
}

impl BootChainWebExt for BootChain {
    fn add_web_server<F, Fut>(self, name: &str, rs: Arc<RunState>, init_router: F) -> Self
    where
        F: FnOnce(axum::Router) -> Fut + Send + 'static,
        Fut: Future<Output = axum::Router> + Send + 'static,
    {
        let port = rs.cfg().app_cfg.http_port;

        let app_name = rs.app_name().to_string();
        let profile = rs.profile().to_string();
        let mip = rs.mip().to_string();

        self.add_frontend(name, move |token| {
            async move {
                let router = init_router(axum::Router::new()).await;
                let middleware = TraceLayer::new_for_http()
                    .make_span_with(move |request: &Request<_>| {
                        // 给每一个请求携带span
                        tracing::info_span!(
                            "app_meta",
                            app_name = %app_name,
                            profile = %profile,
                            mip = %mip,
                            method = %request.method(),
                            uri = %request.uri().path(),
                        )
                    })
                    .on_response(
                        // status = 200时 输出
                        |response: &axum::http::Response<_>,
                         latency: time::Duration,
                         _: &tracing::Span| {
                            tracing::debug!(
                                status = %response.status().as_u16(),
                                latency = ?latency,
                                "request completed"
                            )
                        },
                    );

                // 挂载到业务路由上
                let router = router.layer(middleware);
                AxumServer::new(port, token).run(|_| router).await
            }
        })
    }
}
