use nacos_sdk::api::error;

pub mod client;

pub mod configuration;

pub mod discovery;

pub mod proxy;

pub mod registry;

pub type NacosError = error::Error;

pub type NacosResult<T> = error::Result<T>;
