use crate::config::GatewayServerConfig;
use fw_boot::ext::RunConfigExt;

pub trait GatewayRunConfigExt: RunConfigExt {
    fn get_gateway_server_config(&self) -> &GatewayServerConfig;
}
