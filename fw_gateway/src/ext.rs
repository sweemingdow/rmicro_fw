pub mod config_ext;

use crate::PingoraResult;
use async_trait::async_trait;
use pingora_http::{RequestHeader, ResponseHeader};
use pingora_proxy::Session;
use prost::bytes::Bytes;
use std::time::Duration;

#[async_trait]
pub trait GatewayHookExt: Send + Sync {
    type CTX: Send + Sync + 'static;

    fn new_ctx(&self) -> Self::CTX;

    // 对应: request_filter
    // 请求过滤 / 鉴权 / 限流 / 短路返回
    async fn on_request(&self, session: &mut Session, ctx: &mut Self::CTX) -> PingoraResult<bool> {
        tracing::trace!("default on_request");
        Ok(false)
    }

    // 对应: upstream_request_filter
    // 修改发往后端的请求头
    async fn on_upstream_request(
        &self,
        _: &mut Session,
        _: &mut RequestHeader,
        _ctx: &mut Self::CTX,
    ) -> PingoraResult<()> {
        tracing::trace!("default on_upstream_request");
        Ok(())
    }

    // 对应: response_filter
    // 修改从后端返回的响应头
    async fn on_response(
        &self,
        _: &mut Session,
        _: &mut ResponseHeader,
        _ctx: &mut Self::CTX,
    ) -> PingoraResult<()> {
        tracing::trace!("default on_response");
        Ok(())
    }

    // 对应: response_body_filter
    // 逐块处理/修改响应体（可返回延迟）
    fn on_response_body(
        &self,
        _: &mut Session,
        _: &mut Option<Bytes>,
        _: bool,
        _ctx: &mut Self::CTX,
    ) -> PingoraResult<Option<Duration>> {
        tracing::trace!("default on_response_body");
        Ok(None)
    }

    // 对应: logging
    // 请求结束日志 & 清理（总是执行）
    async fn on_logging(
        &self,
        _: &mut Session,
        _: Option<&pingora::Error>,
        _ctx: &mut Self::CTX,
    ) {
        tracing::trace!("default on_logging");
    }
}
