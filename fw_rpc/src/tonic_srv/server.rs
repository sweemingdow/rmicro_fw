use crate::tonic_srv::tracer::{
    FwTraceRouter, FwTraceServer, FwTraceTimeoutRouter, FwTraceTimeoutServer, RpcMakeSpan,
    RpcOnResponse,
};
use fw_error::lib_error::FwError;
use fw_error::result::FwResult;
use tokio_util::sync;
use tonic::transport::Server;
use tonic::transport::server::Router;
use tower::timeout::TimeoutLayer;
use tower_http::trace::TraceLayer;

pub struct TonicServer {
    port: u16,
    cancel_token: sync::CancellationToken,
}

impl TonicServer {
    pub fn new(port: u16, cancel_token: sync::CancellationToken) -> Self {
        Self { port, cancel_token }
    }

    pub async fn run<F>(&self, init_services: F) -> FwResult<()>
    where
        F: FnOnce(&mut Server) -> Router,
    {
        let addr = format!("0.0.0.0:{}", self.port).parse().map_err(|_| {
            FwError::RunningError("topic server addr parse", format!("port={}", self.port))
        })?;

        let mut srv = Server::builder();

        let router = init_services(&mut srv);

        tracing::info!("tonic rpc server listening on {}", addr);

        let token = self.cancel_token.clone();

        router
            .serve_with_shutdown(addr, async move {
                token.cancelled().await;
                tracing::info!("tonic server received stop signal, start shutdown gracefully...");
            })
            .await
            .map_err(|e| FwError::RunningError("tonic server run", e.to_string()))?;

        tracing::info!("tonic server exited safely");

        Ok(())
    }

    pub async fn run_with_trace<F>(&self, init_services: F) -> FwResult<()>
    where
        F: FnOnce(&mut FwTraceServer) -> FwTraceRouter,
    {
        let addr = format!("0.0.0.0:{}", self.port).parse().map_err(|_| {
            FwError::RunningError("topic server addr parse", format!("port={}", self.port))
        })?;

        // 手动构造 Layer，指定使用 RpcMakeSpan
        let trace_layer = TraceLayer::new_for_grpc()
            .make_span_with(RpcMakeSpan)
            .on_response(RpcOnResponse);
        // .on_failure(RpcOnFailure);

        // 类型完美对齐 FwTraceServer
        let mut srv: FwTraceServer = Server::builder().layer(trace_layer);

        let router = init_services(&mut srv);

        tracing::info!("tonic rpc server with trace listening on {}", addr);

        router
            .serve_with_shutdown(addr, async move {
                self.cancel_token.cancelled().await;
            })
            .await
            .map_err(|e| FwError::RunningError("tonic server run with trace", e.to_string()))?;

        tracing::info!("tonic server with trace exited safely");

        Ok(())
    }

    pub async fn run_with_timeout<F>(
        &self,
        timeout: std::time::Duration,
        init_services: F,
    ) -> FwResult<()>
    where
        F: FnOnce(&mut FwTraceTimeoutServer) -> FwTraceTimeoutRouter,
    {
        let addr = format!("0.0.0.0:{}", self.port).parse().map_err(|_| {
            FwError::RunningError("topic server addr parse", format!("port={}", self.port))
        })?;

        let trace_layer = TraceLayer::new_for_grpc()
            .make_span_with(RpcMakeSpan)
            .on_response(RpcOnResponse);
        // .on_failure(RpcOnFailure);

        let timeout_layer = TimeoutLayer::new(timeout);

        let mut srv: FwTraceTimeoutServer =
            Server::builder().layer(trace_layer).layer(timeout_layer);

        let router = init_services(&mut srv);

        tracing::info!("tonic rpc server with timeout listening on {}", addr);

        router
            .serve_with_shutdown(addr, async move {
                self.cancel_token.cancelled().await;
            })
            .await
            .map_err(|e| FwError::RunningError("tonic server with timeout run", e.to_string()))?;

        tracing::info!("tonic server with timeout exited safely");

        Ok(())
    }
}
