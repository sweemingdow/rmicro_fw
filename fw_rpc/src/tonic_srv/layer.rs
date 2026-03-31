/*use futures::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use tonic::Status;
use tonic::body::BoxBody;

#[derive(Clone)]
pub struct TimeoutGrpcInterceptor<S> {
    inner: S,
}

impl<S, ReqBody> tower::Service<http::Request<ReqBody>> for TimeoutGrpcInterceptor<S>
where
    S: tower::Service<http::Request<ReqBody>, Response = http::Response<BoxBody>>,
    S::Error: Into<tower::BoxError>, // 捕获 TimeoutLayer 抛出的 BoxError
{
    type Response = http::Response<BoxBody>;
    type Error = tonic::Status; // 统一转为 gRPC Status
    type Future = TimeoutGrpcFuture<S::Future>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner
            .poll_ready(cx)
            .map_err(|e| Status::internal(e.into().to_string()))
    }

    fn call(&mut self, req: http::Request<ReqBody>) -> Self::Future {
        tracing::error!("!!! INTERCEPTOR CALLED !!!");
        TimeoutGrpcFuture {
            inner: self.inner.call(req),
        }
    }
}

#[pin_project::pin_project]
pub struct TimeoutGrpcFuture<F> {
    #[pin]
    inner: F,
}

// layer.rs 中的 Future 实现
impl<F, E> Future for TimeoutGrpcFuture<F>
where
    F: Future<Output = Result<http::Response<BoxBody>, E>>,
    E: Into<tower::BoxError>,
{
    type Output = Result<http::Response<BoxBody>, tonic::Status>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        match this.inner.poll(cx) {
            Poll::Ready(Ok(resp)) => {
                // 确保返回的是干净的 BoxBody
                Poll::Ready(Ok(resp.map(BoxBody::new)))
            }
            Poll::Ready(Err(e)) => {
                let err: tower::BoxError = e.into();
                tracing::error!("!!! SUCCESS: INTERCEPTOR CAUGHT: {:?}", err);

                let status = if err.is::<tower::timeout::error::Elapsed>() {
                    Status::deadline_exceeded("callee service timeout")
                } else {
                    Status::internal(err.to_string())
                };
                // into_http() 已经返回了 Response<BoxBody>
                Poll::Ready(Ok(status.into_http()))
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

// 对应的 Layer
#[derive(Clone, Copy)] // Layer 通常不持有状态，直接派生 Clone 和 Copy
pub struct TimeoutGrpcLayer;
impl<S> tower::Layer<S> for TimeoutGrpcLayer {
    type Service = TimeoutGrpcInterceptor<S>;
    fn layer(&self, inner: S) -> Self::Service {
        TimeoutGrpcInterceptor { inner }
    }
}
*/