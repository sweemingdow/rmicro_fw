use fw_error::{AppError, FwError, FwResult};
use std::sync::OnceLock;
use crate::pass::gw_pass::AuthInfoPassStrategyEnum;


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


static OL_PASS_STRATEGY: OnceLock<AuthInfoPassStrategyEnum> = OnceLock::new();

pub fn init_pass_strategy(strategy: &str) -> FwResult<()> {
    let pass_strategy = AuthInfoPassStrategyEnum::new(strategy);

    OL_PASS_STRATEGY
        .set(pass_strategy)
        .map_err(|e| FwError::InitError("init pass strategy", "already initialized".to_string()))?;
    Ok(())
}

pub fn get_pass_strategy() -> &'static AuthInfoPassStrategyEnum {
    OL_PASS_STRATEGY
        .get()
        .expect("pass strategy not initialized")
}
