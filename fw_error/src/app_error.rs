use crate::FwError;
use thiserror;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("{0}")]
    ApiError(String),

    #[error("sql db error, {0}")]
    SqlDbError(String),

    #[error("rpc call error, {0}")]
    RpcCallError(String),

    #[error("unauthorized error")]
    UnauthorizedError(),

    #[error("inner error, {0}")]
    InnerError(String),

    #[error("parse error for {0}, {1}")]
    ParseError(&'static str, String),

    #[error("forbidden error, {0}")]
    ForbiddenError(String),

    #[error("reject error, {0}")]
    RejectError(String),

    #[error("timeout error for {0}, {1}")]
    TimeoutError(&'static str, String),

    #[error("internal fw error, {0}")]
    InternalFwError(#[from] FwError),

    #[error("internal error, {0}")]
    InternalError(#[from] anyhow::Error),

    #[error("unknow error, {0}")]
    UnknownError(String),
}

impl AppError {
    pub fn final_display(&self) -> (&'static str, String) {
        match self {
            Self::ApiError(msg) => ("0", msg.clone()),

            Self::SqlDbError(_) => ("900", "Inner Server Error".to_string()),
            Self::RpcCallError(_) => ("901", "Reject Error".to_string()),
            Self::UnauthorizedError() => ("990", "Unauthorized Error".to_string()),
            Self::InnerError(_) => ("992", "Inner Error".to_string()),
            Self::ParseError(_, _) => ("993", "Parse Error".to_string()),
            Self::ForbiddenError(_) => ("994", "Forbidden Error".to_string()),
            Self::RejectError(_) => ("995", "Reject Error".to_string()),
            Self::TimeoutError(_, _) => ("996", "Server Busy".to_string()),
            Self::InternalFwError(_) => ("997", "Internal System Error".to_string()),
            Self::InternalError(_) => ("998", "Internal Error".to_string()),
            Self::UnknownError(_) => ("999", "Unknown Error".to_string()),
        }
    }

    pub fn err_depth(&self) -> i16 {
        match self {
            Self::ApiError(_) => 1,
            _ => -1,
        }
    }
}
