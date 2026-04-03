use crate::nacos::client::NacosClient;
use async_trait::async_trait;
use fw_error::lib_error::FwError;
use fw_error::result::FwResult;
use nacos_sdk::api::naming;
use std::collections::HashMap;

#[async_trait]
pub trait NacosRegister {
    // 注册到nacos中
    async fn register(&self, reg_ops: RegisterOptions) -> FwResult<()>;

    // 从nacos中注销
    async fn deregister(&self, deg_ops: DeregisterOptions) -> FwResult<()>;
}

#[derive(Debug)]
pub struct RegisterOptions {
    pub cluster_name: Option<String>,
    pub group_name: Option<String>,
    pub srv_name: String,
    pub addr: String,
    pub weight: f64,
    pub meta_data: HashMap<String, String>,
}

#[derive(Debug)]
pub struct DeregisterOptions {
    pub cluster_name: Option<String>,
    pub group_name: Option<String>,
    pub srv_name: String,
    pub addr: String,
}

#[derive(Clone)]
pub struct NacosRegisterImpl {
    naming_cli: naming::NamingService,
}

impl NacosRegisterImpl {
    pub fn new(nacos_cli: &NacosClient) -> Box<dyn NacosRegister + Send + Sync> {
        Box::new(Self {
            naming_cli: nacos_cli.get_naming_cli(),
        })
    }

    // 从服务地址中解析出ip, port
    fn extract_ip_port(addr: &str) -> FwResult<(&str, u16)> {
        addr.split_once(":")
            .ok_or_else(|| FwError::ParseError(format!("parse ip:port failed, addr={}", addr)))
            .and_then(|(ip, port_str)| {
                port_str
                    .parse::<u16>()
                    .map_err(|_| {
                        FwError::ParseError(format!("invalid port, port_str={}", port_str))
                    })
                    .map(|port| (ip, port))
            })
    }
}

#[async_trait]
impl NacosRegister for NacosRegisterImpl {
    async fn register(&self, op: RegisterOptions) -> FwResult<()> {
        let mut ins = naming::ServiceInstance::default();
        let (ip, port) = Self::extract_ip_port(&op.addr)?;

        ins.cluster_name = op.cluster_name;
        ins.ip = ip.to_string();
        ins.port = port as i32;
        ins.weight = op.weight;
        ins.enabled = true;
        ins.healthy = true;
        ins.metadata = op.meta_data.clone();
        ins.ephemeral = true;

        self.naming_cli
            .register_instance(op.srv_name, op.group_name, ins)
            .await
            .map_err(|e| FwError::SdkError("nacos sdk register", e.to_string()))
    }

    async fn deregister(&self, op: DeregisterOptions) -> FwResult<()> {
        let mut ins = naming::ServiceInstance::default();
        let (ip, port) = Self::extract_ip_port(&op.addr)?;

        ins.cluster_name = op.cluster_name.map(|s| s.to_string());
        ins.ip = ip.to_string();
        ins.port = port as i32;
        ins.ephemeral = true;
        self.naming_cli
            .deregister_instance(
                op.srv_name.to_string(),
                op.group_name.map(|s| s.to_string()),
                ins,
            )
            .await
            .map_err(|e| FwError::SdkError("nacos sdk deregister", e.to_string()))
    }
}
