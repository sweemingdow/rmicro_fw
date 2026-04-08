use crate::tonic_srv::server::TonicServer;
use crate::tonic_srv::tracer::{
    FwTraceRouter, FwTraceServer, FwTraceTimeoutRouter, FwTraceTimeoutServer,
};
use fw_boot::BootChain;
use fw_boot::state::RunState;
use std::sync::Arc;
use tonic::transport::{Server, server};
use tracing::Instrument;

pub trait BootChainRpcExt {
    fn add_rpc_server<F>(self, name: &str, rs: Arc<RunState>, init_services: F) -> Self
    where
        F: FnOnce(&mut Server) -> server::Router + Send + 'static;
}

pub trait BootChainRpcTraceExt {
    fn add_rpc_server_with_trace<F>(self, name: &str, rs: Arc<RunState>, init_services: F) -> Self
    where
        F: FnOnce(&mut FwTraceServer) -> FwTraceRouter + Send + 'static;
}

impl BootChainRpcExt for BootChain {
    fn add_rpc_server<F>(self, name: &str, rs: Arc<RunState>, init_services: F) -> Self
    where
        F: FnOnce(&mut Server) -> server::Router + Send + 'static,
    {
        let port = rs.rpc_port();
        let span = tracing::Span::current();

        self.add_backend(name, move |token| {
            async move {
                let rpc_srv = TonicServer::new(port, token);

                rpc_srv.run(init_services).await
            }
            .instrument(span)
        })
    }
}

impl BootChainRpcTraceExt for BootChain {
    fn add_rpc_server_with_trace<F>(self, name: &str, rs: Arc<RunState>, init_services: F) -> Self
    where
        F: FnOnce(&mut FwTraceServer) -> FwTraceRouter + Send + 'static,
    {
        let port = rs.rpc_port();
        let span = tracing::Span::current();

        self.add_backend(name, move |token| {
            async move {
                let rpc_srv = TonicServer::new(port, token);

                rpc_srv.run_with_trace(init_services).await
            }
            .instrument(span)
        })
    }
}

pub trait BootChainRpcTimeoutExt {
    fn add_rpc_server_with_global_timeout<F>(
        self,
        name: &str,
        rs: Arc<RunState>,
        timeout: std::time::Duration,
        init_services: F,
    ) -> Self
    where
        F: FnOnce(&mut FwTraceTimeoutServer) -> FwTraceTimeoutRouter + Send + 'static;
}

impl BootChainRpcTimeoutExt for BootChain {
    fn add_rpc_server_with_global_timeout<F>(
        self,
        name: &str,
        rs: Arc<RunState>,
        timeout: std::time::Duration,
        init_services: F,
    ) -> Self
    where
        F: FnOnce(&mut FwTraceTimeoutServer) -> FwTraceTimeoutRouter + Send + 'static,
    {
        let port = rs.rpc_port();
        let span = tracing::Span::current();

        self.add_backend(name, move |token| {
            async move {
                let rpc_srv = TonicServer::new(port, token);
                rpc_srv.run_with_timeout(timeout, init_services).await
            }
            .instrument(span)
        })
    }
}
