use crate::config::router_config::TimeoutItem;
use crate::ext::GatewayHookExt;
use crate::proxy::HttpServerProxy;
use crate::{PingoraPeerResult, PingoraResult};
use arc_swap::ArcSwap;
use pingora_core::upstreams::peer::Peer;
use pingora_http::{RequestHeader, ResponseHeader};
use pingora_proxy::{ProxyHttp, Session};
use prost::bytes::Bytes;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

pub struct GatewayRouter {
    state: ArcSwap<RouterState>,

    // 抽取的segment深度, 从0开始. 如 extract_depth=2, 则 /gw/api/user_api/profile =>  /profile
    extract_depth: u8,
}

#[derive(Default, Clone)]
pub struct RouterState {
    // srv_name_to_proxy: DashMap<String, Arc<HttpServerProxy>>,
    routes: HashMap<String, Arc<HttpServerProxy>>,

    // TableItem只读
    timeouts: HashMap<String, Arc<TimeoutItem>>,
}

impl GatewayRouter {
    pub fn new(extract_depth: u8) -> Self {
        Self {
            state: ArcSwap::new(Arc::new(RouterState::default())),
            extract_depth,
        }
    }

    pub fn get_proxy(&self, srv_name: &str) -> Option<Arc<HttpServerProxy>> {
        self.state.load().routes.get(srv_name).cloned()
    }

    pub fn get_timeout_item(&self, srv_name: &str) -> Option<Arc<TimeoutItem>> {
        self.state.load().timeouts.get(srv_name).cloned()
    }

    pub fn add_proxies(&self, proxies: Vec<HttpServerProxy>) {
        if proxies.is_empty() {
            return;
        }

        let updates: Vec<(String, Arc<HttpServerProxy>)> = proxies
            .into_iter()
            .map(|p| (p.get_srv_name().to_string(), Arc::new(p)))
            .collect();

        self.state.rcu(|current| {
            let mut new_state = (**current).clone();
            for (name, proxy_arc) in &updates {
                tracing::info!("{} proxy added/updated via batch", name);
                new_state.routes.insert(name.clone(), proxy_arc.clone());
            }
            Arc::new(new_state)
        });
    }

    /// 批量移除代理
    pub fn remove_proxies(&self, srv_names: &[String]) {
        if srv_names.is_empty() {
            return;
        }

        self.state.rcu(|current| {
            // 检查是否有任何 key 存在，避免无效的克隆
            let any_exists = srv_names
                .iter()
                .any(|name| current.routes.contains_key(name));
            if !any_exists {
                return current.clone();
            }

            let mut new_state = current.as_ref().clone();
            for name in srv_names {
                if new_state.routes.remove(name).is_some() {
                    tracing::info!("{} proxy removed via batch", name);
                }
            }
            Arc::new(new_state)
        });
    }

    /// 保留单体函数作为简易接口，内部复用批量逻辑
    pub fn add_proxy(&self, proxy: HttpServerProxy) {
        self.add_proxies(vec![proxy]);
    }

    pub fn remove_proxy(&self, srv_name: &str) {
        self.remove_proxies(&[srv_name.to_string()]);
    }

    pub fn replace_all(
        &self,
        proxies: Vec<HttpServerProxy>,
        timeout_map: HashMap<String, TimeoutItem>,
    ) {
        let routes: HashMap<_, _> = proxies
            .into_iter()
            .map(|p| (p.get_srv_name().to_string(), Arc::new(p)))
            .collect();

        let timeouts: HashMap<_, _> = timeout_map
            .into_iter()
            .map(|(k, v)| (k, Arc::new(v)))
            .collect();

        let new_state = RouterState { routes, timeouts };
        let len = new_state.routes.len();

        self.state.store(Arc::new(new_state));
        tracing::info!("all routes and timeouts replaced, routes_len={len}");
    }
}

// 规避孤儿规则
pub struct GatewayRouterExt<E: GatewayHookExt> {
    router: Arc<GatewayRouter>,
    hook_ext: E,
}

#[async_trait::async_trait]
impl<E: GatewayHookExt> ProxyHttp for GatewayRouterExt<E> {
    type CTX = E::CTX;

    fn new_ctx(&self) -> Self::CTX {
        self.hook_ext.new_ctx()
    }

    async fn upstream_peer(
        &self,
        session: &mut Session,
        _ctx: &mut Self::CTX,
    ) -> PingoraPeerResult {
        let path = session.req_header().uri.path();
        let (Some(srv_name), prefix_len) = Self::extract_srv_info(path, self.router.extract_depth)
        else {
            return Err(pingora::Error::create(
                pingora::ErrorType::HTTPStatus(404),
                pingora::ErrorSource::Upstream,
                Some("Not Found".into()),
                None,
            ));
        };

        tracing::trace!(?path, ?srv_name, "upstream peer");

        if let Some(proxy) = self.router.get_proxy(srv_name) {
            let timeout_item = self.router.get_timeout_item(srv_name);

            Self::rewrite_path(session, prefix_len);

            return proxy.select_peer(timeout_item).await.inspect(|peer| {
                tracing::debug!(
                    "selected upstream peer, srv_name={}, upstream_addr={}",
                    proxy.get_srv_name(),
                    peer.address(),
                )
            });
        };

        Err(pingora::Error::create(
            pingora::ErrorType::HTTPStatus(502),
            pingora::ErrorSource::Upstream,
            Some("Bad Gateway".into()),
            None,
        ))
    }

