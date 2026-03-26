use thiserror;

#[derive(Debug, thiserror::Error, Clone)]
pub enum FwError {
    #[error("sdk error for {0}, {1}")]
    SdkError(&'static str, String),

    #[error("running error for {0}, {1}")]
    RunningError(&'static str, String),

    #[error("result error, {0}")]
    ResultError(String),

    #[error("parse error, {0}")]
    ParseError(String),

    #[error("load error for {0}, {1}")]
    LoadError(&'static str, String),

    #[error("file error for {0}, {1}")]
    FileError(&'static str, String),

    #[error("config error for {0}, {1}")]
    ConfigError(&'static str, String),

    #[error("timeout error for {0}, {1}")]
    TimeoutError(&'static str, String),

    #[error("init error for {0}, {1}")]
    InitError(&'static str, String),
}
