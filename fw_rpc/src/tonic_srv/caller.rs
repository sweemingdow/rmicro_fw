use crate::tonic_srv::tracer::RpcTraceUnit;
use fw_base::web_ctx_from_scope;
use fw_error::AppError;
use std::time::Duration;
use tonic;
use tonic::metadata::{Ascii, MetadataValue};
use tonic::{Code, Status};

pub struct RpcCaller;

impl RpcCaller {
    const DEFAULT_TIMEOUT: Duration = Duration::from_secs(1);
}

impl RpcCaller {
    // 强制超时
    pub async fn call_with_timeout<F, Fut, Req, Resp>(
        timeout: Duration,
        req: Req,
        run: F,
    ) -> Result<tonic::Response<Resp>, AppError>
    where
        F: FnOnce(tonic::Request<Req>) -> Fut,
        Fut: Future<Output = Result<tonic::Response<Resp>, tonic::Status>>,
    {
        let mut tonic_req = tonic::Request::new(req);
        tonic_req.set_timeout(timeout);

        Self::do_call(timeout, tonic_req, run).await
    }

    pub async fn call<F, Fut, Req, Resp>(
        req: Req,
        run: F,
    ) -> Result<tonic::Response<Resp>, AppError>
    where
        F: FnOnce(tonic::Request<Req>) -> Fut,
        Fut: Future<Output = Result<tonic::Response<Resp>, Status>>,
    {
        Self::call_with_timeout(Self::DEFAULT_TIMEOUT, req, run).await
    }

    pub async fn call_with_trace<F, Fut, Req, Resp>(
        action: &str,
        timeout: Option<Duration>,
        trace_unit: Option<RpcTraceUnit<'_>>,
        req: Req,
        run: F,
    ) -> Result<tonic::Response<Resp>, AppError>
    where
        F: FnOnce(tonic::Request<Req>) -> Fut,
        Fut: Future<Output = Result<tonic::Response<Resp>, tonic::Status>>,
    {
        let timeout = timeout.unwrap_or(Self::DEFAULT_TIMEOUT);
        let mut tonic_req = tonic::Request::new(req);
        tonic_req.set_timeout(timeout);

        let mm = tonic_req.metadata_mut();
        mm.insert("x-action", Self::wrap_val(action)?);
        match trace_unit {
            None => {
                let ctx = web_ctx_from_scope()?;
                mm.insert("x-req-id", Self::wrap_val(ctx.req_id())?);
                mm.insert("x-uid", Self::wrap_val(ctx.__no_matter_uid())?);
            }
            Some(unit) => {
                mm.insert("x-req-id", Self::wrap_val(unit.x_req_id())?);
                mm.insert("x-uid", Self::wrap_val(unit.x_uid().unwrap_or(""))?);
            }
        }

        Self::do_call(timeout, tonic_req, run).await
    }

    pub async fn call_trace_default<F, Fut, Req, Resp>(
        action: &str,
        req: Req,
        run: F,
    ) -> Result<tonic::Response<Resp>, AppError>
    where
        F: FnOnce(tonic::Request<Req>) -> Fut,
        Fut: Future<Output = Result<tonic::Response<Resp>, tonic::Status>>,
    {
        Self::call_with_trace(action, None, None, req, run).await
    }

    pub async fn call_trace_with_timeout<F, Fut, Req, Resp>(
        action: &str,
        timeout: Duration,
        req: Req,
        run: F,
    ) -> Result<tonic::Response<Resp>, AppError>
    where
        F: FnOnce(tonic::Request<Req>) -> Fut,
        Fut: Future<Output = Result<tonic::Response<Resp>, tonic::Status>>,
    {
        Self::call_with_trace(action, Some(timeout), None, req, run).await
    }

    fn wrap_val(val: &str) -> Result<MetadataValue<Ascii>, AppError> {
        MetadataValue::try_from(val).map_err(|e| AppError::RpcCallError(e.to_string()))
    }
}

impl RpcCaller {
    #[inline]
    async fn do_call<F, Fut, Req, Resp>(
        timeout: Duration,
        req: tonic::Request<Req>,
        run: F,
    ) -> Result<tonic::Response<Resp>, AppError>
    where
        F: FnOnce(tonic::Request<Req>) -> Fut,
        Fut: Future<Output = Result<tonic::Response<Resp>, tonic::Status>>,
    {
        match run(req).await {
            Ok(result) => Ok(result),
            Err(status) => {
                if Self::is_caller_timeout_error(&status) {
                    return Err(AppError::TimeoutError(
                        "rpc call",
                        format!("after {:?} ago", timeout),
                    ));
                }

                Err(AppError::RpcCallError(format!(
                    "rpc call err, status={:?}",
                    status
                )))
            }
        }
    }

    fn is_caller_timeout_error(status: &Status) -> bool {
        status.code() == Code::Cancelled && status.message() == "Timeout expired"
    }
}
