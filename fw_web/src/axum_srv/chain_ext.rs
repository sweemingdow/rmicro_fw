use crate::axum_srv::middleware::auth_layer;
use crate::axum_srv::server::AxumServer;
use axum::http::Request;
use axum::middleware;
use fw_boot::BootChain;
use fw_boot::state::RunState;
use std::sync::Arc;
use std::time;
use tower_http::trace::TraceLayer;
use tracing::field;
use fw_base::web_ctx_from_scope;

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

                let trace_layer = TraceLayer::new_for_http()
                    .make_span_with(move |request: &Request<_>| {
                        let ctx_opt = web_ctx_from_scope().ok();
                        let req_id = ctx_opt.as_ref().map(|c| c.req_id()).unwrap_or("");
                        let uid = ctx_opt
                            .as_ref()
                            .and_then(|c| c.uid_with_check().ok())
                            .unwrap_or("");

                        /*let (req_id, uid) = request
                        .extensions()
                        .get::<WebContext>()
                        .map(|ctx| (ctx.req_id(), ctx.uid_with_check().unwrap_or("")))
                        .unwrap_or_else(|| ("", ""));*/

                        // 给每一个请求携带span
                        tracing::info_span!(
                            "trace_meta",
                            %req_id,
                            %uid,
                            app_name = %app_name,
                            profile = %profile,
                            mip = %mip,
                            action = field::Empty, // 占位符, 稍后填充该值
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

                let router = router
                    .layer(trace_layer) // 第二层
                    .layer(middleware::from_fn(auth_layer)); // 第一层

                AxumServer::new(port, token).run(|_| router).await
            }
        })
    }
}
