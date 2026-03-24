use crate::tonic_srv::chan_factory::RpcChanFactory;
use fw_error::result::FwResult;
use std::sync::Arc;
use tokio::sync;
use tonic::transport;

#[derive(Clone)]
pub struct RpcProviderHolder {
    chan_factory: Arc<RpcChanFactory>,
}

impl RpcProviderHolder {
    pub fn new(chan_factory: Arc<RpcChanFactory>) -> Self {
        Self { chan_factory }
    }

    pub async fn get_or_init_client<C, F>(
        &self,
        cell: &sync::OnceCell<C>,
        srv_name: &str,
        constructor: F,
    ) -> FwResult<C>
    where
        C: Clone + Send + Sync + 'static,
        F: Fn(transport::Channel) -> C,
    {
        cell.get_or_try_init(|| async {
            self.chan_factory
                .acquire_chan(srv_name)
                .await
                .map(constructor)
        })
        .await
        .cloned()
    }
}
