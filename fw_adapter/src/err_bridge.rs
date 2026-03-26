use crate::web_bridge::response;
use fw_error::FwError;
use thiserror;

pub type AnyResult<T> = anyhow::Result<T>;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("{0}")]
    ApiError(String),

    #[cfg(feature = "sqlx_bridge")]
    #[error("sql db error, {0}")]
    SqlDbError(#[from] sqlx::Error),

    #[error("internal fw error, {0}")]
    InternalFwError(#[from] FwError),

    #[error("internal error, {0}")]
    InternalError(#[from] anyhow::Error),
}

impl AppError {
    pub fn final_display(&self) -> (&'static str, String) {
        match self {
            Self::ApiError(msg) => (response::GEN_ERR, msg.clone()),

            #[cfg(feature = "sqlx_bridge")]
            Self::SqlDbError(_) => ("998", "Inner Server Error".to_string()),

            Self::InternalFwError(_) => ("997", "Internal System Error".to_string()),
            Self::InternalError(_) => ("998", "Internal Error".to_string()),
        }
    }

    pub fn err_depth(&self) -> i16 {
        match self {
            Self::ApiError(_) => 1,
            _ => -1,
        }
    }
}