    // 请求来到网关
    async fn request_filter(
        &self,
        session: &mut Session,
        ctx: &mut Self::CTX,
    ) -> PingoraResult<bool>
    where
        Self::CTX: Send + Sync,
    {
        self.hook_ext.on_request(session, ctx).await
    }

    // 发往upstream server前
    async fn upstream_request_filter(
        &self,
        session: &mut Session,
        upstream_request: &mut RequestHeader,
        ctx: &mut Self::CTX,
    ) -> PingoraResult<()>
    where
        Self::CTX: Send + Sync,
    {
        self.hook_ext
            .on_upstream_request(session, upstream_request, ctx)
            .await
    }

    // 收到后端响应头
    async fn response_filter(
        &self,
        session: &mut Session,
        upstream_response: &mut ResponseHeader,
        ctx: &mut Self::CTX,
    ) -> PingoraResult<()>
    where
        Self::CTX: Send + Sync,
    {
        self.hook_ext
            .on_response(session, upstream_response, ctx)
            .await
    }

    // 可以修改响应体
    fn response_body_filter(
        &self,
        session: &mut Session,
        body: &mut Option<Bytes>,
        end_of_stream: bool,
        ctx: &mut Self::CTX,
    ) -> PingoraResult<Option<Duration>>
    where
        Self::CTX: Send + Sync,
    {
        self.hook_ext
            .on_response_body(session, body, end_of_stream, ctx)
    }

    // 请求结束
    async fn logging(&self, session: &mut Session, e: Option<&pingora::Error>, ctx: &mut Self::CTX)
    where
        Self::CTX: Send + Sync,
    {
        self.hook_ext.on_logging(session, e, ctx).await;
    }
}

impl<E: GatewayHookExt> GatewayRouterExt<E> {
    pub fn new(router: Arc<GatewayRouter>, hook_ext: E) -> Self {
        Self { router, hook_ext }
    }

    pub fn get_router(&self) -> Arc<GatewayRouter> {
        self.router.clone()
    }

    // 不分配堆String, 返回字符切片及srv_name的索引
    fn extract_srv_info(path: &str, depth: u8) -> (Option<&str>, usize) {
        let segments = path.split_terminator('/');
        let mut current_pos = 0;

        for (i, seg) in segments.enumerate() {
            if i == 0 && seg.is_empty() {
                // 处理开头的 /
                current_pos += 1;
                continue;
            }

            if (i - 1) as u8 == depth {
                // 匹配到对应深度的 segment
                let start = current_pos;
                let end = start + seg.len();
                return (Some(&path[start..end]), end);
            }

            current_pos += seg.len() + 1; // +1 是为了跳过 /
        }

        (None, 0)
    }

    // 极致优化的路径重写：在无 Query 场景下实现零 String 分配
    fn rewrite_path(session: &mut Session, prefix_len: usize) {
        let header = session.req_header_mut();
        let uri = &header.uri;

        // 1. 获取原始的 PathAndQuery
        if let Some(pq) = uri.path_and_query() {
            let old_path = pq.path();

            // 2. 边界检查：如果路径长度不足以裁剪，说明配置或逻辑有误
            if old_path.len() < prefix_len {
                return;
            }

            // 3. 裁剪前缀，获取新的路径切片
            let new_path_slice = &old_path[prefix_len..];
            let final_path = if new_path_slice.is_empty() {
                "/"
            } else {
                new_path_slice
            };

            // 4. 核心优化逻辑：分支处理 Query
            let new_pq = match pq.query() {
                // 情况 A: 有 Query 参数，无法避免拼接，必须分配 String
                Some(q) => {
                    let mut new_pq_raw = String::with_capacity(final_path.len() + q.len() + 1);
                    new_pq_raw.push_str(final_path);
                    new_pq_raw.push('?');
                    new_pq_raw.push_str(q);

                    // 从 String 转换，消耗掉这个 String
                    http::uri::PathAndQuery::try_from(new_pq_raw).ok()
                }
                // 情况 B: 无 Query 参数，直接从切片构造，避免 String 分配
                None => {
                    // TryFrom<&str> 对于 PathAndQuery 是高度优化的
                    http::uri::PathAndQuery::try_from(final_path).ok()
                }
            };

            // 5. 更新 URI
            if let Some(new_pq) = new_pq {
                let mut parts = uri.clone().into_parts();
                parts.path_and_query = Some(new_pq);

                if let Ok(new_uri) = http::Uri::from_parts(parts) {
                    header.set_uri(new_uri);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ext::GatewayHookExt; // 假设你的 trait 在这里

    // 定义一个最小的 Dummy 类型用于测试（满足 bound）
    struct DummyHook;

    #[async_trait::async_trait]
    impl GatewayHookExt for DummyHook {
        type CTX = ();

        fn new_ctx(&self) -> Self::CTX {
            ()
        }

        // 其他方法用默认实现即可（如果你的 trait 有默认实现）
    }

    #[test]
    fn test_extract_srv_name() {
        let url = "/test_api/test";
        let (res, idx) = GatewayRouterExt::<DummyHook>::extract_srv_info(url, 0); // 或用任意类型
        println!("res={res:?}, idx={idx}");

        let url = "/fsx/endpoints/test_api/test";
        let (res, idx) = GatewayRouterExt::<DummyHook>::extract_srv_info(url, 2);
        println!("res={res:?}, idx={idx}");
    }
}
