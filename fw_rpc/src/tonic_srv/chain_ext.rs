use crate::tonic_srv::server::TonicServer;
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

impl BootChainRpcExt for BootChain {
    fn add_rpc_server<F>(self, name: &str, rs: Arc<RunState>, init_services: F) -> Self
    where
        F: FnOnce(&mut Server) -> server::Router + Send + 'static,
    {
        let port = rs.cfg().app_cfg.rpc_port;
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
