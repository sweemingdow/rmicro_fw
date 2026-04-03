新请求到达 (客户端发来请求头)
↓
early_request_filter()          ← 最早阶段（极早期过滤）
↓
new_ctx()                       ← 为本次请求创建上下文（同步）
↓
request_filter()                ← 请求过滤 / 鉴权 / 限流 / 短路返回
↓
upstream_peer()                 ← 必须实现：选择后端 Peer（关键路由点）
↓
upstream_request_filter()       ← 修改发往后端的请求头
↓
(连接上游 → 发送请求 → 接收响应头)
↓
response_filter()               ← 修改从后端返回的响应头
↓
response_body_filter()          ← 逐块处理/修改响应体（可返回延迟）
↓
(响应发送给客户端完成)
↓
logging()                       ← 请求结束日志 & 清理（总是执行）


| 钩子函数                    | 返回类型                                              | 是否必须 | 调用时机（执行阶段）                                         | 主要用途                                                     | 常见注意事项                            |
| --------------------------- | ----------------------------------------------------- | -------- | ------------------------------------------------------------ | ------------------------------------------------------------ | --------------------------------------- |
| **new_ctx**                 | `Self::CTX` (同步)                                    | 是       | 每次新请求开始时，在 `request_filter` 之前调用               | 为本次请求创建上下文（Context），用于在不同阶段之间传递状态  | 必须实现，通常返回 `()` 或自定义 struct |
| **early_request_filter**    | `pingora::Result<bool>` (async)                       | 否       | **最早阶段**：读取到请求头后，但在任何 downstream module 执行之前 | 极早期干预（如模块控制、极早拒绝），一般不建议放业务逻辑     | 比 `request_filter` 更早，慎用          |
| **request_filter**          | `pingora::Result<bool>` (async)                       | 否       | 请求头读取完成后，`upstream_peer` 之前                       | **最常用过滤点**：鉴权、ACL、限流、请求校验、短路返回（返回 `Ok(true)` 表示不再代理） | 返回 `true` 可终止后续代理流程          |
| **upstream_peer**           | `PingoraPeerResult` (`Result<Box<HttpPeer>>`) (async) | **是**   | `request_filter` 之后，需要连接上游时调用                    | **核心路由钩子**：决定请求要转发到哪个后端（IP/端口/TLS/SNI 等） | 必须实现，否则无法代理                  |
| **upstream_request_filter** | `pingora::Result<()>` (async)                         | 否       | 已选好 Peer 并准备发送请求到上游 **之前**                    | 修改即将发往后端的请求头（添加/删除/改写 Header）            | 可用于统一注入 Header、改 Host 等       |
| **response_filter**         | `pingora::Result<()>` (async)                         | 否       | 收到上游响应 **头** 后，但在把响应头发给客户端之前           | 修改响应头（状态码、Header、CORS、缓存控制等）               | 常用修改 `Server`、`Cache-Control` 等   |
| **response_body_filter**    | `pingora::Result<Option<Duration>>` (同步)            | 否       | 接收上游响应 **体** 时，**逐 chunk** 调用（可多次）          | 修改/过滤/压缩响应体、统计大小、注入内容等                   | 返回 `Some(duration)` 可控制发送延迟    |
| **logging**                 | `()` (async)                                          | 否       | **请求完全结束** 时（无论成功、失败、超时、错误），在最终清理前调用 | 统一日志记录、指标上报、清理资源、请求耗时统计等             | 总是被调用，适合放全局统计              |