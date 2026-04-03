use crate::PingoraPeerResult;
use crate::config::router_config::TimeoutItem;
use arc_swap::ArcSwap;
use fw_error::{FwError, FwResult};
use pingora_core::prelude::HttpPeer;
use pingora_load_balancing::LoadBalancer;
use pingora_load_balancing::prelude::RoundRobin;
use std::collections::HashSet;
use std::sync::Arc;
use std::time;

/// 维护一个服务下的所有访问的 endpoints
pub struct HttpServerProxy {
    tls: bool,
    srv_name: String,
    state: ArcSwap<SrvProxyState>,
}

struct SrvProxyState {
    lb: LoadBalancer<RoundRobin>,
    addresses: HashSet<String>,
}

impl HttpServerProxy {
    const TOTAL_CONNECTION_TIMEOUT: time::Duration = time::Duration::from_secs(15);
    const IDLE_TIMEOUT: time::Duration = time::Duration::from_secs(45);
    const READ_TIMEOUT: time::Duration = time::Duration::from_secs(15);
    const WRITE_TIMEOUT: time::Duration = time::Duration::from_secs(15);
    const CONNECTION_TIMEOUT: time::Duration = time::Duration::from_millis(800);

    fn create_lb(addresses: &HashSet<String>) -> FwResult<LoadBalancer<RoundRobin>> {
        LoadBalancer::<RoundRobin>::try_from_iter(addresses).map_err(|e| {
            FwError::InitError(
                "create lb",
                format!("with addresses:{:#?}, err={}", addresses, e),
            )
        })
    }

    pub fn new(srv_name: &str, tls: bool, addresses: HashSet<String>) -> FwResult<Self> {
        let lb = Self::create_lb(&addresses)?;
        let state = SrvProxyState { lb, addresses };

        Ok(Self {
            tls,
            srv_name: srv_name.to_string(),
            state: ArcSwap::new(Arc::new(state)),
        })
    }

    pub fn add_instance(&self, addr: &str) {
        self.state.rcu(|curr| {
            if curr.addresses.contains(addr) {
                return curr.clone();
            }

            let mut new_set = curr.addresses.clone();
            new_set.insert(addr.to_string());

            match Self::create_lb(&new_set) {
                Ok(new_lb) => Arc::new(SrvProxyState {
                    lb: new_lb,
                    addresses: new_set,
                }),
                Err(e) => {
                    tracing::error!(
                        service = %self.srv_name,
                        addr = %addr,
                        error = %e,
                        "Failed to add backend instance"
                    );
                    curr.clone() // 创建失败时保持旧状态
                }
            }
        });
    }

    pub fn remove_instance(&self, addr: &str) {
        self.state.rcu(|curr| {
            if !curr.addresses.contains(addr) {
                return curr.clone();
            }

            let mut new_set = curr.addresses.clone();
            new_set.remove(addr);

            if new_set.is_empty() {
                return curr.clone();
            }

            match Self::create_lb(&new_set) {
                Ok(new_lb) => Arc::new(SrvProxyState {
                    lb: new_lb,
                    addresses: new_set,
                }),
                Err(e) => {
                    tracing::error!(
                        service = %self.srv_name,
                        addr = %addr,
                        error = %e,
                        "Failed to remove backend instance"
                    );
                    curr.clone()
                }
            }
        });
    }

    pub fn replace_all(&self, instances: HashSet<String>) -> Result<(), FwError> {
        let new_lb = Self::create_lb(&instances)?;
        self.state.store(Arc::new(SrvProxyState {
            lb: new_lb,
            addresses: instances,
        }));
        Ok(())
    }

    pub fn get_srv_name(&self) -> &str {
        &self.srv_name
    }

    pub async fn select_peer(&self, timeout_cfg: Option<Arc<TimeoutItem>>) -> PingoraPeerResult {
        let state = self.state.load();

        let backend = state.lb.select(b"", 256).ok_or_else(|| {
            tracing::error!("no instances found for {}", self.srv_name);

            pingora::Error::create(
                pingora::ErrorType::HTTPStatus(503),
                pingora::ErrorSource::Upstream,
                Some("Service Unavailable".into()),
                None,
            )
        })?;

        let mut peer = Box::new(HttpPeer::new(
            // backend.addr.clone(),
            backend.addr,
            self.tls,
            "".to_string(),
        ));

        if let Some(timeout_cfg) = timeout_cfg {
            peer.options.total_connection_timeout = Some(timeout_cfg.total_conn_timeout);
            peer.options.idle_timeout = Some(timeout_cfg.idle_timeout);
            peer.options.read_timeout = Some(timeout_cfg.read_timeout);
            peer.options.write_timeout = Some(timeout_cfg.write_timeout);
            peer.options.connection_timeout = Some(timeout_cfg.conn_timeout);
        } else {
            peer.options.total_connection_timeout = Some(Self::TOTAL_CONNECTION_TIMEOUT);
            peer.options.idle_timeout = Some(Self::IDLE_TIMEOUT);
            peer.options.read_timeout = Some(Self::READ_TIMEOUT);
            peer.options.write_timeout = Some(Self::WRITE_TIMEOUT);
            peer.options.connection_timeout = Some(Self::CONNECTION_TIMEOUT);
        }

        Ok(peer)
    }
}
