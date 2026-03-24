use thiserror;
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("")]
    RpcCallError(),
}
