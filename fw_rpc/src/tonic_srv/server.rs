use fw_error::lib_error::FwError;
use fw_error::result::FwResult;
use tokio_util::sync;
use tonic::transport::{Server, server};

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
        F: FnOnce(&mut Server) -> server::Router,
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
}
