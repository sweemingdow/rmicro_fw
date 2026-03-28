pub mod app_error;
pub mod lib_error;
pub mod recorder;
pub mod result;

pub use crate::result::FwResult;

pub use crate::lib_error::FwError;

pub use crate::app_error::AppError;

pub type AnyResult<T> = anyhow::Result<T>;
