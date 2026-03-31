use http::{HeaderMap, HeaderValue, Request};
use std::time::Duration;
use tonic::transport::Server;
use tonic::transport::server::Router;
use tower::layer::util::{Identity, Stack};
use tower::timeout::TimeoutLayer;
use tower_http::classify::{GrpcErrorsAsFailures, SharedClassifier};
use tower_http::trace::DefaultOnEos; // 默认值
use tower_http::trace::DefaultOnFailure;
use tower_http::trace::DefaultOnRequest; // 默认值
use tower_http::trace::OnResponse;
use tower_http::trace::{DefaultOnBodyChunk, OnFailure}; // 默认值
use tower_http::trace::{MakeSpan, TraceLayer};
use tracing::Span;
// 默认值

// 重点：必须套上 SharedClassifier，这才是 new_for_grpc 的真实返回类型
// pub type FwTraceLayer = TraceLayer<SharedClassifier<GrpcErrorsAsFailures>, RpcMakeSpan>;
// // 这就是你想要给 v 定义的最终别名
// pub type FwTraceServer = Server<Stack<FwTraceLayer, Identity>>;
// pub type FwTraceRouter = Router<Stack<FwTraceLayer, Identity>>;

// 定义带 Trace 和 Timeout 的完整 Layer 栈

pub type FwTraceLayer = TraceLayer<
    SharedClassifier<GrpcErrorsAsFailures>,
    RpcMakeSpan,
    DefaultOnRequest,   // 第三个：OnRequest
    RpcOnResponse,      // 第四个：OnResponse (换成我们的结构体)
    DefaultOnBodyChunk, // 第五个
    DefaultOnEos,       // 第六个
    DefaultOnFailure,   // 第七个
>;

pub type FwTraceServer = Server<Stack<FwTraceLayer, Identity>>;
pub type FwTraceRouter = Router<Stack<FwTraceLayer, Identity>>;

pub type FwTraceTimeoutLayer = Stack<TimeoutLayer, FwTraceLayer>;

// 最终的 Server 和 Router 类型
// 不要单独给 Layer 定义别名了，直接定义 Server 的完整结构
// 这样能清晰看到嵌套关系：Timeout 在最外层，包裹着里面的 (Trace + Identity)

pub type FwTraceTimeoutServer = Server<Stack<TimeoutLayer, Stack<FwTraceLayer, Identity>>>;

pub type FwTraceTimeoutRouter = Router<Stack<TimeoutLayer, Stack<FwTraceLayer, Identity>>>;

// pub type FwTraceTimeoutServer =
//     Server<Stack<TimeoutGrpcLayer, Stack<Stack<FwTraceLayer, Identity>>>>;
// 
// pub type FwTraceTimeoutRouter =
//     Router<Stack<TimeoutGrpcLayer, Stack<TimeoutLayer, Stack<FwTraceLayer, Identity>>>>;

#[derive(Clone, Copy)]
pub struct RpcMakeSpan;

impl RpcMakeSpan {
    fn unwrap_val<'a>(hm: &'a HeaderMap<HeaderValue>, key: &str) -> &'a str {
        hm.get(key).and_then(|v| v.to_str().ok()).unwrap_or("")
    }
}

impl<B> MakeSpan<B> for RpcMakeSpan {
    fn make_span(&mut self, request: &Request<B>) -> Span {
        let hm = request.headers();
        let req_id = Self::unwrap_val(hm, "x-req-id");
        let action = Self::unwrap_val(hm, "x-action");
        let uid = Self::unwrap_val(hm, "x-uid");

        tracing::info_span!(
            "trace_meta",
            %req_id,
            %action,
            %uid,
            rpc_uri = %request.uri().path(),
        )
    }
}

#[derive(Clone, Copy)]
pub struct RpcOnResponse;

impl<B> OnResponse<B> for RpcOnResponse {
    fn on_response(self, response: &http::Response<B>, latency: Duration, _span: &Span) {
        tracing::debug!(
            status = %response.status().as_u16(),
            latency = ?latency,
            "rpc in callee completed"
        );
    }
}

// 1. 定义一个具名的 Failure Handler
#[derive(Clone, Copy)]
pub struct RpcOnFailure;

impl<T> OnFailure<T> for RpcOnFailure
where
    T: std::fmt::Display,
{
    fn on_failure(&mut self, failure: T, latency: std::time::Duration, _span: &tracing::Span) {
        let err_msg = failure.to_string();
        if err_msg.contains("timed out") {
            tracing::warn!(latency = ?latency, "Service timeout handled by interceptor");
        } else {
            tracing::error!(latency = ?latency, error = %err_msg, "RPC request failed");
        }
    }
}

// 自定义 OnFailure 实现

pub struct RpcTraceUnit<'a>(&'a str, Option<&'a str>);

impl<'a> RpcTraceUnit<'a> {
    pub fn with(req_uid: &'a str, uid: &'a str) -> Self {
        Self {
            0: req_uid,
            1: Some(uid),
        }
    }

    pub fn x_req_id(&self) -> &'a str {
        self.0
    }

    pub fn x_uid(&self) -> Option<&'a str> {
        self.1
    }
}
