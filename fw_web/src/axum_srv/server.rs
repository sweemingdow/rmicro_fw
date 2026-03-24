use fw_error::lib_error::FwError;
use fw_error::result::FwResult;
use tokio_util::sync;

pub struct AxumServer {
    port: u16,
    cancel_token: sync::CancellationToken,
}


impl AxumServer {
    pub fn new(port: u16, cancel_token: sync::CancellationToken) -> Self {
        Self { port, cancel_token }
    }

    pub async fn run<F>(&self, init_router: F) -> FwResult<()>
    where
        F: FnOnce(axum::Router) -> axum::Router,
    {
        let mut app = axum::Router::new();
        app = init_router(app);

        app = app.fallback(handler_404);

        let addr = format!("0.0.0.0:{}", self.port);
        let listener = tokio::net::TcpListener::bind(&addr)
            .await
            .map_err(|e| FwError::RunningError("axum server addr bind", e.to_string()))?;

        tracing::info!("axum server listening on {}", addr);

        let token = self.cancel_token.clone();
        axum::serve(listener, app)
            .with_graceful_shutdown(async move {
                token.cancelled().await;
                tracing::warn!("axum server received stop signal, start shutdown gracefully...");
            })
            .await
            .map_err(|e| FwError::RunningError("axum server run", e.to_string()))?;

        tracing::info!("axum server exited safely");

        Ok(())
    }
}

async fn handler_404(uri: axum::http::Uri) -> axum::http::StatusCode {
    tracing::warn!(%uri, "no resource found");

    axum::http::StatusCode::NOT_FOUND
}
