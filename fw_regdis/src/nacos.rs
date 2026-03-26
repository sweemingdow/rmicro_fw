
const STATIC_CONFIG_NAME: &'static str = "static-config.yaml";
const DYNAMIC_CONFIG_NAME: &'static str = "dynamic-config.yaml";

use nacos_sdk::api::error;

pub mod client;

pub mod configure;

pub mod discovery;

pub mod proxy;

pub mod registry;

pub mod configuration;

pub type NacosError = error::Error;

pub type NacosResult<T> = error::Result<T>;
