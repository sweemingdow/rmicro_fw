use pingora_core::prelude::HttpPeer;

pub mod config;
pub mod ext;
pub mod proxy;
pub mod router;
pub mod run;
pub mod state;
pub mod background;

pub type PingoraPeerResult = pingora::Result<Box<HttpPeer>>;

pub type PingoraResult<T> = pingora::Result<T>;

pub use fw_boot::config::Config as boot_cfg;
