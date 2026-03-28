use anyhow::anyhow;
use fw_base::from_scope;
use fw_error::AnyResult;
use fw_error::AppError;
use std::time;
use std::time::Duration;
use tonic;
use tonic::metadata::MetadataValue;

pub struct RpcCaller;

impl RpcCaller {
    const DEFAULT_TIMEOUT: Duration = Duration::from_secs(1);
}

impl RpcCaller {
    // 强制超时
    pub async fn call_with_timeout<F, Fut, Req, Resp>(
        timeout: Option<Duration>,
        req: Req,
        run: F,
    ) -> Result<tonic::Response<Resp>, AppError>
    where
        F: FnOnce(tonic::Request<Req>) -> Fut,
        Fut: Future<Output = Result<tonic::Response<Resp>, tonic::Status>>,
    {
        let timeout = timeout.unwrap_or(Self::DEFAULT_TIMEOUT);
        let tonic_req = tonic::Request::new(req);

        match tokio::time::timeout(timeout, run(tonic_req)).await {
            Ok(result) => match result {
                Ok(resp) => Ok(resp),
                Err(status) => Err(AppError::RpcCallError(status.message().to_string())),
            },
            Err(_) => Err(AppError::TimeoutError(
                "rpc call",
                format!("after {:?} ago", timeout),
            )),
        }
    }

    pub async fn call_default_timeout<F, Fut, Req, Resp>(
        req: Req,
        run: F,
    ) -> Result<tonic::Response<Resp>, AppError>
    where
        F: FnOnce(tonic::Request<Req>) -> Fut,
        Fut: Future<Output = Result<tonic::Response<Resp>, tonic::Status>>,
    {
        Self::call_with_timeout(None, req, run).await
    }

    pub async fn call_with_trace<F, Fut, Req, Resp>(
        action: &str,
        req: Req,
        run: F,
    ) -> Result<tonic::Response<Resp>, AppError>
    where
        F: FnOnce(tonic::Request<Req>) -> Fut,
        Fut: Future<Output = Result<tonic::Response<Resp>, tonic::Status>>,
    {
        let mut tonic_req = tonic::Request::new(req);
        let mm = tonic_req.metadata_mut();

        let ctx = from_scope()?;

        let req_id_val = MetadataValue::try_from(ctx.req_id())
            .map_err(|e| AppError::RpcCallError(e.to_string()))?;
        let action_val =
            MetadataValue::try_from(action).map_err(|e| AppError::RpcCallError(e.to_string()))?;

        let uid = ctx.__no_matter_uid();
        if uid != "" {
            let uid_val =
                MetadataValue::try_from(uid).map_err(|e| AppError::RpcCallError(e.to_string()))?;
            mm.insert("x-uid", uid_val);
        }
        mm.insert("x-req-id", req_id_val);
        mm.insert("x-action", action_val);

        match tokio::time::timeout(Self::DEFAULT_TIMEOUT, run(tonic_req)).await {
            Ok(result) => match result {
                Ok(resp) => Ok(resp),
                Err(status) => Err(AppError::RpcCallError(status.message().to_string())),
            },
            Err(_) => Err(AppError::TimeoutError(
                "rpc call",
                format!("after {:?} ago", Self::DEFAULT_TIMEOUT),
            )),
        }
    }
}
