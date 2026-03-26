use axum::{
    Json,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use std::time::{SystemTime, UNIX_EPOCH};

const OK: &'static str = "1";
pub const GEN_ERR: &'static str = "0";

#[derive(Debug, Serialize)]
pub struct RespResult<T> {
    code: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    sub_code: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    msg: Option<String>,

    ts: u64,

    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<T>,
}

impl<T: Serialize> IntoResponse for RespResult<T> {
    fn into_response(self) -> Response {
        Json(self).into_response()
    }
}

impl<T> RespResult<T> {
    pub fn all<C, S, M>(code: C, sub_code: S, msg: M, data: T) -> Self
    where
        C: Into<String>,
        S: Into<String>,
        M: Into<String>,
    {
        Self {
            code: code.into(),
            sub_code: Some(sub_code.into()),
            msg: Some(msg.into()),
            ts: Self::curr_ts(),
            data: Some(data),
        }
    }

    pub fn just_ok() -> Self {
        Self {
            code: OK.to_owned(),
            sub_code: None,
            msg: None,
            ts: Self::curr_ts(),
            data: None,
        }
    }

    pub fn ok(data: T) -> Self {
        Self {
            code: OK.to_owned(),
            sub_code: None,
            msg: None,
            ts: Self::curr_ts(),
            data: Some(data),
        }
    }

    pub fn msg_err<M: Into<String>>(msg: M) -> Self {
        Self {
            code: GEN_ERR.to_owned(),
            sub_code: None,
            msg: Some(msg.into()),
            ts: Self::curr_ts(),
            data: None,
        }
    }

    pub fn code_sub_err<C, S>(code: C, sub_code: S, data: T) -> Self
    where
        C: Into<String>,
        S: Into<String>,
    {
        Self {
            code: code.into(),
            sub_code: Some(sub_code.into()),
            msg: None,
            ts: Self::curr_ts(),
            data: Some(data),
        }
    }

    pub fn code_msg_err<C: Into<String>, M: Into<String>>(code: C, msg: M) -> Self {
        Self {
            code: code.into(),
            sub_code: None,
            msg: Some(msg.into()),
            ts: Self::curr_ts(),
            data: None,
        }
    }

    pub fn code_err<C: Into<String>>(code: C) -> Self {
        Self {
            code: code.into(),
            sub_code: None,
            msg: None,
            ts: Self::curr_ts(),
            data: None,
        }
    }

    pub fn sub_err<S: Into<String>>(sub_code: S) -> Self {
        Self {
            code: GEN_ERR.to_owned(),
            sub_code: Some(sub_code.into()),
            msg: None,
            ts: Self::curr_ts(),
            data: None,
        }
    }

    fn curr_ts() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64
    }
}
