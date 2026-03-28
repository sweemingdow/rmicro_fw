use fw_error::{AppError, FwError, FwResult};
use std::sync::OnceLock;

static GW_DISPATCH_VAL_LOCK: OnceLock<String> = OnceLock::new();

pub fn set_gw_dispatch_val(val: &str) -> FwResult<()> {
    match GW_DISPATCH_VAL_LOCK.set(val.to_string()) {
        Ok(_) => Ok(()),
        Err(_) => Err(FwError::InitError(
            "gw dispatch val",
            "only once allowed setting".to_string(),
        )),
    }
}

pub fn get_gw_dispatch_val() -> Result<&'static str, AppError> {
    GW_DISPATCH_VAL_LOCK
        .get()
        .map(|s| s.as_str())
        .ok_or_else(|| AppError::InnerError("setting dispatch val firstly".to_string()))
}
