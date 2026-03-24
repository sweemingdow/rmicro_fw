use tokio::signal;

#[derive(Debug)]
pub enum ExitSignal {
    CtrlC,
    Terminate,
}

pub async fn listen_exit_signal<F, Fut>(cb: F)
where
    F: FnOnce(ExitSignal) -> Fut,
    Fut: Future<Output = ()>,
{
    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    let sig = tokio::select! {
        _ = signal::ctrl_c() => {
            ExitSignal::CtrlC
        },
        _ = terminate => {
            ExitSignal::Terminate
        },
    };

    tracing::warn!(?sig, "received exit signal, executing callback...");

    cb(sig).await
}
