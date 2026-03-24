use crate::lib_error::FwError;

// lib层
pub type FwResult<T> = Result<T, FwError>;

// bin层
pub type AnyResult<T> = anyhow::Result<T>;
