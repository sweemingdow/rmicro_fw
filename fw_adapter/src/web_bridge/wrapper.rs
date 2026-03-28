use crate::web_bridge::RespResult;
use anyhow::anyhow;
use axum::response::{IntoResponse, Response};
use fw_error::app_error::AppError;
use fw_error::recorder::ErrLogRecorder;

pub struct AnyErrorWrapper(pub anyhow::Error);

impl AnyErrorWrapper {
    pub fn from_app_err(ae: AppError) -> Self {
        Self(anyhow!(ae))
    }
}

impl<E> From<E> for AnyErrorWrapper
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self(err.into())
    }
}

pub type WebResult<T> = Result<T, AnyErrorWrapper>;

impl IntoResponse for AnyErrorWrapper {
    fn into_response(self) -> Response {
        let err = self.0;

        if let Some(ae) = err.downcast_ref::<AppError>() {
            let (code, msg) = ae.final_display();

            err.log_record(ae.err_depth() as isize);

            return RespResult::<()>::code_msg_err(code, msg).into_response();
        }

        err.log_record(-1);

        RespResult::<()>::code_msg_err("999", "Internal Error".to_string()).into_response()
    }
}
